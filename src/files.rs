//! Files: the instance lock, live inputs, the state, atomic writes, backups, and the reload.

use crate::config::CircuitNames;
use crate::model::{CsvRow, State};
use crate::state::{read_csv, seed_nodes, seed_state, split_static};
use anyhow::{Context, bail};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

/// Take the instance lock on `<state_dir>/lqosync3.lock`. Keep the file open while the lock is needed.
pub fn lock(state_dir: &Path) -> anyhow::Result<File> {
    std::fs::create_dir_all(state_dir)
        .with_context(|| format!("cannot make {}", state_dir.display()))?;
    let path = state_dir.join("lqosync3.lock");
    let f = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .with_context(|| format!("cannot open {}", path.display()))?;
    match f.try_lock() {
        Ok(()) => Ok(f),
        Err(std::fs::TryLockError::WouldBlock) => {
            bail!("another lqosync3 is running on {}", state_dir.display())
        }
        Err(std::fs::TryLockError::Error(e)) => {
            Err(e).with_context(|| format!("cannot lock {}", path.display()))
        }
    }
}

/// Write `bytes` to `path` only if they differ from the file. Atomic: temporary file, sync, rename,
/// sync of the folder. Returns `true` if the file was written.
pub fn write_if_changed(path: &Path, bytes: &[u8]) -> anyhow::Result<bool> {
    if std::fs::read(path).is_ok_and(|old| old == bytes) {
        return Ok(false);
    }
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).with_context(|| format!("cannot make {}", dir.display()))?;
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = dir.join(format!(".{name}.tmp-{}", std::process::id()));
    let mut f = File::create(&tmp).with_context(|| format!("cannot write {}", tmp.display()))?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path)
        .with_context(|| format!("cannot rename {} to {}", tmp.display(), path.display()))?;
    File::open(dir)?.sync_all()?;
    Ok(true)
}

/// The live LibreQoS files of this cycle.
#[derive(Debug, Clone, Default)]
pub struct Live {
    pub static_rows: Vec<CsvRow>,
    pub other_rows: Vec<CsvRow>,
    pub network: Option<serde_json::Value>,
}

