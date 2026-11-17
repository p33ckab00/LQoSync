use super::*;
use crate::topology_manager::legacy_static;

fn proposal() -> legacy_static::StaticAdoption {
    let f = legacy_static::tests::fixture();
    legacy_static::adopt(
        &f.pair,
        &f.rows,
        &f.network,
        &f.manifest,
        &f.roots,
        &f.attachments,
    )
    .unwrap()
}

#[test]
fn installation_recovers_after_every_durable_boundary() {
    for stop in 0..5 {
        let state = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(state.path()).unwrap();
        let p = proposal();
        stage(&mut store, &p).unwrap();
        assert!(store.commit(0, store.document().unwrap().clone()).is_err());
        assert!(super::super::runtime::topology_owns(state.path()).is_err());
        assert!(
            recover_inner(&mut store, |step| {
                anyhow::ensure!(step != stop, "simulated interruption");
                Ok(())
            })
            .is_err()
        );
        drop(store);
        let mut store = RegistryStore::open(state.path()).unwrap();
        assert!(recover(&mut store).unwrap());
        assert_eq!(store.document().unwrap().revision, 1);
        let source = super::super::static_input::CurrentStatic::load(state.path()).unwrap();
        assert_eq!(source.binding.revision, 1);
        assert_eq!(
            std::fs::read(source.binding.path).unwrap(),
            p.source_document(1).unwrap()
        );
        assert!(!recover(&mut store).unwrap());
        assert!(!super::super::runtime::topology_owns(state.path()).unwrap());
    }
}

#[tokio::test(start_paused = true)]
async fn changed_and_expired_proposals_leave_no_transaction() {
    let state = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(state.path()).unwrap();
    let mut p = proposal();
    p.registry.next_id += 1;
    assert!(stage(&mut store, &p).is_err());
    let p = proposal();
    tokio::time::advance(std::time::Duration::from_secs(11)).await;
    assert!(stage(&mut store, &p).is_err());
    assert!(absent(&directory(&store).join(JOURNAL)).unwrap());
    assert_eq!(store.document().unwrap().revision, 0);
}

#[test]
fn existing_sources_and_owners_are_never_adopted_or_replaced_implicitly() {
    for name in ["static-input.json", OWNER] {
        let state = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(state.path()).unwrap();
        let path = directory(&store).join(name);
        crate::files::write_secret(&path, b"existing").unwrap();
        assert!(stage(&mut store, &proposal()).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"existing\n");
        assert!(absent(&directory(&store).join(JOURNAL)).unwrap());
    }
}

#[test]
fn cancellation_is_available_only_before_source_installation() {
    for stop in [0, 1, 2, 3] {
        let state = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(state.path()).unwrap();
        stage(&mut store, &proposal()).unwrap();
        assert!(
            recover_inner(&mut store, |step| {
                anyhow::ensure!(step != stop, "interrupted");
                Ok(())
            })
            .is_err()
        );
        if stop < 2 {
            cancel(&mut store).unwrap();
            assert!(!recover(&mut store).unwrap());
            assert!(!super::super::runtime::topology_owns(state.path()).unwrap());
            stage(&mut store, &proposal()).unwrap();
        } else {
            assert!(cancel(&mut store).is_err());
        }
        recover(&mut store).unwrap();
        assert_eq!(store.document().unwrap().revision, 1);
    }
}

#[test]
fn unknown_source_and_owner_changes_block_recovery_and_keep_the_journal() {
    for name in ["static-input.json", OWNER] {
        let state = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(state.path()).unwrap();
        stage(&mut store, &proposal()).unwrap();
        let path = directory(&store).join(name);
        crate::files::write_secret(&path, b"unknown").unwrap();
        assert!(recover(&mut store).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"unknown\n");
        assert_eq!(store.document().unwrap().revision, 0);
        assert!(!absent(&directory(&store).join(JOURNAL)).unwrap());
        assert!(super::super::runtime::claim(state.path()).is_err());
    }
}

#[test]
fn source_removal_or_change_after_registry_commit_requires_explicit_repair() {
    for remove in [false, true] {
        let state = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(state.path()).unwrap();
        stage(&mut store, &proposal()).unwrap();
        assert!(
            recover_inner(&mut store, |step| {
                anyhow::ensure!(step != 3, "interrupted after registry commit");
                Ok(())
            })
            .is_err()
        );
        let source = directory(&store).join("static-input.json");
        if remove {
            std::fs::remove_file(&source).unwrap();
        } else {
            crate::files::write_secret(&source, b"changed").unwrap();
        }
        drop(store);
        let mut store = RegistryStore::open(state.path()).unwrap();
        assert!(recover(&mut store).is_err());
        assert_eq!(store.document().unwrap().revision, 1);
        assert!(super::super::runtime::topology_owns(state.path()).is_err());
        if remove {
            assert!(absent(&source).unwrap());
        } else {
            assert_eq!(std::fs::read(source).unwrap(), b"changed\n");
        }
    }
}

