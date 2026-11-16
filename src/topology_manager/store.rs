//! One owner for durable registry edits. No shaping files are written here.

use super::model::{RegistryDocument, SubscriberRecord};
use anyhow::{Context, ensure};
use serde::Deserialize;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct RegistryStore {
    _lock: File,
    path: PathBuf,
    document: Option<RegistryDocument>,
    disk_schema: u32,
    history_cleanup_pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitOutcome {
    pub revision: u64,
    pub changed: bool,
}

impl RegistryStore {
    pub fn open(state_dir: &Path) -> anyhow::Result<Self> {
        let lock = crate::files::lock(state_dir)?;
        let private = state_dir.join("topology-manager");
        let created = match std::fs::DirBuilder::new().mode(0o700).create(&private) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
            Err(error) => return Err(error).context("cannot create private registry directory"),
        };
        check_private_directory(&private)?;
        // Confirm the private directory entry before a document can become durable.
        File::open(state_dir)?.sync_all()?;
        let path = private.join("registry.json");
        let loaded = match load(&path)? {
            Some(loaded) => loaded,
            None if created => {
                let empty = RegistryDocument::default();
                crate::files::write_secret(&path, &serde_json::to_vec(&empty)?)?;
                LoadedDocument {
                    bytes: std::fs::read(&path)?,
                    document: empty,
                    disk_schema: 3,
                }
            }
            None => anyhow::bail!(
                "registry file is missing from an existing directory. Recovery is required"
            ),
        };
        // Reconfirm a rename that could have preceded an earlier directory-sync error.
        File::open(&private)?.sync_all()?;
        let mut store = Self {
            _lock: lock,
            path,
            document: Some(loaded.document),
            disk_schema: loaded.disk_schema,
            history_cleanup_pending: false,
        };
        store.history_cleanup_pending = store.prune_history().is_err();
        Ok(store)
    }

    pub fn document(&self) -> anyhow::Result<&RegistryDocument> {
        self.document
            .as_ref()
            .context("registry state is uncertain. Reopen the store")
    }

    /// Save the exact confirmed disk document. Never replace an existing backup.
    pub fn backup_current(&mut self) -> anyhow::Result<BackupId> {
        let current = self.document()?;
        let disk = load(&self.path);
        let loaded = match disk {
            Ok(Some(loaded))
                if loaded.document == *current && loaded.disk_schema == self.disk_schema =>
            {
                loaded
            }
            result => {
                self.document = None;
                result.context("cannot confirm registry before backup")?;
                anyhow::bail!("registry changed before backup. Reopen the store");
            }
        };
        let id = BackupId {
            schema_version: loaded.disk_schema,
            revision: loaded.document.revision,
        };
        let path = self.backup_path(id)?;
        match load(&path)? {
            Some(existing) => ensure!(
                existing.bytes == loaded.bytes,
                "existing registry backup conflicts with current bytes"
            ),
            None => write_backup(&path, &loaded.bytes)?,
        }
        let verified = load(&path)?.context("registry backup is missing")?;
        ensure!(
            verified.bytes == loaded.bytes,
            "registry backup verification failed"
        );
        File::open(path.parent().context("registry directory is missing")?)?.sync_all()?;
        self.history_cleanup_pending = self.prune_history().is_err();
        Ok(id)
    }

    /// History maintenance errors do not change the result of a confirmed policy write.
    pub fn history_cleanup_pending(&self) -> bool {
        self.history_cleanup_pending
    }

    fn prune_history(&self) -> anyhow::Result<()> {
        let revision = self.document()?.revision;
        let directory = self
            .path
            .parent()
            .context("registry directory is missing")?;
        let mut history = Vec::new();
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let Some(number) = name
                .strip_prefix("registry-v3-r")
                .and_then(|s| s.strip_suffix(".json"))
            else {
                continue;
            };
            let Ok(number) = number.parse::<u64>() else {
                continue;
            };
            if number <= revision && name == format!("registry-v3-r{number}.json") {
                history.push((number, entry.path()));
            }
        }
        history.sort_unstable_by_key(|(revision, _)| *revision);
        let obsolete = history.len().saturating_sub(16);
        for (revision, path) in history.into_iter().take(obsolete) {
            let loaded = load(&path)?.context("registry history entry is missing")?;
            ensure!(
                loaded.disk_schema == 3 && loaded.document.revision == revision,
                "registry history metadata conflict"
            );
            std::fs::remove_file(path)?;
        }
        if obsolete > 0 {
            File::open(directory)?.sync_all()?;
        }
        Ok(())
    }

    /// Restore topology/source settings as a forward edit. Keep current identities and subscriber intent.
    pub fn restore_policy(
        &mut self,
        expected_revision: u64,
        backup: BackupId,
    ) -> anyhow::Result<CommitOutcome> {
        let current = self.document()?;
        ensure!(
            current.revision == expected_revision,
            "policy recovery revision conflict"
        );
        ensure!(
            backup.revision <= expected_revision,
            "policy recovery cannot use a future backup"
        );
        let loaded = load(&self.backup_path(backup)?)?.context("registry backup is missing")?;
        ensure!(
            loaded.disk_schema == backup.schema_version
                && loaded.document.revision == backup.revision,
            "registry backup metadata conflict"
        );
        let mut candidate = current.clone();
        candidate.catalog = loaded.document.catalog;
        candidate.source_settings = loaded.document.source_settings;
        self.commit(expected_revision, candidate)
    }

    fn backup_path(&self, backup: BackupId) -> anyhow::Result<PathBuf> {
        ensure!(
            (1..=3).contains(&backup.schema_version),
            "unsupported registry backup schema"
        );
        Ok(self.path.with_file_name(format!(
            "registry-v{}-r{}.json",
            backup.schema_version, backup.revision
        )))
    }

    pub fn commit(
        &mut self,
        expected_revision: u64,
        candidate: RegistryDocument,
    ) -> anyhow::Result<CommitOutcome> {
        self.commit_inner(expected_revision, candidate, crate::files::write_secret)
    }

    fn commit_inner(
        &mut self,
        expected_revision: u64,
        mut candidate: RegistryDocument,
        write: impl FnOnce(&Path, &[u8]) -> anyhow::Result<()>,
    ) -> anyhow::Result<CommitOutcome> {
        let current = self.document()?;
        ensure!(
            expected_revision == current.revision,
            "subscriber registry revision conflict"
        );
        ensure!(
            candidate.revision == expected_revision,
            "candidate registry revision conflict"
        );
        let disk_matches = load(&self.path).map(|disk| {
            disk.as_ref().is_some_and(|loaded| {
                loaded.document == *current && loaded.disk_schema == self.disk_schema
            })
        });
        match disk_matches {
            Ok(true) => {}
            result => {
                self.document = None;
                result.context("cannot confirm registry on disk")?;
                anyhow::bail!("subscriber registry changed on disk. Reopen the store");
            }
        }

        let current = self.document()?;
        ensure!(
            current.source_settings.is_none() || candidate.source_settings.is_some(),
            "ordinary edit cannot remove configured source settings"
        );
        if let Some(settings) = &mut candidate.source_settings {
            settings.canonicalize();
        }
        ensure!(
            current.catalog.is_none() || candidate.catalog.is_some(),
            "ordinary edit cannot remove a configured catalog"
        );
        if let Some(catalog) = &mut candidate.catalog {
            catalog.canonicalize();
            if let Some(previous) = &current.catalog {
                catalog.validate_origins(previous)?;
            }
        }
        candidate
            .subscribers
            .sort_unstable_by(|a, b| a.id.cmp(&b.id));
        for subscriber in &mut candidate.subscribers {
            subscriber.identities.sort_unstable();
        }
        let index = candidate.index()?;
        ensure!(
            candidate.next_id >= current.next_id,
            "subscriber ID counter cannot move backward"
        );
        for subscriber in &current.subscribers {
            ensure!(
                index.by_id.contains_key(subscriber.id.as_str()),
                "ordinary edit cannot delete a subscriber registry record"
            );
            for identity in &subscriber.identities {
                let owner = index
                    .by_identity
                    .get(identity)
                    .context("ordinary edit cannot remove a verified identity")?;
                ensure!(
                    candidate.subscribers[*owner].id == subscriber.id,
                    "ordinary edit cannot transfer identity ownership"
                );
            }
        }
        if candidate == *current && self.disk_schema == 3 {
            return Ok(CommitOutcome {
                revision: current.revision,
                changed: false,
            });
        }
        candidate.revision = current
            .revision
            .checked_add(1)
            .context("subscriber registry revision is exhausted")?;
        let bytes = serde_json::to_vec(&candidate).context("cannot encode subscriber registry")?;
        let revision = candidate.revision;
        self.backup_current()?;
        // An error can follow rename. Never expose old memory as a confirmed current document.
        self.document = None;
        write(&self.path, &bytes)?;
        self.document = Some(candidate);
        self.disk_schema = 3;
        Ok(CommitOutcome {
            revision,
            changed: true,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackupId {
    pub schema_version: u32,
    pub revision: u64,
}

fn write_backup(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temp = path.with_extension(format!("tmp-{}-{sequence}", std::process::id()));
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&temp)?;
    let result = (|| -> anyhow::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        // A hard link publishes the complete file without replacing an existing backup.
        std::fs::hard_link(&temp, path)?;
        File::open(path.parent().context("registry directory is missing")?)?.sync_all()?;
        Ok(())
    })();
    drop(file);
    let cleanup = std::fs::remove_file(&temp);
    result?;
    cleanup?;
    File::open(path.parent().context("registry directory is missing")?)?.sync_all()?;
    Ok(())
}

fn check_private_directory(path: &Path) -> anyhow::Result<()> {
    let metadata =
        std::fs::symlink_metadata(path).context("cannot inspect private registry directory")?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "registry directory must be a real directory"
    );
    // SAFETY: geteuid has no preconditions.
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() } && metadata.mode() & 0o077 == 0,
        "registry directory must be private and owned by the current user"
    );
    Ok(())
}

