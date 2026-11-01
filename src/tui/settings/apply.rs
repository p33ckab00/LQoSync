//! Apply: write the secrets and the config file, then verify the reload in `status.json`.

use super::secrets::{Pending, write_secrets};
use crate::cli::config_digest;
use crate::tui::dashboard::Sent;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ApplyError {
    /// The file on disk is not the file at the start of the edit. The text on disk.
    Changed {
        disk: String,
    },
    Io(String),
}

/// Keep the newest `keep` config backups (`<12 digits>.toml`). Never remove `new`.
/// The order key of a config backup name: `<12 digits>.toml` or `<12 digits>-<n>.toml`.
fn backup_key(p: &Path) -> Option<(u64, u64)> {
    if p.extension().is_none_or(|x| x != "toml") {
        return None;
    }
    let stem = p.file_stem()?.to_str()?;
    let (secs, n) = match stem.split_once('-') {
        Some((s, n)) => (s, n.parse::<u64>().ok()?),
        None => (stem, 0),
    };
    if secs.len() != 12 || !secs.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((secs.parse().ok()?, n))
}

/// Keep the newest `keep` config backups. Never remove `new`.
fn prune(dir: &Path, keep: usize, new: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut names: Vec<((u64, u64), PathBuf)> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter_map(|p| backup_key(&p).map(|k| (k, p)))
        .collect();
    names.sort();
    let extra = names.len().saturating_sub(keep.max(1));
    for (_, p) in names.into_iter().take(extra).filter(|(_, p)| p != new) {
        let _ = std::fs::remove_file(p);
    }
}

