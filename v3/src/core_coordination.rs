//! Read-only coordination. Shadow tickets deliberately have no router mutation capability.
use crate::migration_fence::Lease;
use anyhow::{Result, ensure};
use std::{
    fs::{File, OpenOptions},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::Path,
    sync::{Arc, Mutex},
};
#[derive(Default)]
struct Policy {
    revision: String,
    enabled: bool,
    epoch: u64,
}
#[derive(Clone, Default)]
pub struct Coordinator(Arc<Mutex<Policy>>);
impl Coordinator {
    pub fn configure(&self, revision: &str, enabled: bool) {
        let mut policy = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if policy.revision != revision || policy.enabled != enabled {
            policy.epoch = policy.epoch.wrapping_add(1);
            policy.revision = revision.into();
            policy.enabled = enabled;
        }
    }
    /// Call from the blocking pool. The extra shared lease preserves maintenance exclusion.
    pub fn begin(&self, root: &Path, revision: &str) -> Result<Ticket> {
        let epoch = {
            let p = self.0.lock().unwrap_or_else(|p| p.into_inner());
            ensure!(
                p.enabled && p.revision == revision,
                "Shadow worker is paused or settings changed"
            );
            p.epoch
        };
        let root = root.canonicalize()?;
        let console = Lease::console(&root)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(root.join("state/core-shadow.lock"))?;
        // SAFETY: file owns a live descriptor; flock retains no Rust pointer.
        ensure!(
            unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
            "Another shadow worker owns this data directory"
        );
        let ticket = Ticket {
            coordinator: self.clone(),
            epoch,
            _console: console,
            file,
        };
        ticket.check(revision)?;
        Ok(ticket)
    }
}
pub struct Ticket {
    coordinator: Coordinator,
    epoch: u64,
    _console: Lease,
    file: File,
}
impl Ticket {
    pub fn check(&self, revision: &str) -> Result<()> {
        let p = self.coordinator.0.lock().unwrap_or_else(|p| p.into_inner());
        ensure!(
            p.enabled && p.epoch == self.epoch && p.revision == revision,
            "Shadow work cancelled by pause or changed settings"
        );
        Ok(())
    }
    pub fn require_mutation(&self) -> Result<()> {
        anyhow::bail!("Shadow tickets cannot authorize router writes")
    }
}
impl Drop for Ticket {
    fn drop(&mut self) {
        // SAFETY: the owned descriptor is live. Explicit unlock also handles inherited fork descriptors.
        let _ = unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
    }
}
