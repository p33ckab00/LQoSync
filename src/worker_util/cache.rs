//! Keep the public cache path compatible while its data lives on tmpfs.
use super::{CACHE, Telemetry};
use anyhow::{Context, ensure};
use std::os::unix::{
    ffi::OsStrExt,
    fs::{DirBuilderExt, MetadataExt, PermissionsExt},
};
use std::path::{Path, PathBuf};

pub(super) struct Publisher {
    state: PathBuf,
    ram: Option<PathBuf>,
}
impl Publisher {
    pub(super) fn new(state: &Path) -> Self {
        // SAFETY: geteuid has no arguments or memory preconditions.
        let uid = unsafe { libc::geteuid() };
        Self::with_root(state, &PathBuf::from(format!("/dev/shm/lqosync3-{uid}")))
    }
    fn with_root(state: &Path, root: &Path) -> Self {
        let ram = runtime_dir(state, root);
        match &ram {
            Ok(path) => eprintln!("worker telemetry cache: tmpfs {}", path.display()),
            Err(error) => eprintln!("worker telemetry cache: disk fallback: {error:#}"),
        }
        Self {
            state: state.into(),
            ram: ram.ok(),
        }
    }
    pub(super) fn publish(&mut self, report: &Telemetry) -> anyhow::Result<()> {
        if let Some(ram) = &self.ram {
            let result = super::publish(ram, report)
                .and_then(|()| public_link(&self.state, &ram.join(CACHE)));
            match result {
                Ok(()) => return Ok(()),
                Err(error) => eprintln!("worker telemetry cache: disk fallback: {error:#}"),
            }
            self.ram = None;
        }
        super::publish(&self.state, report)
    }
}

fn owned_dir(path: &Path) -> anyhow::Result<()> {
    let created = match std::fs::DirBuilder::new().mode(0o755).create(path) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
        Err(error) => return Err(error.into()),
    };
    let meta = std::fs::symlink_metadata(path)?;
    // SAFETY: geteuid has no arguments or memory preconditions.
    let uid = unsafe { libc::geteuid() };
    ensure!(
        meta.is_dir() && meta.uid() == uid && meta.mode() & 0o022 == 0,
        "runtime directory must be owned by the service UID without group/other write access"
    );
    if created {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    } else {
        ensure!(
            meta.mode() & 0o005 == 0o005,
            "runtime directory must permit public cache reads"
        );
    }
    Ok(())
}

fn runtime_dir(state: &Path, root: &Path) -> anyhow::Result<PathBuf> {
    owned_dir(root)?;
    let name = std::ffi::CString::new(root.as_os_str().as_bytes())?;
    let mut info = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: name is NUL-terminated, and info points to writable statfs storage.
    let result = unsafe { libc::statfs(name.as_ptr(), info.as_mut_ptr()) };
    if result != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: a successful statfs initialized info.
    ensure!(
        unsafe { info.assume_init() }.f_type == libc::TMPFS_MAGIC,
        "runtime cache requires tmpfs"
    );
    let canonical = std::fs::canonicalize(state).context("resolve telemetry state directory")?;
    let path = root.join(crate::write_receipt::sha256(
        canonical.as_os_str().as_bytes(),
    ));
    owned_dir(&path)?;
    Ok(path)
}

