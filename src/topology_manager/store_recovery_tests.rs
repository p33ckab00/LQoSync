use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};

fn primary(root: &Path) -> PathBuf {
    root.join("topology-manager/registry.json")
}
fn backup_path(root: &Path, schema: u32, revision: u64) -> PathBuf {
    root.join(format!(
        "topology-manager/registry-v{schema}-r{revision}.json"
    ))
}

#[test]
fn schema_two_migrates_without_inventing_authority_or_rewriting_disk() {
    let root = tempfile::tempdir().unwrap();
    drop(RegistryStore::open(root.path()).unwrap());
    let old = br#"{"schema_version":2,"revision":7,"next_id":90,"subscribers":[],"catalog":null}"#;
    crate::files::write_secret(&primary(root.path()), old).unwrap();
    let before = std::fs::read(primary(root.path())).unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    let value = serde_json::to_value(store.document().unwrap()).unwrap();
    assert_eq!(value["schema_version"], SCHEMA);
    assert!(value["source_settings"].is_null());
    assert_eq!(value["next_id"], 90);
    assert_eq!(std::fs::read(primary(root.path())).unwrap(), before);
    let candidate = store.document().unwrap().clone();
    assert_eq!(store.commit(7, candidate).unwrap().revision, 8);
    assert_eq!(
        std::fs::read(backup_path(root.path(), 2, 7)).unwrap(),
        before
    );
}