#[test]
fn unsafe_corrupt_and_future_journals_block_writers_without_installation() {
    use std::os::unix::fs::PermissionsExt;
    for case in 0..6 {
        let state = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(state.path()).unwrap();
        stage(&mut store, &proposal()).unwrap();
        let path = directory(&store).join(JOURNAL);
        match case {
            0 => crate::files::write_secret(&path, b"{").unwrap(),
            1 | 2 => {
                let mut j: Journal =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                if case == 1 {
                    j.schema_version += 1;
                } else {
                    j.payload.target.next_id += 1;
                }
                crate::files::write_secret(&path, &serde_json::to_vec(&j).unwrap()).unwrap();
            }
            3 => std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap(),
            4 => {
                std::fs::remove_file(&path).unwrap();
                std::os::unix::fs::symlink("/dev/null", &path).unwrap();
            }
            _ => std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_len(LIMIT as u64 + 1)
                .unwrap(),
        }
        assert!(recover(&mut store).is_err());
        assert!(cancel(&mut store).is_err());
        assert!(store.commit(0, store.document().unwrap().clone()).is_err());
        assert!(super::super::runtime::topology_owns(state.path()).is_err());
        assert!(absent(&directory(&store).join("static-input.json")).unwrap());
    }
}

#[test]
fn disk_registry_drift_is_detected_before_source_installation() {
    let state = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(state.path()).unwrap();
    stage(&mut store, &proposal()).unwrap();
    let dir = directory(&store);
    let mut changed = store.document().unwrap().clone();
    changed.revision = 7;
    crate::files::write_secret(
        &dir.join("registry.json"),
        &serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    assert!(recover(&mut store).is_err());
    assert!(store.document().is_err());
    drop(store);
    let mut store = RegistryStore::open(state.path()).unwrap();
    assert!(recover(&mut store).is_err());
    assert_eq!(store.document().unwrap().revision, 7);
    assert!(absent(&dir.join("static-input.json")).unwrap());
}

#[test]
fn symlinked_source_is_never_followed_or_replaced() {
    let state = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(state.path()).unwrap();
    stage(&mut store, &proposal()).unwrap();
    let path = directory(&store).join("static-input.json");
    std::os::unix::fs::symlink("/dev/null", &path).unwrap();
    assert!(recover(&mut store).is_err());
    assert!(
        std::fs::symlink_metadata(path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(store.document().unwrap().revision, 0);
}

#[tokio::test(start_paused = true)]
async fn staged_recovery_does_not_reuse_expired_dynamic_observations() {
    let state = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(state.path()).unwrap();
    stage(&mut store, &proposal()).unwrap();
    tokio::time::advance(std::time::Duration::from_secs(60)).await;
    recover(&mut store).unwrap();
    let journal_path = directory(&store).join(JOURNAL);
    assert!(absent(&journal_path).unwrap());
    let bytes = std::fs::read(directory(&store).join("static-input.json")).unwrap();
    let source: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(source["rows"].as_array().unwrap().len(), 1);
    let rows: Vec<crate::model::CsvRow> = serde_json::from_value(source["rows"].clone()).unwrap();
    assert_eq!(rows[0].circuit_id, "static-circuit");
    // Recovery installs policy only. A new publication still needs fresh collection.
    assert!(!super::super::runtime::topology_owns(state.path()).unwrap());
}

#[test]
fn a_replaced_instance_lock_prevents_staging_and_recovery() {
    for pending in [false, true] {
        let state = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(state.path()).unwrap();
        if pending {
            stage(&mut store, &proposal()).unwrap();
        }
        let path = state.path().join("lqosync3.lock");
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"replaced").unwrap();
        if pending {
            assert!(recover(&mut store).is_err());
        } else {
            assert!(stage(&mut store, &proposal()).is_err());
        }
        assert!(absent(&directory(&store).join("static-input.json")).unwrap());
    }
}

#[test]
fn the_pending_owner_marker_alone_blocks_the_strict_legacy_writer() {
    let state = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(state.path()).unwrap();
    stage(&mut store, &proposal()).unwrap();
    assert!(
        recover_inner(&mut store, |step| {
            anyhow::ensure!(step != 1, "interrupted after owner marker");
            Ok(())
        })
        .is_err()
    );
    let dir = directory(&store);
    // Remove only the new guard to exercise the earlier ownership parser.
    std::fs::remove_file(dir.join(JOURNAL)).unwrap();
    let error = super::super::runtime::topology_owns(state.path()).unwrap_err();
    assert!(error.to_string().contains("unsupported writer ownership"));
}

#[test]
fn installation_publishes_exact_bytes_without_replacing_existing_files() {
    let state = tempfile::tempdir().unwrap();
    let store = RegistryStore::open(state.path()).unwrap();
    let path = directory(&store).join("exclusive-test.json");
    super::super::store::write_new(&path, b"first").unwrap();
    assert!(super::super::store::write_new(&path, b"second").is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"first");
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink("/dev/null", &path).unwrap();
    assert!(super::super::store::write_new(&path, b"second").is_err());
    assert!(
        std::fs::symlink_metadata(path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn a_copied_journal_cannot_install_into_another_runtime_owner() {
    let original = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let mut first = RegistryStore::open(original.path()).unwrap();
    let mut second = RegistryStore::open(other.path()).unwrap();
    stage(&mut first, &proposal()).unwrap();
    let bytes = std::fs::read(directory(&first).join(JOURNAL)).unwrap();
    crate::files::write_secret(&directory(&second).join(JOURNAL), &bytes).unwrap();
    assert!(recover(&mut second).is_err());
    assert!(cancel(&mut second).is_err());
    assert!(absent(&directory(&second).join(OWNER)).unwrap());
    assert!(absent(&directory(&second).join("static-input.json")).unwrap());
    assert_eq!(second.document().unwrap().revision, 0);
    recover(&mut first).unwrap();
}