struct LoadedDocument {
    document: RegistryDocument,
    disk_schema: u32,
    bytes: Vec<u8>,
}

#[derive(Deserialize)]
struct SchemaHeader {
    schema_version: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyRegistry {
    schema_version: u32,
    revision: u64,
    next_id: u64,
    subscribers: Vec<SubscriberRecord>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaTwoRegistry {
    schema_version: u32,
    revision: u64,
    next_id: u64,
    subscribers: Vec<SubscriberRecord>,
    catalog: Option<super::catalog::TopologyCatalog>,
}

fn load(path: &Path) -> anyhow::Result<Option<LoadedDocument>> {
    check_private_directory(path.parent().context("registry directory is missing")?)?;
    let mut file = match File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("cannot open subscriber registry"),
    };
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file(),
        "subscriber registry must be a regular file"
    );
    // SAFETY: geteuid has no preconditions.
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() } && metadata.mode() & 0o077 == 0,
        "subscriber registry must be private and owned by the current user"
    );
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let header: SchemaHeader =
        serde_json::from_slice(&bytes).context("cannot decode registry schema")?;
    // Decode the original bytes again so duplicate fields cannot disappear into a JSON map.
    let document = match header.schema_version {
        1 => {
            let legacy: LegacyRegistry =
                serde_json::from_slice(&bytes).context("cannot decode schema-1 registry")?;
            ensure!(
                legacy.schema_version == 1,
                "registry schema changed during load"
            );
            RegistryDocument {
                schema_version: 3,
                revision: legacy.revision,
                next_id: legacy.next_id,
                subscribers: legacy.subscribers,
                catalog: None,
                source_settings: None,
            }
        }
        2 => {
            let legacy: SchemaTwoRegistry =
                serde_json::from_slice(&bytes).context("cannot decode schema-2 registry")?;
            ensure!(
                legacy.schema_version == 2,
                "registry schema changed during load"
            );
            RegistryDocument {
                schema_version: 3,
                revision: legacy.revision,
                next_id: legacy.next_id,
                subscribers: legacy.subscribers,
                catalog: legacy.catalog,
                source_settings: None,
            }
        }
        3 => serde_json::from_slice(&bytes).context("cannot decode schema-3 registry")?,
        _ => anyhow::bail!("unsupported subscriber registry schema"),
    };
    document.validate()?;
    Ok(Some(LoadedDocument {
        document,
        disk_schema: header.schema_version,
        bytes,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology_manager::model::{
        IdentityRef, SourceKind, SourceScope, SubscriberPolicy, SubscriberRecord,
    };
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

    fn candidate() -> RegistryDocument {
        RegistryDocument {
            next_id: 2,
            subscribers: vec![SubscriberRecord {
                id: "tm-sub-0000000000000001".into(),
                identities: vec![IdentityRef {
                    scope: SourceScope {
                        router_id: "router-1".into(),
                        source_id: "service-1".into(),
                        kind: SourceKind::Ppp,
                    },
                    account_ref: "account-1".into(),
                }],
                policy: SubscriberPolicy::default(),
            }],
            ..RegistryDocument::default()
        }
    }

    fn path(root: &Path) -> PathBuf {
        root.join("topology-manager/registry.json")
    }

    #[test]
    fn commit_is_private_and_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(dir.path()).unwrap();
        assert_eq!(store.document().unwrap(), &RegistryDocument::default());
        assert_eq!(
            store.commit(0, candidate()).unwrap(),
            CommitOutcome {
                revision: 1,
                changed: true
            }
        );
        let expected = store.document().unwrap().clone();
        assert_eq!(
            std::fs::metadata(path(dir.path()))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(path(dir.path()).parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        drop(store);
        let reopened = RegistryStore::open(dir.path()).unwrap();
        assert_eq!(reopened.document().unwrap(), &expected);
        let text = std::fs::read_to_string(path(dir.path())).unwrap();
        assert!(!text.contains("session_id") && !text.contains("ips") && !text.contains("mac"));
    }

    #[test]
    fn store_holds_the_existing_daemon_instance_lock() {
        let dir = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(dir.path()).unwrap();
        assert!(RegistryStore::open(dir.path()).is_err());
        assert!(crate::files::lock(dir.path()).is_err());
        drop(store);
        RegistryStore::open(dir.path()).unwrap();
    }

    #[test]
    fn unchanged_candidate_keeps_revision_bytes_and_inode() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(dir.path()).unwrap();
        assert!(
            !store
                .commit(0, RegistryDocument::default())
                .unwrap()
                .changed
        );
        assert!(path(dir.path()).exists());
        store.commit(0, candidate()).unwrap();
        let before = std::fs::read(path(dir.path())).unwrap();
        let inode = std::fs::metadata(path(dir.path())).unwrap().ino();
        assert_eq!(
            store.commit(1, store.document().unwrap().clone()).unwrap(),
            CommitOutcome {
                revision: 1,
                changed: false
            }
        );
        assert_eq!(std::fs::read(path(dir.path())).unwrap(), before);
        assert_eq!(std::fs::metadata(path(dir.path())).unwrap().ino(), inode);
    }

    #[test]
    fn stale_revision_and_invalid_candidate_leave_current_state_intact() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(dir.path()).unwrap();
        store.commit(0, candidate()).unwrap();
        let before = std::fs::read(path(dir.path())).unwrap();
        assert!(store.commit(0, candidate()).is_err());
        let mut invalid = store.document().unwrap().clone();
        invalid.next_id = 1;
        assert!(store.commit(1, invalid).is_err());
        let mut invalid = store.document().unwrap().clone();
        invalid.revision = 2;
        assert!(store.commit(1, invalid).is_err());
        assert_eq!(store.document().unwrap().revision, 1);
        assert_eq!(std::fs::read(path(dir.path())).unwrap(), before);
    }

    #[test]
    fn ordinary_edits_cannot_delete_records_or_transfer_identities() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(dir.path()).unwrap();
        store.commit(0, candidate()).unwrap();
        let mut removed = store.document().unwrap().clone();
        removed.subscribers.clear();
        assert!(store.commit(1, removed).is_err());
        let mut stolen = store.document().unwrap().clone();
        stolen.subscribers[0].id = "different-id".into();
        assert!(store.commit(1, stolen).is_err());
        let mut replaced = store.document().unwrap().clone();
        replaced.subscribers[0].identities[0].account_ref = "different-account".into();
        assert!(store.commit(1, replaced).is_err());
        let mut changed = store.document().unwrap().clone();
        changed.subscribers[0].policy.pin = Some("desired-instance".into());
        assert_eq!(store.commit(1, changed).unwrap().revision, 2);
    }

    #[test]
    fn id_counter_cannot_roll_back_even_when_no_current_id_uses_the_gap() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(dir.path()).unwrap();
        let mut doc = candidate();
        doc.next_id = 100;
        store.commit(0, doc).unwrap();
        let mut rolled_back = store.document().unwrap().clone();
        rolled_back.next_id = 2;
        assert!(store.commit(1, rolled_back).is_err());
        assert_eq!(store.document().unwrap().next_id, 100);
    }

    #[test]
    fn corrupt_and_future_schema_files_do_not_reset_the_store() {
        for bad in [
            "{",
            r#"{"schema_version":4,"revision":0,"next_id":1,"subscribers":[]}"#,
            r#"{"schema_version":1,"revision":0,"next_id":1,"subscribers":[],"ips":[]}"#,
        ] {
            let dir = tempfile::tempdir().unwrap();
            drop(RegistryStore::open(dir.path()).unwrap());
            crate::files::write_secret(&path(dir.path()), bad.as_bytes()).unwrap();
            assert!(RegistryStore::open(dir.path()).is_err());
            assert_eq!(
                std::fs::read_to_string(path(dir.path()))
                    .unwrap()
                    .trim_end(),
                bad
            );
        }
    }

    #[test]
    fn external_replacement_or_deletion_blocks_further_use() {
        for delete in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let mut store = RegistryStore::open(dir.path()).unwrap();
            store.commit(0, candidate()).unwrap();
            if delete {
                std::fs::remove_file(path(dir.path())).unwrap();
            } else {
                let mut edited = store.document().unwrap().clone();
                edited.next_id = 100;
                crate::files::write_secret(
                    &path(dir.path()),
                    &serde_json::to_vec(&edited).unwrap(),
                )
                .unwrap();
            }
            let edit = candidate();
            assert!(
                store
                    .commit(
                        1,
                        RegistryDocument {
                            revision: 1,
                            ..edit
                        }
                    )
                    .is_err()
            );
            assert!(store.document().is_err());
        }
    }

    #[test]
    fn failed_replacement_blocks_use_until_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(dir.path()).unwrap();
        let initial = std::fs::read(path(dir.path())).unwrap();
        let result = store.commit_inner(0, candidate(), |_, _| {
            anyhow::bail!("injected write failure")
        });
        assert!(result.is_err());
        assert!(store.document().is_err());
        assert!(store.commit(0, candidate()).is_err());
        assert_eq!(std::fs::read(path(dir.path())).unwrap(), initial);
        drop(store);
        assert_eq!(
            RegistryStore::open(dir.path())
                .unwrap()
                .document()
                .unwrap()
                .revision,
            0
        );
    }

    #[test]
    fn failure_after_rename_does_not_claim_old_memory_is_current() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(dir.path()).unwrap();
        assert!(
            store
                .commit_inner(0, candidate(), |path, bytes| {
                    crate::files::write_secret(path, bytes)?;
                    anyhow::bail!("injected error after replacement")
                })
                .is_err()
        );
        assert!(store.document().is_err());
        drop(store);
        let reopened = RegistryStore::open(dir.path()).unwrap();
        assert_eq!(reopened.document().unwrap().revision, 1);
        assert_eq!(reopened.document().unwrap().subscribers.len(), 1);
    }

    #[test]
    fn revision_exhaustion_keeps_existing_bytes_and_memory() {
        let dir = tempfile::tempdir().unwrap();
        drop(RegistryStore::open(dir.path()).unwrap());
        let doc = RegistryDocument {
            revision: u64::MAX,
            ..RegistryDocument::default()
        };
        crate::files::write_secret(&path(dir.path()), &serde_json::to_vec(&doc).unwrap()).unwrap();
        let mut store = RegistryStore::open(dir.path()).unwrap();
        let before = std::fs::read(path(dir.path())).unwrap();
        assert!(
            store
                .commit(
                    u64::MAX,
                    RegistryDocument {
                        revision: u64::MAX,
                        ..candidate()
                    }
                )
                .is_err()
        );
        assert_eq!(store.document().unwrap(), &doc);
        assert_eq!(std::fs::read(path(dir.path())).unwrap(), before);
    }

    #[test]
    fn symlinked_or_nonprivate_policy_files_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let foreign = tempfile::tempdir().unwrap();
        drop(RegistryStore::open(dir.path()).unwrap());
        let target = foreign.path().join("foreign.json");
        crate::files::write_secret(
            &target,
            &serde_json::to_vec(&RegistryDocument::default()).unwrap(),
        )
        .unwrap();
        std::fs::remove_file(path(dir.path())).unwrap();
        symlink(&target, path(dir.path())).unwrap();
        assert!(RegistryStore::open(dir.path()).is_err());
        std::fs::remove_file(path(dir.path())).unwrap();
        crate::files::write_secret(
            &path(dir.path()),
            &serde_json::to_vec(&RegistryDocument::default()).unwrap(),
        )
        .unwrap();
        std::fs::set_permissions(path(dir.path()), std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(RegistryStore::open(dir.path()).is_err());
    }

    #[test]
    fn missing_registry_after_restart_is_not_a_new_empty_registry() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(dir.path()).unwrap();
        store.commit(0, candidate()).unwrap();
        drop(store);
        std::fs::remove_file(path(dir.path())).unwrap();
        assert!(RegistryStore::open(dir.path()).is_err());
        assert!(!path(dir.path()).exists());
    }

    #[test]
    fn non_regular_registry_is_rejected_without_waiting_for_a_writer() {
        use std::os::unix::ffi::OsStrExt;
        let dir = tempfile::tempdir().unwrap();
        drop(RegistryStore::open(dir.path()).unwrap());
        std::fs::remove_file(path(dir.path())).unwrap();
        let name = std::ffi::CString::new(path(dir.path()).as_os_str().as_bytes()).unwrap();
        // SAFETY: name is a live NUL-terminated path; the mode is a valid permission mask.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(RegistryStore::open(dir.path()).is_err());
    }

    pub(super) fn catalog_candidate() -> RegistryDocument {
        let mut doc = candidate();
        doc.catalog = Some(super::super::catalog::tests::catalog());
        doc.subscribers[0].identities[0].scope = SourceScope {
            router_id: "distro".into(),
            source_id: "ppp-service".into(),
            kind: SourceKind::Ppp,
        };
        doc.subscribers[0].policy = SubscriberPolicy {
            profile_id: Some("local-plan".into()),
            assignment: Some("group1".into()),
            pin: Some("group1".into()),
        };
        doc
    }

    #[test]
    fn legacy_upgrade_preserves_intent_and_requires_an_explicit_commit() {
        let dir = tempfile::tempdir().unwrap();
        drop(RegistryStore::open(dir.path()).unwrap());
        let mut legacy = candidate();
        legacy.revision = 7;
        legacy.next_id = 100;
        legacy.subscribers[0].policy = SubscriberPolicy {
            profile_id: Some("legacy-plan".into()),
            assignment: Some("legacy-instance".into()),
            pin: Some("legacy-instance".into()),
        };
        let mut json = serde_json::to_value(&legacy).unwrap();
        json["schema_version"] = serde_json::json!(1);
        json.as_object_mut().unwrap().remove("catalog");
        json.as_object_mut().unwrap().remove("source_settings");
        crate::files::write_secret(&path(dir.path()), &serde_json::to_vec(&json).unwrap()).unwrap();
        let before = std::fs::read(path(dir.path())).unwrap();
        let mut store = RegistryStore::open(dir.path()).unwrap();
        assert_eq!(store.document().unwrap().schema_version, 3);
        assert_eq!(store.document().unwrap().subscribers, legacy.subscribers);
        assert_eq!(store.document().unwrap().next_id, 100);
        assert_eq!(store.document().unwrap().revision, 7);
        assert!(store.document().unwrap().catalog.is_none());
        assert_eq!(std::fs::read(path(dir.path())).unwrap(), before);
        let upgraded = store.document().unwrap().clone();
        assert_eq!(
            store.commit(7, upgraded.clone()).unwrap(),
            CommitOutcome {
                revision: 8,
                changed: true
            }
        );
        assert!(store.commit(7, upgraded).is_err());
        drop(store);
        let reopened = RegistryStore::open(dir.path()).unwrap();
        assert_eq!(reopened.document().unwrap().revision, 8);
        assert_eq!(reopened.document().unwrap().subscribers, legacy.subscribers);
        assert_eq!(reopened.document().unwrap().schema_version, 3);
    }

    #[test]
    fn a_catalog_and_its_subscriber_bindings_commit_as_one_checked_revision() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(dir.path()).unwrap();
        store.commit(0, catalog_candidate()).unwrap();
        let before = std::fs::read(path(dir.path())).unwrap();
        let mut removed = store.document().unwrap().clone();
        let c = removed.catalog.as_mut().unwrap();
        c.nodes.retain(|n| n.id != "group1");
        c.sources
            .iter_mut()
            .find(|s| s.id == "profile-source")
            .unwrap()
            .targets
            .retain(|id| id != "group1");
        assert!(store.commit(1, removed.clone()).is_err());
        assert_eq!(std::fs::read(path(dir.path())).unwrap(), before);
        assert_eq!(store.document().unwrap().revision, 1);
        removed.subscribers[0].policy.assignment = Some("group2".into());
        removed.subscribers[0].policy.pin = Some("group2".into());
        assert_eq!(store.commit(1, removed).unwrap().revision, 2);
        let mut cleared = store.document().unwrap().clone();
        cleared.catalog = None;
        assert!(store.commit(2, cleared).is_err());
    }

    #[test]
    fn strict_schema_loading_rejects_a_false_downgrade_and_duplicate_fields() {
        for text in [
            r#"{"schema_version":1,"revision":0,"revision":1,"next_id":1,"subscribers":[]}"#,
            r#"{"schema_version":1,"revision":0,"next_id":1,"subscribers":[],"catalog":null}"#,
            r#"{"schema_version":4,"revision":0,"next_id":1,"subscribers":[]}"#,
        ] {
            let dir = tempfile::tempdir().unwrap();
            drop(RegistryStore::open(dir.path()).unwrap());
            crate::files::write_secret(&path(dir.path()), text.as_bytes()).unwrap();
            assert!(RegistryStore::open(dir.path()).is_err());
        }
    }

    #[test]
    fn an_existing_source_id_cannot_change_its_authoritative_origin() {
        use super::super::catalog::SourceOrigin;
        let dir = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(dir.path()).unwrap();
        store.commit(0, catalog_candidate()).unwrap();
        let before = std::fs::read(path(dir.path())).unwrap();
        let mut replaced = store.document().unwrap().clone();
        let source = replaced
            .catalog
            .as_mut()
            .unwrap()
            .sources
            .iter_mut()
            .find(|s| s.id == "wan-source-1")
            .unwrap();
        if let SourceOrigin::Wan { endpoint_ref, .. } = &mut source.origin {
            *endpoint_ref = "different-endpoint".into();
        }
        assert!(store.commit(1, replaced).is_err());
        assert_eq!(std::fs::read(path(dir.path())).unwrap(), before);
        let mut factors = store.document().unwrap().clone();
        let source = factors
            .catalog
            .as_mut()
            .unwrap()
            .sources
            .iter_mut()
            .find(|s| s.id == "profile-source")
            .unwrap();
        if let SourceOrigin::Subscribers { group, .. } = &mut source.origin {
            group.factors.down = super::super::rates::Fraction {
                numerator: 1,
                denominator: 2,
            };
        }
        assert_eq!(store.commit(1, factors).unwrap().revision, 2);
    }
}

#[cfg(test)]
#[path = "store_recovery_tests.rs"]
mod recovery_tests;