#[test]
fn changed_commit_preserves_exact_previous_bytes_in_a_private_backup() {
    let root = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    let before = std::fs::read(primary(root.path())).unwrap();
    let mut next = store.document().unwrap().clone();
    next.next_id = 20;
    store.commit(0, next).unwrap();
    let backup = backup_path(root.path(), SCHEMA, 0);
    assert_eq!(std::fs::read(&backup).unwrap(), before);
    assert_eq!(
        std::fs::metadata(backup).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn conflicting_or_linked_backup_blocks_commit_without_primary_changes() {
    for linked in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(root.path()).unwrap();
        let before = std::fs::read(primary(root.path())).unwrap();
        let backup = backup_path(root.path(), SCHEMA, 0);
        if linked {
            symlink(primary(root.path()), &backup).unwrap();
        } else {
            crate::files::write_secret(&backup, b"corrupt evidence").unwrap();
        }
        let mut candidate = store.document().unwrap().clone();
        candidate.next_id = 3;
        assert!(store.commit(0, candidate).is_err());
        assert_eq!(std::fs::read(primary(root.path())).unwrap(), before);
        assert_eq!(store.document().unwrap().revision, 0);
    }
}

fn configured() -> RegistryDocument {
    use crate::topology_manager::identity::{ReferenceAuthority, SourceSelection};
    use crate::topology_manager::settings::SourceSettings;
    use crate::topology_manager::source_policy::{ProfileBinding, RateRead, SourcePolicy};
    let mut doc = super::tests::catalog_candidate();
    doc.source_settings = Some(SourceSettings {
        selections: vec![SourceSelection {
            scope: doc.catalog.as_ref().unwrap().required_scopes().unwrap()[0].clone(),
            service_names: vec![],
            authority: ReferenceAuthority::OperatorManagedComment,
            policy: SourcePolicy {
                profiles: vec![ProfileBinding {
                    source_ref: "provider-profile-id".into(),
                    current_name: "gold".into(),
                    rate: RateRead::Disabled,
                }],
                unprofiled_rate: RateRead::Disabled,
                legacy_priority: None,
                dhcp_overrides: vec![],
            },
        }],
    });
    doc
}

#[test]
fn saved_settings_survive_restart_and_order_only_edits_do_not_advance_revision() {
    let root = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    let mut doc = configured();
    doc.source_settings.as_mut().unwrap().selections[0].service_names =
        vec!["sstp".into(), "pppoe".into()];
    store.commit(0, doc).unwrap();
    drop(store);
    let mut store = RegistryStore::open(root.path()).unwrap();
    let mut doc = store.document().unwrap().clone();
    assert!(
        doc.source_settings
            .as_ref()
            .unwrap()
            .ready_selections(doc.catalog.as_ref().unwrap())
            .is_ok()
    );
    doc.source_settings.as_mut().unwrap().selections[0]
        .service_names
        .reverse();
    assert!(!store.commit(1, doc).unwrap().changed);
    assert!(!backup_path(root.path(), SCHEMA, 1).exists());
    assert_eq!(
        store.backup_current().unwrap(),
        BackupId {
            schema_version: SCHEMA,
            revision: 1
        }
    );
    let first = std::fs::read(backup_path(root.path(), SCHEMA, 1)).unwrap();
    store.backup_current().unwrap();
    assert_eq!(
        std::fs::read(backup_path(root.path(), SCHEMA, 1)).unwrap(),
        first
    );
}

#[test]
fn forward_policy_restore_keeps_new_subscribers_ids_pins_and_counter() {
    let root = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    store.commit(0, configured()).unwrap();
    let old = store.backup_current().unwrap();
    let mut next = store.document().unwrap().clone();
    next.catalog.as_mut().unwrap().root_label = "Revised topology".into();
    next.source_settings.as_mut().unwrap().selections[0].service_names = vec!["pppoe".into()];
    let mut added = next.subscribers[0].clone();
    added.id = "tm-sub-0000000000000002".into();
    added.identities[0].account_ref = "new-independent-reference".into();
    next.subscribers.push(added);
    next.next_id = 30;
    store.commit(1, next).unwrap();
    let current = store.document().unwrap().clone();
    assert_eq!(store.restore_policy(2, old).unwrap().revision, 3);
    let restored = store.document().unwrap();
    assert_eq!(restored.next_id, current.next_id);
    assert_eq!(restored.subscribers, current.subscribers);
    assert_ne!(restored.catalog, current.catalog);
    assert!(
        restored.source_settings.as_ref().unwrap().selections[0]
            .service_names
            .is_empty()
    );
    drop(store);
    assert_eq!(
        RegistryStore::open(root.path())
            .unwrap()
            .document()
            .unwrap()
            .revision,
        3
    );
}

#[test]
fn recovery_rejects_stale_revisions_wrong_metadata_and_incompatible_current_pins() {
    let root = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    store.commit(0, configured()).unwrap();
    let old = store.backup_current().unwrap();
    let mut next = store.document().unwrap().clone();
    let catalog = next.catalog.as_mut().unwrap();
    let mut node = catalog
        .nodes
        .iter()
        .find(|n| n.id == "group1")
        .unwrap()
        .clone();
    node.id = "new-group".into();
    node.source_id = None;
    catalog.nodes.push(node);
    catalog
        .sources
        .iter_mut()
        .find(|s| s.id == "profile-source")
        .unwrap()
        .targets
        .push("new-group".into());
    next.subscribers[0].policy.assignment = Some("new-group".into());
    next.subscribers[0].policy.pin = Some("new-group".into());
    store.commit(1, next).unwrap();
    let before = std::fs::read(primary(root.path())).unwrap();
    assert!(store.restore_policy(1, old).is_err());
    assert!(store.restore_policy(2, old).is_err());
    let bad = BackupId {
        schema_version: SCHEMA,
        revision: 2,
    };
    crate::files::write_secret(
        &backup_path(root.path(), SCHEMA, 2),
        &std::fs::read(backup_path(root.path(), SCHEMA, 1)).unwrap(),
    )
    .unwrap();
    assert!(store.restore_policy(2, bad).is_err());
    assert!(
        store
            .restore_policy(
                2,
                BackupId {
                    schema_version: 5,
                    revision: 1
                }
            )
            .is_err()
    );
    assert_eq!(std::fs::read(primary(root.path())).unwrap(), before);
    assert_eq!(store.document().unwrap().revision, 2);
}

#[test]
fn invalid_settings_or_removal_do_not_replace_the_confirmed_registry() {
    let root = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    store.commit(0, configured()).unwrap();
    let before = std::fs::read(primary(root.path())).unwrap();
    let mut next = store.document().unwrap().clone();
    next.source_settings.as_mut().unwrap().selections.clear();
    assert!(store.commit(1, next).is_err());
    let mut next = store.document().unwrap().clone();
    next.source_settings = None;
    assert!(store.commit(1, next).is_err());
    assert_eq!(std::fs::read(primary(root.path())).unwrap(), before);
    assert!(!backup_path(root.path(), SCHEMA, 1).exists());
}

#[test]
fn corrupt_primary_requires_evidence_even_if_an_older_backup_is_available() {
    let root = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    store.commit(0, configured()).unwrap();
    store.backup_current().unwrap();
    drop(store);
    crate::files::write_secret(&primary(root.path()), b"corrupt primary").unwrap();
    assert!(RegistryStore::open(root.path()).is_err());
    assert_eq!(
        std::fs::read(primary(root.path())).unwrap(),
        b"corrupt primary\n"
    );
}

#[test]
fn backup_preserves_a_legacy_file_without_a_trailing_newline() {
    let root = tempfile::tempdir().unwrap();
    drop(RegistryStore::open(root.path()).unwrap());
    let bytes = br#"{"schema_version":1,"revision":8,"next_id":200,"subscribers":[]}"#;
    std::fs::write(primary(root.path()), bytes).unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    assert!(store.document().unwrap().source_settings.is_none());
    let candidate = store.document().unwrap().clone();
    store.commit(8, candidate).unwrap();
    assert_eq!(
        std::fs::read(backup_path(root.path(), 1, 8)).unwrap(),
        bytes
    );
}

#[test]
fn normal_history_is_bounded_and_the_migration_backup_remains_available() {
    let root = tempfile::tempdir().unwrap();
    drop(RegistryStore::open(root.path()).unwrap());
    let old = br#"{"schema_version":2,"revision":7,"next_id":90,"subscribers":[],"catalog":null}"#;
    std::fs::write(primary(root.path()), old).unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    for revision in 7..27 {
        let mut candidate = store.document().unwrap().clone();
        candidate.next_id += 1;
        store.commit(revision, candidate).unwrap();
    }
    let history: Vec<_> = std::fs::read_dir(root.path().join("topology-manager"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(&format!("registry-v{SCHEMA}-")))
        .collect();
    assert_eq!(history.len(), 16);
    assert_eq!(std::fs::read(backup_path(root.path(), 2, 7)).unwrap(), old);
    assert!(backup_path(root.path(), SCHEMA, 26).exists());
    assert!(!backup_path(root.path(), SCHEMA, 8).exists());
}

#[test]
fn optional_history_cleanup_failure_does_not_hide_a_confirmed_commit() {
    let root = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    for revision in 0..17 {
        let mut next = store.document().unwrap().clone();
        next.next_id += 1;
        store.commit(revision, next).unwrap();
    }
    assert!(!store.history_cleanup_pending());
    let corrupt_old = backup_path(root.path(), SCHEMA, 0);
    crate::files::write_secret(&corrupt_old, b"corrupt old history").unwrap();
    let mut next = store.document().unwrap().clone();
    next.next_id += 1;
    assert_eq!(store.commit(17, next).unwrap().revision, 18);
    assert!(store.history_cleanup_pending());
    assert_eq!(store.document().unwrap().revision, 18);
    assert!(backup_path(root.path(), SCHEMA, 17).exists());
    drop(store);
    let reopened = RegistryStore::open(root.path()).unwrap();
    assert_eq!(reopened.document().unwrap().revision, 18);
    assert!(reopened.history_cleanup_pending());
}

#[test]
fn schema_three_upgrade_keeps_exact_backup_and_adds_no_ownership() {
    let root = tempfile::tempdir().unwrap();
    drop(RegistryStore::open(root.path()).unwrap());
    let old = br#"{"schema_version":3,"revision":9,"next_id":200,"subscribers":[],"catalog":null,"source_settings":null}"#;
    crate::files::write_secret(&primary(root.path()), old).unwrap();
    let before = std::fs::read(primary(root.path())).unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    assert_eq!(store.document().unwrap().schema_version, SCHEMA);
    assert!(store.document().unwrap().export_ownership.is_none());
    assert_eq!(std::fs::read(primary(root.path())).unwrap(), before);
    assert_eq!(
        store
            .commit(9, store.document().unwrap().clone())
            .unwrap()
            .revision,
        10
    );
    assert_eq!(
        std::fs::read(backup_path(root.path(), 3, 9)).unwrap(),
        before
    );
    drop(store);
    assert_eq!(
        RegistryStore::open(root.path())
            .unwrap()
            .document()
            .unwrap()
            .next_id,
        200
    );
}

#[test]
fn export_ownership_survives_restart_and_cannot_transfer_or_disappear() {
    use crate::topology_manager::ownership::{CircuitAlias, DeviceAlias, ExportOwnership};
    let root = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    let mut value = super::tests::candidate();
    value.export_ownership = Some(ExportOwnership {
        circuits: vec![CircuitAlias {
            subscriber_id: value.subscribers[0].id.clone(),
            id: "old-circuit".into(),
            name: "Alice".into(),
        }],
        devices: vec![DeviceAlias {
            identity: value.subscribers[0].identities[0].clone(),
            id: "old-device".into(),
            name: "Alice device".into(),
        }],
        ..Default::default()
    });
    store.commit(0, value).unwrap();
    drop(store);
    let mut store = RegistryStore::open(root.path()).unwrap();
    let original = store.document().unwrap().clone();
    assert_eq!(
        original.export_ownership.as_ref().unwrap().circuits[0].id,
        "old-circuit"
    );
    let before = std::fs::read(primary(root.path())).unwrap();
    for action in 0..3 {
        let mut next = original.clone();
        match action {
            0 => next.export_ownership = None,
            1 => next.export_ownership.as_mut().unwrap().circuits[0].id = "other".into(),
            _ => next.export_ownership.as_mut().unwrap().devices.clear(),
        }
        assert!(store.commit(1, next).is_err());
        assert_eq!(std::fs::read(primary(root.path())).unwrap(), before);
    }
    let mut renamed = original;
    renamed.export_ownership.as_mut().unwrap().circuits[0].name = "Alice new label".into();
    store.commit(1, renamed).unwrap();
}

fn located_catalog() -> crate::topology_manager::catalog::TopologyCatalog {
    let mut c = crate::topology_manager::catalog::tests::catalog();
    let g = c.nodes.iter_mut().find(|n| n.id == "group1").unwrap();
    g.geo = Some(web_contract::topology::NodeGeo {
        geo_type: web_contract::topology::GeoType::Site,
        location: Some(web_contract::topology::Location {
            latitude: "12.236199459585354".into(),
            longitude: "122.02419129509887".into(),
        }),
    });
    c
}

#[test]
fn schema_four_migrates_to_five_and_keeps_the_schema_four_backup() {
    let root = tempfile::tempdir().unwrap();
    drop(RegistryStore::open(root.path()).unwrap());
    let mut old = serde_json::to_value(RegistryDocument {
        catalog: Some(crate::topology_manager::catalog::tests::catalog()),
        ..RegistryDocument::default()
    })
    .unwrap();
    old["schema_version"] = 4.into();
    old["revision"] = 3.into();
    crate::files::write_secret(&primary(root.path()), &serde_json::to_vec(&old).unwrap()).unwrap();
    let before = std::fs::read(primary(root.path())).unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    let mut doc = store.document().unwrap().clone();
    assert_eq!(doc.schema_version, 5);
    let ids: std::collections::BTreeSet<_> = doc
        .catalog
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .map(|n| n.id.clone())
        .collect();
    doc.catalog = Some(located_catalog());
    assert_eq!(store.commit(3, doc).unwrap().revision, 4);
    assert_eq!(
        std::fs::read(backup_path(root.path(), 4, 3)).unwrap(),
        before
    );
    drop(store);
    let store = RegistryStore::open(root.path()).unwrap();
    let saved = store.document().unwrap();
    let disk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(primary(root.path())).unwrap()).unwrap();
    assert_eq!(disk["schema_version"], 5);
    let nodes = &saved.catalog.as_ref().unwrap().nodes;
    assert_eq!(
        nodes
            .iter()
            .map(|n| n.id.clone())
            .collect::<std::collections::BTreeSet<_>>(),
        ids
    );
    let g = nodes.iter().find(|n| n.id == "group1").unwrap();
    let geo = g.geo.as_ref().unwrap();
    assert_eq!(
        geo.location.as_ref().unwrap().latitude,
        "12.236199459585354"
    );
    assert_eq!(geo.geo_type, web_contract::topology::GeoType::Site);
}

#[test]
fn a_future_schema_or_a_schema_four_file_with_canvas_fields_is_refused() {
    let root = tempfile::tempdir().unwrap();
    drop(RegistryStore::open(root.path()).unwrap());
    let mut doc = serde_json::to_value(RegistryDocument {
        catalog: Some(located_catalog()),
        ..RegistryDocument::default()
    })
    .unwrap();
    for schema in [4, 6] {
        doc["schema_version"] = schema.into();
        crate::files::write_secret(&primary(root.path()), &serde_json::to_vec(&doc).unwrap())
            .unwrap();
        assert!(RegistryStore::open(root.path()).is_err(), "schema {schema}");
    }
}

#[test]
fn a_location_outside_the_valid_range_is_not_a_valid_registry() {
    let mut c = located_catalog();
    let g = c.nodes.iter_mut().find(|n| n.id == "group1").unwrap();
    g.geo.as_mut().unwrap().location = Some(web_contract::topology::Location {
        latitude: "91".into(),
        longitude: "0".into(),
    });
    assert!(c.validate().is_err());
}
