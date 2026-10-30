//! Cooperative process fence. External RouterOS writers require a separate maintenance window.
use anyhow::{Context, Result, ensure};
use std::{
    fs::{self, File, OpenOptions},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf},
};
pub struct Lease {
    _file: File,
    marker: Option<PathBuf>,
}
impl Lease {
    fn lock(root: &Path, exclusive: bool) -> Result<Self> {
        Self::lock_mode(root, exclusive, true)
    }
    fn lock_mode(root: &Path, exclusive: bool, create: bool) -> Result<Self> {
        if create {
            fs::create_dir_all(root.join("state"))?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(create)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(root.join("state/application-writer.lock"))?;
        let mode = if exclusive {
            libc::LOCK_EX
        } else {
            libc::LOCK_SH
        };
        // SAFETY: the owned File supplies a live descriptor. flock does not retain a Rust pointer.
        ensure!(
            unsafe { libc::flock(file.as_raw_fd(), mode | libc::LOCK_NB) } == 0,
            "Application writer fence is busy; stop every console before maintenance"
        );
        if exclusive {
            reject_live_console(root)?;
        }
        Ok(Self {
            _file: file,
            marker: None,
        })
    }
    pub fn console(root: &Path) -> Result<Self> {
        let lease = Self::lock(root, false)?;
        ensure!(
            !root.join("state/core-migration-active.json").exists(),
            "Migration recovery is pending; reconcile its journal before starting the console"
        );
        Ok(lease)
    }
    pub fn exclusive(root: &Path) -> Result<Self> {
        let lease = Self::lock(root, true)?;
        ensure!(
            !root.join("state/core-migration-active.json").exists(),
            "Migration recovery is pending"
        );
        Ok(lease)
    }
    pub(crate) fn recovery(root: &Path, revision: &str) -> Result<(Self, String, Option<String>)> {
        let mut lease = Self::lock_mode(root, true, false)?;
        // The artifact reader validates the matching marker without following its supplied path.
        let (journal, manifest) = crate::core_recovery::worker_artifacts(root, revision)?;
        lease.marker = Some(root.join("state/core-migration-active.json"));
        Ok((lease, journal, manifest))
    }
    pub fn maintenance(root: &Path, journal: &str, revision: &str) -> Result<Self> {
        let mut lease = Self::lock(root, true)?;
        let marker = root.join("state/core-migration-active.json");
        let expected = serde_json::json!({"journal":journal,"revision":revision});
        if marker.exists() {
            ensure!(
                serde_json::from_str::<serde_json::Value>(&fs::read_to_string(&marker)?)?
                    == expected,
                "Recovery marker belongs to another migration"
            );
        } else {
            crate::store::atomic_write(&marker, &serde_json::to_string(&expected)?)?;
        }
        lease.marker = Some(marker);
        Ok(lease)
    }
    pub fn finish(mut self) -> Result<()> {
        if let Some(marker) = self.marker.take() {
            fs::remove_file(&marker)
                .context("Migration completed but recovery marker removal failed")?;
            File::open(marker.parent().unwrap())?.sync_all()?;
        }
        Ok(())
    }
}

fn reject_live_console(root: &Path) -> Result<()> {
    use std::os::unix::ffi::OsStringExt;
    let root = root.canonicalize()?;
    for item in fs::read_dir("/proc")? {
        let item = item?;
        if item.file_name().to_string_lossy().parse::<u32>().is_err() {
            continue;
        }
        let path = item.path();
        let Ok(name) = fs::read_to_string(path.join("comm")) else {
            continue;
        };
        if name.trim() != "lqosync-v3" || process_ended(&path) {
            continue;
        }
        let environment = match fs::read(path.join("environ")) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) if process_ended(&path) => continue,
            Err(e) => return Err(e).context("Cannot inspect a live legacy console"),
        };
        let data = (|| -> std::io::Result<PathBuf> {
            let data = if let Some(value) = environment
                .split(|b| *b == 0)
                .find_map(|part| part.strip_prefix(b"LQOSYNC3_DATA="))
            {
                let value = PathBuf::from(std::ffi::OsString::from_vec(value.to_vec()));
                if value.is_absolute() {
                    value
                } else {
                    fs::read_link(path.join("cwd"))?.join(value)
                }
            } else {
                fs::read_link(path.join("cwd"))?.join("data")
            };
            data.canonicalize()
        })();
        let data = match data {
            Ok(value) => value,
            Err(_) if process_ended(&path) => continue,
            Err(error) => {
                return Err(error).context("Cannot resolve live legacy console data directory");
            }
        };
        ensure!(
            data != root,
            "A console still uses this data directory; stop it before maintenance, including older builds without leases"
        );
    }
    Ok(())
}

impl Drop for Lease {
    fn drop(&mut self) {
        // SAFETY: the owned descriptor is live. Explicit unlock ends this scope even
        // when a fork temporarily retains another descriptor for the same open file.
        let _ = unsafe { libc::flock(self._file.as_raw_fd(), libc::LOCK_UN) };
    }
}

fn process_ended(path: &Path) -> bool {
    match fs::read_to_string(path.join("stat")) {
        Ok(stat) => stat.rsplit_once(')').is_some_and(|(_, rest)| {
            let fields = rest.split_whitespace().collect::<Vec<_>>();
            // Linux include/linux/sched.h: PF_EXITING = 0x00000004.
            // Exiting tasks may lose /proc/environ access before their state becomes Z.
            fields.first().is_some_and(|s| *s == "Z" || *s == "X")
                || fields
                    .get(6)
                    .and_then(|s| s.parse::<u64>().ok())
                    .is_some_and(|flags| flags & 4 != 0)
        }),
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn exiting_process_is_not_a_live_writer_before_it_becomes_a_zombie() {
        let dir = tempfile::tempdir().unwrap();
        for (state, flags, ended) in [
            ("R", 0, false),
            ("R", 4, true),
            ("Z", 0, true),
            ("X", 0, true),
        ] {
            std::fs::write(
                dir.path().join("stat"),
                format!("123 (lqosync-v3) {state} 0 0 0 0 0 {flags} 0"),
            )
            .unwrap();
            assert_eq!(super::process_ended(dir.path()), ended);
        }
    }
}