/// Write the new config. Returns the digest of the new text.
/// 1. The file on disk must be the file at the start of the edit. 2. Backup of the old file.
/// 3. The secret files. 4. The config file; on an error, the secret files go back.
pub fn apply(
    path: &Path,
    start_digest: &str,
    new_text: &str,
    secrets: &[Pending],
    state_dir: &Path,
    keep: usize,
    now: u64,
) -> Result<String, ApplyError> {
    let io = |what: &str, e: &dyn std::fmt::Display| ApplyError::Io(format!("{what}: {e:#}"));
    let disk =
        std::fs::read(path).map_err(|e| io(&format!("cannot read {}", path.display()), &e))?;
    if config_digest(&disk) != start_digest {
        return Err(ApplyError::Changed {
            disk: String::from_utf8_lossy(&disk).into_owned(),
        });
    }
    let dir = state_dir.join("config-backups");
    std::fs::create_dir_all(&dir).map_err(|e| io(&format!("cannot make {}", dir.display()), &e))?;
    // Two applies in one second: the second backup gets a number, so none is replaced.
    let backup = (0..)
        .map(|n: u64| match n {
            0 => dir.join(format!("{now:012}.toml")),
            n => dir.join(format!("{now:012}-{n}.toml")),
        })
        .find(|p| !p.exists())
        .expect("a free backup name");
    // fs::copy keeps the mode of the old file.
    std::fs::copy(path, &backup)
        .map_err(|e| io(&format!("cannot write {}", backup.display()), &e))?;
    prune(&dir, keep, &backup);
    let undo =
        write_secrets(secrets).map_err(|e| io("cannot write a secret file", &format!("{e:#}")))?;
    if let Err(e) = crate::files::write_if_changed(path, new_text.as_bytes()) {
        let errors = undo.rollback();
        let mut m = format!("cannot write {}: {e:#}", path.display());
        if !errors.is_empty() {
            m.push_str(&format!(
                "; the secret files are NOT all back: {}",
                errors.join("; ")
            ));
        }
        return Err(ApplyError::Io(m));
    }
    undo.commit();
    Ok(config_digest(new_text.as_bytes()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verify {
    /// No reload status for this apply yet.
    WaitReload,
    /// The service did not load the new file: the old config stays.
    Rejected(String),
    /// The service loaded the new file; a cycle runs.
    Loaded,
    /// The first cycle with the new config ended: its result.
    Done(String),
    /// No proof that the service loaded the new file.
    NotVerified {
        want: String,
        have: String,
        why: String,
    },
}

/// The state of an apply, from `status.json`. `loaded_seq` is the `cycle_seq` when `Loaded` was seen.
/// Only `Loaded` and `Done` mean "applied".
pub fn verify(
    sent: &Sent,
    want_digest: &str,
    loaded_seq: Option<u64>,
    status: Option<&Value>,
    now: u64,
    timeout: u64,
) -> Verify {
    let have = |st: Option<&Value>| {
        st.and_then(|s| s["config_digest"].as_str())
            .unwrap_or("unknown")
            .to_string()
    };
    if let Some(st) = status {
        if st["pid"].as_u64().is_some_and(|p| p != u64::from(sent.pid)) {
            return Verify::NotVerified {
                want: want_digest.into(),
                have: have(status),
                why: "the service restarted; it reads the file on disk at its start".into(),
            };
        }
        let seq = st["cycle_seq"].as_u64();
        if let Some(q) = loaded_seq {
            if seq.is_some_and(|s| s > q) {
                let routers: Vec<String> = st["routers"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|r| {
                        format!(
                            "{} {}",
                            r["name"].as_str().unwrap_or("?"),
                            r["state"].as_str().unwrap_or("unknown")
                        )
                    })
                    .collect();
                let c = &st["report"]["counts"];
                let n = |k: &str| c[k].as_u64().map_or("unknown".into(), |x| x.to_string());
                let mut m = format!(
                    "applied: the first cycle ended: {} | added {} updated {} removed {}",
                    routers.join(", "),
                    n("added"),
                    n("updated"),
                    n("removed")
                );
                if let Some(e) = st["error"].as_str() {
                    m.push_str(&format!(" | error: {e}"));
                }
                return Verify::Done(m);
            }
            return Verify::Loaded;
        }
        let fresh = st["trigger"].as_str() == Some("reload")
            && st["updated"].as_u64().is_some_and(|u| u >= sent.at);
        if fresh {
            if st["config_digest"].as_str() == Some(want_digest) {
                return Verify::Loaded;
            }
            if let Some(e) = st["config_error"].as_str() {
                return Verify::Rejected(e.to_string());
            }
        }
    }
    if now >= sent.at + timeout {
        return Verify::NotVerified {
            want: want_digest.into(),
            have: have(status),
            why: format!("no reload status with the new digest after {timeout} s"),
        };
    }
    Verify::WaitReload
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::config_digest;
    use crate::tui::dashboard::Kind;
    use crate::tui::settings::secrets::Secret;
    use serde_json::json;

    struct Env {
        dir: tempfile::TempDir,
        cfg: PathBuf,
        state: PathBuf,
    }

    fn env() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("lqosync3.toml");
        std::fs::write(&cfg, "# old\n[runtime]\nmode = \"shadow\"\n").unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        Env { dir, cfg, state }
    }

    fn secret(e: &Env) -> (PathBuf, Vec<Pending>) {
        let p = e.dir.path().join("R1.password");
        (
            p.clone(),
            vec![Pending {
                path: p,
                value: Secret::new("pw".into()),
            }],
        )
    }

    fn backups(e: &Env) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(e.state.join("config-backups"))
            .map(|d| {
                d.map(|x| x.unwrap().file_name().into_string().unwrap())
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }

    #[test]
    fn a_changed_file_on_disk_stops_the_apply() {
        let e = env();
        let start = config_digest(b"# another text\n");
        let (pw, secrets) = secret(&e);
        match apply(
            &e.cfg,
            &start,
            "[runtime]\nmode = \"apply\"\n",
            &secrets,
            &e.state,
            5,
            1_000,
        ) {
            Err(ApplyError::Changed { disk }) => {
                assert!(disk.contains("shadow"))
            }
            other => panic!("{other:?}"),
        }
        assert!(backups(&e).is_empty() && !pw.exists());
        assert_eq!(
            std::fs::read_to_string(&e.cfg).unwrap(),
            "# old\n[runtime]\nmode = \"shadow\"\n"
        );
    }

    #[test]
    fn apply_writes_a_backup_and_keeps_n() {
        let e = env();
        for now in [1, 2, 3] {
            let start = config_digest(&std::fs::read(&e.cfg).unwrap());
            let text = format!("[runtime]\nscan_interval_idle = {}\n", 100 + now);
            let d = apply(&e.cfg, &start, &text, &[], &e.state, 2, now).unwrap();
            assert_eq!(d, config_digest(text.as_bytes()));
            assert_eq!(std::fs::read_to_string(&e.cfg).unwrap(), text);
        }
        assert_eq!(backups(&e), ["000000000002.toml", "000000000003.toml"]);
        let b = std::fs::read_to_string(e.state.join("config-backups/000000000003.toml")).unwrap();
        assert!(b.contains("102"), "{b}");
    }

    #[test]
    fn two_applies_in_one_second_keep_both_backups() {
        let e = env();
        for (n, text) in [
            "[runtime]\nscan_interval_idle = 101\n",
            "[runtime]\nscan_interval_idle = 102\n",
            "[runtime]\nscan_interval_idle = 103\n",
        ]
        .iter()
        .enumerate()
        {
            let start = config_digest(&std::fs::read(&e.cfg).unwrap());
            apply(
                &e.cfg,
                &start,
                text,
                &[],
                &e.state,
                2,
                if n == 0 { 5 } else { 7 },
            )
            .unwrap();
        }
        // The two newest stay: the second and the third apply (both at second 7).
        assert_eq!(backups(&e), ["000000000007-1.toml", "000000000007.toml"]);
        let b =
            std::fs::read_to_string(e.state.join("config-backups/000000000007-1.toml")).unwrap();
        assert!(b.contains("102"), "{b}");
    }

    #[test]
    fn a_toml_write_error_rolls_back_the_secrets() {
        let e = env();
        std::fs::create_dir(
            e.dir
                .path()
                .join(format!(".lqosync3.toml.tmp-{}", std::process::id())),
        )
        .unwrap();
        let start = config_digest(&std::fs::read(&e.cfg).unwrap());
        let (pw, secrets) = secret(&e);
        match apply(
            &e.cfg,
            &start,
            "[runtime]\nmode = \"apply\"\n",
            &secrets,
            &e.state,
            5,
            1_000,
        ) {
            Err(ApplyError::Io(m)) => assert!(m.contains("lqosync3.toml"), "{m}"),
            other => panic!("{other:?}"),
        }
        assert!(!pw.exists(), "the new secret file is removed");
        assert_eq!(
            std::fs::read_to_string(&e.cfg).unwrap(),
            "# old\n[runtime]\nmode = \"shadow\"\n"
        );
    }

    fn sent() -> Sent {
        Sent {
            kind: Kind::Reload,
            at: 1_000,
            pid: 7,
            seq: 3,
        }
    }

    #[test]
    fn verify_needs_the_new_digest() {
        let old = json!({"pid": 7, "trigger": "reload", "updated": 1_001, "config_digest": "old", "cycle_seq": 3});
        assert_eq!(
            verify(&sent(), "new", None, Some(&old), 1_001, 120),
            Verify::WaitReload
        );
        match verify(&sent(), "new", None, Some(&old), 1_120, 120) {
            Verify::NotVerified { want, have, .. } => {
                assert_eq!((want.as_str(), have.as_str()), ("new", "old"))
            }
            other => panic!("{other:?}"),
        }
        let restarted = json!({"pid": 8, "trigger": "start", "updated": 1_002, "config_digest": "new", "cycle_seq": 1});
        assert!(
            matches!(verify(&sent(), "new", None, Some(&restarted), 1_002, 120), Verify::NotVerified { why, .. } if why.contains("restarted"))
        );
    }

    #[test]
    fn verify_rejected_shows_config_error() {
        let bad = json!({"pid": 7, "trigger": "reload", "updated": 1_001, "config_digest": "old", "config_error": "line 3: bad", "cycle_seq": 3});
        assert_eq!(
            verify(&sent(), "new", None, Some(&bad), 1_001, 120),
            Verify::Rejected("line 3: bad".into())
        );
    }

    #[test]
    fn verify_loaded_then_done_after_a_higher_cycle_seq() {
        let loaded = json!({"pid": 7, "trigger": "reload", "updated": 1_001, "config_digest": "new", "cycle_seq": 3});
        assert_eq!(
            verify(&sent(), "new", None, Some(&loaded), 1_001, 120),
            Verify::Loaded
        );
        assert_eq!(
            verify(&sent(), "new", Some(3), Some(&loaded), 1_500, 120),
            Verify::Loaded
        );
        let done = json!({"pid": 7, "trigger": "reload", "updated": 1_003, "config_digest": "new", "cycle_seq": 4,
            "routers": [{"name": "R1", "state": "online"}], "report": {"counts": {"added": 1, "updated": 0, "removed": 0}}});
        match verify(&sent(), "new", Some(3), Some(&done), 1_003, 120) {
            Verify::Done(m) => assert!(m.contains("R1 online") && m.contains("added 1"), "{m}"),
            other => panic!("{other:?}"),
        }
    }
}