fn public_link(state: &Path, target: &Path) -> anyhow::Result<()> {
    let path = state.join(CACHE);
    if std::fs::read_link(&path).is_ok_and(|old| old == target) {
        return Ok(());
    }
    let tmp = state.join(format!(".{CACHE}.link-{}", std::process::id()));
    std::os::unix::fs::symlink(target, &tmp)?;
    if let Err(error) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(tmp);
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker_util::{CACHE, read_cache};
    use serde_json::json;

    fn ram() -> tempfile::TempDir {
        let dir = tempfile::tempdir_in("/dev/shm").unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        dir
    }

    fn report() -> Telemetry {
        serde_json::from_value(json!({
            "version":1,"updated":10,"stale":false,"reason":null,
            "workers":[{"queue_id":0,"busy_ppm":125000}],
            "traffic":[{"at":10,"down_mbps":13.0,"up_mbps":2.0}]
        }))
        .unwrap()
    }

    #[test]
    fn ram_cache_keeps_legacy_reader_and_public_link_between_updates() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let state = tempfile::tempdir().unwrap();
        let ram = ram();
        let path = state.path().join(CACHE);
        std::fs::write(&path, "old cache").unwrap();
        std::fs::write(state.path().join("write-receipt.json"), "durable").unwrap();
        let mut publisher = Publisher::with_root(state.path(), ram.path());
        let mut sample = report();
        publisher.publish(&sample).unwrap();
        let target = std::fs::read_link(&path).expect("public path must point to RAM");
        assert!(target.starts_with(ram.path()));
        assert_eq!(
            std::fs::metadata(target.parent().unwrap()).unwrap().mode() & 0o777,
            0o755
        );
        assert_eq!(std::fs::metadata(ram.path()).unwrap().mode() & 0o777, 0o755);
        assert!(read_cache(&path).live(10));
        assert_eq!(read_cache(&path).traffic[0].down_mbps, Some(13.0));
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o644
        );
        let link_inode = std::fs::symlink_metadata(&path).unwrap().ino();
        sample.updated = 12;
        publisher.publish(&sample).unwrap();
        assert_eq!(std::fs::symlink_metadata(&path).unwrap().ino(), link_inode);
        assert!(read_cache(&path).live(12));
        assert!(!read_cache(&path).live(19));
        assert_eq!(
            std::fs::read_to_string(state.path().join("write-receipt.json")).unwrap(),
            "durable"
        );
    }

    #[test]
    fn rollback_regular_writer_does_not_modify_ram_target() {
        let state = tempfile::tempdir().unwrap();
        let ram = ram();
        let mut publisher = Publisher::with_root(state.path(), ram.path());
        publisher.publish(&report()).unwrap();
        let path = state.path().join(CACHE);
        let target = std::fs::read_link(&path).unwrap();
        let old = std::fs::read(&target).unwrap();
        let mut sample = report();
        sample.updated = 12;
        super::super::publish(state.path(), &sample).unwrap();
        assert!(!std::fs::symlink_metadata(&path).unwrap().is_symlink());
        assert!(read_cache(&path).live(12));
        assert_eq!(std::fs::read(&target).unwrap(), old);
        publisher.publish(&sample).unwrap();
        assert!(std::fs::read_link(&path).is_ok());
    }

    #[test]
    fn missing_runtime_data_is_unavailable_until_new_publisher_recovers() {
        let state = tempfile::tempdir().unwrap();
        let ram = ram();
        let mut publisher = Publisher::with_root(state.path(), ram.path());
        publisher.publish(&report()).unwrap();
        let path = state.path().join(CACHE);
        let target = std::fs::read_link(&path).unwrap();
        std::fs::remove_dir_all(target.parent().unwrap()).unwrap();
        assert!(read_cache(&path).stale);
        assert!(!read_cache(&path).live(10));
        let mut restarted = Publisher::with_root(state.path(), ram.path());
        restarted.publish(&report()).unwrap();
        assert!(read_cache(&path).live(10));
    }

    #[test]
    fn runtime_write_failure_replaces_link_with_disk_cache_until_restart() {
        let state = tempfile::tempdir().unwrap();
        let ram = ram();
        let mut publisher = Publisher::with_root(state.path(), ram.path());
        publisher.publish(&report()).unwrap();
        let path = state.path().join(CACHE);
        let target = std::fs::read_link(&path).unwrap();
        std::fs::remove_dir_all(target.parent().unwrap()).unwrap();
        let mut sample = report();
        sample.updated = 12;
        publisher.publish(&sample).unwrap();
        assert!(publisher.ram.is_none());
        assert!(!std::fs::symlink_metadata(&path).unwrap().is_symlink());
        assert!(read_cache(&path).live(12));
        std::fs::create_dir(target.parent().unwrap()).unwrap();
        publisher.publish(&sample).unwrap();
        assert!(std::fs::read_link(&path).is_err());
    }

    #[test]
    fn unsafe_runtime_directories_use_disk_without_following_links() {
        let state = tempfile::tempdir().unwrap();
        let ram = ram();
        let outside = ram.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        let link = ram.path().join("link");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let mut publisher = Publisher::with_root(state.path(), &link);
        publisher.publish(&report()).unwrap();
        assert!(publisher.ram.is_none());
        assert!(std::fs::read_dir(outside).unwrap().next().is_none());
        std::fs::set_permissions(ram.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
        let mut publisher = Publisher::with_root(state.path(), ram.path());
        publisher.publish(&report()).unwrap();
        assert!(publisher.ram.is_none());
        assert_eq!(
            std::fs::symlink_metadata(ram.path()).unwrap().mode() & 0o777,
            0o777
        );
        assert!(read_cache(&state.path().join(CACHE)).live(10));
    }

    #[test]
    fn state_directories_have_isolated_runtime_files_and_complete_histories() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let ram = ram();
        let mut one = Publisher::with_root(first.path(), ram.path());
        let mut two = Publisher::with_root(second.path(), ram.path());
        let mut sample = report();
        let history = (1..=1801)
            .map(|at| super::super::traffic::Point {
                at,
                down_mbps: Some(13.0),
                up_mbps: Some(2.0),
            })
            .collect::<std::collections::VecDeque<_>>();
        sample.updated = 1801;
        sample.traffic = history.clone();
        sample.classification.mapped.history = history.clone();
        sample.classification.unmapped.history = history;
        one.publish(&sample).unwrap();
        two.publish(&report()).unwrap();
        assert_ne!(
            std::fs::read_link(first.path().join(CACHE)).unwrap(),
            std::fs::read_link(second.path().join(CACHE)).unwrap()
        );
        let loaded = read_cache(&first.path().join(CACHE));
        assert!(loaded.live(1801));
        assert_eq!(loaded.traffic.len(), 1801);
        assert_eq!(loaded.classification.mapped.history.len(), 1801);
        assert_eq!(loaded.classification.unmapped.history.len(), 1801);
        assert_eq!(read_cache(&second.path().join(CACHE)).traffic.len(), 1);
    }

    #[test]
    fn failed_runtime_initialization_preserves_unknown_public_target() {
        let state = tempfile::tempdir().unwrap();
        let ram = ram();
        let blocked = ram.path().join("not-a-directory");
        std::fs::write(&blocked, "blocked").unwrap();
        let original = state.path().join("original");
        std::fs::write(&original, "preserve").unwrap();
        std::os::unix::fs::symlink(&original, state.path().join(CACHE)).unwrap();
        let mut publisher = Publisher::with_root(state.path(), &blocked);
        assert_eq!(std::fs::read_to_string(&original).unwrap(), "preserve");
        publisher.publish(&report()).unwrap();
        assert!(publisher.ram.is_none());
        assert_eq!(std::fs::read_to_string(&original).unwrap(), "preserve");
        assert!(read_cache(&state.path().join(CACHE)).live(10));
    }

    #[test]
    #[ignore = "manual isolated publication I/O measurement"]
    fn ram_publication_io_measurement() {
        fn written() -> u64 {
            std::fs::read_to_string("/proc/self/io")
                .unwrap()
                .lines()
                .find_map(|line| line.strip_prefix("write_bytes: ")?.trim().parse().ok())
                .unwrap()
        }
        let state = tempfile::tempdir().unwrap();
        let ram = ram();
        let mut publisher = Publisher::with_root(state.path(), ram.path());
        assert!(publisher.ram.is_some());
        let mut sample = report();
        let history = (1..=1801)
            .map(|at| super::super::traffic::Point {
                at,
                down_mbps: Some(13.123456),
                up_mbps: Some(2.123456),
            })
            .collect::<std::collections::VecDeque<_>>();
        sample.updated = 1801;
        sample.traffic = history.clone();
        sample.classification.mapped.history = history.clone();
        sample.classification.unmapped.history = history;
        publisher.publish(&sample).unwrap();
        let cache_bytes = std::fs::metadata(state.path().join(CACHE)).unwrap().len();
        let before = written();
        let start = std::time::Instant::now();
        for _ in 0..50 {
            publisher.publish(&sample).unwrap();
        }
        let ram_time = start.elapsed();
        let ram_written = written().saturating_sub(before);
        let before = written();
        let start = std::time::Instant::now();
        for _ in 0..50 {
            super::super::publish(state.path(), &sample).unwrap();
        }
        let disk_time = start.elapsed();
        let disk_written = written().saturating_sub(before);
        println!(
            "profile={} samples=50 bytes_per_cache={cache_bytes} ram_write_bytes={ram_written} disk_write_bytes={disk_written} ram_elapsed={ram_time:?} disk_elapsed={disk_time:?}",
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
        );
        assert_eq!(
            ram_written, 0,
            "steady RAM publication must not dirty block-backed pages"
        );
    }

    #[test]
    fn preexisting_private_runtime_directory_keeps_its_permissions() {
        let state = tempfile::tempdir().unwrap();
        let ram = ram();
        std::fs::set_permissions(ram.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut publisher = Publisher::with_root(state.path(), ram.path());
        publisher.publish(&report()).unwrap();
        assert!(publisher.ram.is_none());
        assert_eq!(
            std::fs::symlink_metadata(ram.path()).unwrap().mode() & 0o777,
            0o700
        );
        assert!(std::fs::read_link(state.path().join(CACHE)).is_err());
    }

    #[test]
    fn failed_runtime_rename_cleans_temporary_cache_before_disk_fallback() {
        let state = tempfile::tempdir().unwrap();
        let ram = ram();
        let mut publisher = Publisher::with_root(state.path(), ram.path());
        let destination = publisher.ram.clone().unwrap();
        std::fs::create_dir(destination.join(CACHE)).unwrap();
        publisher.publish(&report()).unwrap();
        assert!(publisher.ram.is_none());
        assert!(
            !destination
                .join(format!(".{CACHE}.tmp-{}", std::process::id()))
                .exists()
        );
        assert!(read_cache(&state.path().join(CACHE)).live(10));
    }
}
