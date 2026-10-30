//! Shared process slot with separate shadow and exact-batch coordinated writer capabilities.
use crate::migration_fence::Lease;
use anyhow::{Result, ensure};
use std::{
    fs::{File, OpenOptions},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Default)]
struct Policy {
    root: Option<PathBuf>,
    revision: String,
    enabled: bool,
    epoch: u64,
}
#[derive(Clone, Default)]
pub struct Coordinator(Arc<Mutex<Policy>>);
impl Coordinator {
    pub fn for_root(root: &Path) -> Result<Self> {
        Ok(Self(Arc::new(Mutex::new(Policy {
            root: Some(root.canonicalize()?),
            ..Default::default()
        }))))
    }
    /// Durable pause. Stale local configuration cannot clear it.
    pub fn pause_all(&self) -> Result<()> {
        let (root, revision) = {
            let mut p = self.0.lock().unwrap_or_else(|p| p.into_inner());
            p.enabled = false;
            p.epoch = p.epoch.wrapping_add(1);
            (
                p.root
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("Coordinator is not bound"))?,
                p.revision.clone(),
            )
        };
        crate::core_shared_control::pause(&root, &revision)
    }
    pub fn configure(&self, revision: &str, enabled: bool) {
        let mut policy = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if policy.revision != revision || policy.enabled != enabled {
            policy.epoch = policy.epoch.wrapping_add(1);
            policy.revision = revision.into();
            policy.enabled = enabled;
        }
    }
    fn matches_policy(&self, epoch: u64, revision: &str) -> bool {
        let p = self.0.lock().unwrap_or_else(|p| p.into_inner());
        p.enabled && p.epoch == epoch && p.revision == revision
    }
    /// Call from the blocking pool. The extra shared lease preserves maintenance exclusion.
    pub fn begin(&self, root: &Path, revision: &str) -> Result<Ticket> {
        let root = root.canonicalize()?;
        let epoch = {
            let mut p = self.0.lock().unwrap_or_else(|p| p.into_inner());
            ensure!(
                p.enabled && p.revision == revision,
                "Shadow worker is paused or settings changed"
            );
            ensure!(
                p.root.as_ref().is_none_or(|bound| bound == &root),
                "Coordinator belongs to another data directory"
            );
            p.root.get_or_insert_with(|| root.clone());
            p.epoch
        };
        let shared = crate::core_shared_control::capture(&root, revision)?;
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
            root,
            shared,
            valid: AtomicBool::new(true),
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
    root: PathBuf,
    shared: Option<crate::core_shared_control::State>,
    valid: AtomicBool,
    epoch: u64,
    _console: Lease,
    file: File,
}
impl Ticket {
    pub fn check(&self, revision: &str) -> Result<()> {
        let valid = self.valid.load(Ordering::SeqCst)
            && self.coordinator.matches_policy(self.epoch, revision)
            && crate::core_shared_control::matches(&self.root, &self.shared, revision)
            && self.coordinator.matches_policy(self.epoch, revision);
        if !valid {
            self.valid.store(false, Ordering::SeqCst);
        }
        ensure!(
            valid,
            "Shadow work cancelled by pause, control loss, or changed settings"
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

/// A local policy grant. Pausing and resuming never revives a captured epoch.
#[derive(Clone)]
pub(crate) struct WriterGuard {
    coordinator: Coordinator,
    epoch: u64,
    revision: String,
    alive: Arc<AtomicBool>,
    root: PathBuf,
    shared: Option<crate::core_shared_control::State>,
}
impl WriterGuard {
    pub(crate) fn recovery(root: &Path, revision: &str) -> Result<Self> {
        let coordinator = Coordinator::for_root(root)?;
        coordinator.configure(revision, true);
        let shared = crate::core_shared_control::capture(root, revision)?;
        crate::core_shared_control::require_initialized(&shared)?;
        let epoch = coordinator
            .0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .epoch;
        Ok(Self {
            coordinator,
            epoch,
            revision: revision.into(),
            alive: Arc::new(AtomicBool::new(true)),
            root: root.canonicalize()?,
            shared,
        })
    }
    pub(crate) fn revoke(&self) {
        self.alive.store(false, Ordering::SeqCst);
    }
    pub(crate) fn active(&self) -> bool {
        let active = self.alive.load(Ordering::SeqCst)
            && self.coordinator.matches_policy(self.epoch, &self.revision)
            && crate::core_shared_control::matches(&self.root, &self.shared, &self.revision)
            && self.coordinator.matches_policy(self.epoch, &self.revision);
        if !active {
            self.alive.store(false, Ordering::SeqCst);
        }
        active
    }
}
/// Owns the same process slot as previews while retaining maintenance exclusion.
pub(crate) struct CoordinatedLease {
    ticket: Ticket,
    batch: String,
    guard: WriterGuard,
    marker: Option<(PathBuf, String)>,
}
impl Coordinator {
    pub(crate) fn begin_writer(
        &self,
        root: &Path,
        revision: &str,
        batch: &str,
    ) -> Result<CoordinatedLease> {
        anyhow::ensure!(
            uuid::Uuid::parse_str(batch).is_ok_and(|v| v.to_string() == batch),
            "Invalid batch revision"
        );
        let ticket = self.begin(root, revision)?;
        crate::core_shared_control::require_initialized(&ticket.shared)?;
        let guard = WriterGuard {
            coordinator: self.clone(),
            epoch: ticket.epoch,
            revision: revision.into(),
            alive: Arc::new(AtomicBool::new(true)),
            root: ticket.root.clone(),
            shared: ticket.shared.clone(),
        };
        Ok(CoordinatedLease {
            ticket,
            batch: batch.into(),
            guard,
            marker: None,
        })
    }
}
impl CoordinatedLease {
    pub(crate) fn guard(&self) -> WriterGuard {
        self.guard.clone()
    }
    pub(crate) fn start(&mut self, root: &Path, batch: &str) -> Result<PathBuf> {
        ensure!(
            root.canonicalize()? == self.ticket.root && batch == self.batch && self.guard.active(),
            "Coordinated lease scope changed or paused"
        );
        let path = self
            .ticket
            .root
            .join("state/core-writer")
            .join(format!("{batch}.json"));
        let marker = self.ticket.root.join("state/core-migration-active.json");
        ensure!(
            !marker.try_exists()? && !path.try_exists()?,
            "Coordinated open cannot resume recovery or an existing journal"
        );
        let expected = serde_json::to_string(
            &serde_json::json!({"journal": path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid journal path"))?, "revision": batch}),
        )?;
        crate::store::atomic_write(&marker, &expected)?;
        self.marker = Some((marker, expected));
        Ok(path)
    }
    pub(crate) fn finish(self) -> Result<()> {
        ensure!(self.guard.active(), "Coordinated grant revoked");
        crate::core_console::persisted_policy(&self.ticket.root, &self.guard.revision)?;
        let (path, expected) = self
            .marker
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing coordinated marker"))?;
        ensure!(
            std::fs::read_to_string(path)? == *expected,
            "Recovery marker changed; preserve it for review"
        );
        std::fs::remove_file(path)?;
        File::open(path.parent().unwrap())?.sync_all()?;
        Ok(())
    }
}

impl Drop for CoordinatedLease {
    fn drop(&mut self) {
        self.guard.alive.store(false, Ordering::SeqCst);
    }
}