/// Read `<lqos_dir>/ShapedDevices.csv` and, if `need_network`, `<lqos_dir>/network.json`.
/// A missing file is empty. A `network.json` that is not valid JSON stops the cycle when it is needed.
pub fn read_live(
    lqos_dir: &Path,
    static_comment: &str,
    need_network: bool,
) -> anyhow::Result<Live> {
    let csv_path = lqos_dir.join("ShapedDevices.csv");
    let rows = match std::fs::read_to_string(&csv_path) {
        Ok(t) => read_csv(&t).with_context(|| format!("{}", csv_path.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e).with_context(|| format!("cannot read {}", csv_path.display())),
    };
    let (static_rows, other_rows) = split_static(rows, static_comment);
    let net_path = lqos_dir.join("network.json");
    let network = if need_network {
        match std::fs::read_to_string(&net_path) {
            Ok(t) => Some(serde_json::from_str(&t).with_context(|| {
                format!(
                    "{} is not valid JSON; fix it before the next cycle",
                    net_path.display()
                )
            })?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e).with_context(|| format!("cannot read {}", net_path.display())),
        }
    } else {
        None
    };
    Ok(Live {
        static_rows,
        other_rows,
        network,
    })
}

/// Read `<state_dir>/state.json`, or make the first state from the live files.
pub fn load_state(
    state_dir: &Path,
    live: &Live,
    names: &CircuitNames,
    now: u64,
) -> anyhow::Result<State> {
    let path = state_dir.join("state.json");
    match std::fs::read_to_string(&path) {
        Ok(t) => State::from_json(&t).with_context(|| format!("{}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut st = seed_state(&live.other_rows, names, now);
            if let Some(net) = &live.network {
                st.nodes = seed_nodes(net);
            }
            Ok(st)
        }
        Err(e) => Err(e).with_context(|| format!("cannot read {}", path.display())),
    }
}

/// Copy the existing `sources` into `<applied_dir>/<stamp>/`, then keep only the newest `keep` folders.
pub fn backup(
    applied_dir: &Path,
    sources: &[&Path],
    stamp: &str,
    keep: usize,
) -> anyhow::Result<()> {
    let dir = applied_dir.join(stamp);
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot make {}", dir.display()))?;
    for src in sources.iter().filter(|s| s.exists()) {
        let name = src
            .file_name()
            .context("a backup source has no file name")?;
        std::fs::copy(src, dir.join(name))
            .with_context(|| format!("cannot copy {}", src.display()))?;
    }
    let mut folders: Vec<_> = std::fs::read_dir(applied_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();
    folders.sort();
    let extra = folders.len().saturating_sub(keep);
    for old in &folders[..extra] {
        std::fs::remove_dir_all(old).with_context(|| format!("cannot remove {}", old.display()))?;
    }
    Ok(())
}

/// Run the LibreQoS reload command with a time limit. The error has the exit code and the end of stderr.
pub async fn run_reload(cmd: &str, args: &[String], timeout: Duration) -> Result<(), String> {
    let child = tokio::process::Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("cannot start {cmd}: {e}"))?;
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Err(_) => Err(format!(
            "{cmd} did not finish in the time limit of {} s; it was stopped",
            timeout.as_secs()
        )),
        Ok(Err(e)) => Err(format!("{cmd}: {e}")),
        Ok(Ok(out)) if out.status.success() => Ok(()),
        Ok(Ok(out)) => {
            let text = String::from_utf8_lossy(&out.stderr);
            let lines: Vec<&str> = text.lines().collect();
            let tail = lines[lines.len().saturating_sub(20)..].join("\n");
            Err(format!(
                "{cmd} exit {}: {tail}",
                out.status.code().map_or("signal".into(), |c| c.to_string())
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CircuitNames;
    use std::time::Duration;

    const HDR: &str = "Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,MAC,IPv4,IPv6,Download Min Mbps,Upload Min Mbps,Download Max Mbps,Upload Max Mbps,Comment\n";

    #[test]
    fn second_lock_fails() {
        let d = tempfile::tempdir().unwrap();
        let _first = lock(d.path()).unwrap();
        let e = lock(d.path()).unwrap_err();
        assert!(format!("{e:#}").contains("another lqosync3"), "{e:#}");
    }

    #[test]
    fn write_if_changed_skips_same_bytes_and_leaves_no_temp_file() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.txt");
        assert!(write_if_changed(&p, b"one").unwrap());
        assert!(!write_if_changed(&p, b"one").unwrap());
        assert!(write_if_changed(&p, b"two").unwrap());
        assert_eq!(std::fs::read(&p).unwrap(), b"two");
        let names: Vec<_> = std::fs::read_dir(d.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 1, "{names:?}");
    }

    #[test]
    fn read_live_splits_static_rows() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("ShapedDevices.csv"),
            format!("{HDR}C1,a,D1,a,,,,,5,5,10,10,PPP\nC2,s,D2,s,,,,,5,5,10,10,Static\n"),
        )
        .unwrap();
        let live = read_live(d.path(), "static", false).unwrap();
        assert_eq!((live.other_rows.len(), live.static_rows.len()), (1, 1));
        assert!(live.network.is_none());
        let empty = tempfile::tempdir().unwrap();
        assert!(
            read_live(empty.path(), "static", true)
                .unwrap()
                .other_rows
                .is_empty()
        );
    }

    #[test]
    fn bad_network_json_stops_when_needed() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("network.json"), "{ not json").unwrap();
        assert!(read_live(d.path(), "static", true).is_err());
        assert!(read_live(d.path(), "static", false).is_ok());
    }

    #[test]
    fn load_state_seeds_on_first_run_and_reads_after() {
        let lq = tempfile::tempdir().unwrap();
        let st = tempfile::tempdir().unwrap();
        std::fs::write(
            lq.path().join("ShapedDevices.csv"),
            format!("{HDR}C1,alice,D1,alice,PPP-R1,,,,5,5,10,10,PPP\n"),
        )
        .unwrap();
        std::fs::write(lq.path().join("network.json"), r#"{"PPP-R1":{"children":{},"downloadBandwidthMbps":10.0,"type":"plan","uploadBandwidthMbps":10.0}}"#).unwrap();
        let live = read_live(lq.path(), "static", true).unwrap();
        let s = load_state(st.path(), &live, &CircuitNames::default(), 5).unwrap();
        assert_eq!(s.circuits["alice"].row.circuit_id, "C1");
        assert!(s.nodes["PPP-R1"].seeded);
        let mut saved = s.clone();
        saved.circuits.clear();
        write_if_changed(&st.path().join("state.json"), saved.to_json().as_bytes()).unwrap();
        assert!(
            load_state(st.path(), &live, &CircuitNames::default(), 5)
                .unwrap()
                .circuits
                .is_empty()
        );
    }

    #[test]
    fn backup_keeps_the_last_n() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("ShapedDevices.csv");
        std::fs::write(&src, "x").unwrap();
        let applied = d.path().join("applied");
        for i in 0..5 {
            backup(
                &applied,
                &[src.as_path()],
                &format!("2026-10-31T00:00:0{i}Z"),
                3,
            )
            .unwrap();
        }
        let mut names: Vec<String> = std::fs::read_dir(&applied)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "2026-10-31T00:00:02Z",
                "2026-10-31T00:00:03Z",
                "2026-10-31T00:00:04Z"
            ]
        );
        assert!(
            applied
                .join("2026-10-31T00:00:04Z/ShapedDevices.csv")
                .exists()
        );
    }

    #[tokio::test]
    async fn reload_ok_and_failure_and_timeout() {
        assert_eq!(
            run_reload("/bin/true", &[], Duration::from_secs(5)).await,
            Ok(())
        );
        assert!(
            run_reload("/bin/false", &[], Duration::from_secs(5))
                .await
                .unwrap_err()
                .contains("exit")
        );
        let e = run_reload("/bin/sleep", &["5".into()], Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(e.contains("time"), "{e}");
    }
}
