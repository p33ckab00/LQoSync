use super::*;
use crate::topology_manager::catalog::{CatalogRouter, TopologyCatalog};
use crate::topology_manager::store::RegistryStore;

fn router(id: String) -> CatalogRouter {
    CatalogRouter {
        id,
        name: "Same label".into(),
        roles: Vec::new(),
        ..Default::default()
    }
}

#[test]
fn typed_id_batches_share_one_counter_without_using_names() {
    let mut doc = RegistryDocument::default();
    let ids = reserve_ids(
        &mut doc,
        0,
        &[
            IdKind::Router,
            IdKind::Source,
            IdKind::Profile,
            IdKind::Node,
        ],
    )
    .unwrap();
    assert_eq!(
        ids,
        vec![
            "tm-router-0000000000000001",
            "tm-source-0000000000000002",
            "tm-profile-0000000000000003",
            "tm-node-0000000000000004"
        ]
    );
    assert_eq!(doc.next_id, 5);
    assert_eq!(doc.revision, 0);
    doc.validate().unwrap();
}

#[test]
fn stale_revision_exhaustion_and_invalid_documents_leave_the_draft_unchanged() {
    let mut doc = RegistryDocument::default();
    let original = doc.clone();
    assert!(reserve_ids(&mut doc, 1, &[IdKind::Node]).is_err());
    assert_eq!(doc, original);
    doc.next_id = u64::MAX - 1;
    let original = doc.clone();
    assert!(reserve_ids(&mut doc, 0, &[IdKind::Node, IdKind::Source]).is_err());
    assert_eq!(doc, original);
    doc.next_id = 0;
    let original = doc.clone();
    assert!(reserve_ids(&mut doc, 0, &[IdKind::Node]).is_err());
    assert_eq!(doc, original);
}

#[test]
fn an_empty_reservation_does_not_advance_the_counter() {
    let mut doc = RegistryDocument::default();
    let original = doc.clone();
    assert!(reserve_ids(&mut doc, 0, &[]).unwrap().is_empty());
    assert_eq!(doc, original);
}

#[test]
fn generated_catalog_ids_require_matching_kind_valid_sequence_and_a_higher_counter() {
    for id in [
        "tm-router-0000000000000000",
        "tm-router-0000000000000002",
        "tm-router-000000000000000A",
        "tm-router-invalid",
        "tm-node-0000000000000001",
    ] {
        let doc = RegistryDocument {
            next_id: 2,
            catalog: Some(TopologyCatalog {
                routers: vec![router(id.into())],
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(doc.validate().is_err(), "accepted {id}");
    }
    let doc = RegistryDocument {
        next_id: 2,
        catalog: Some(TopologyCatalog {
            routers: vec![router("tm-router-0000000000000001".into())],
            ..Default::default()
        }),
        ..Default::default()
    };
    doc.validate().unwrap();
}

#[test]
fn custom_catalog_ids_keep_their_existing_meaning() {
    let mut doc = RegistryDocument {
        catalog: Some(TopologyCatalog {
            routers: vec![router("existing-router".into())],
            ..Default::default()
        }),
        ..Default::default()
    };
    let before = doc.catalog.clone();
    reserve_ids(&mut doc, 0, &[IdKind::Source]).unwrap();
    assert_eq!(doc.catalog, before);
}

#[test]
fn restart_and_deleted_definitions_do_not_reuse_generated_ids() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(dir.path()).unwrap();
    let mut draft = store.document().unwrap().clone();
    let first = reserve_ids(&mut draft, 0, &[IdKind::Router, IdKind::Node]).unwrap();
    draft.catalog = Some(TopologyCatalog {
        routers: vec![router(first[0].clone())],
        ..Default::default()
    });
    store.commit(0, draft).unwrap();
    drop(store);
    let mut store = RegistryStore::open(dir.path()).unwrap();
    let mut draft = store.document().unwrap().clone();
    draft.catalog.as_mut().unwrap().routers.clear();
    let next = reserve_ids(&mut draft, 1, &[IdKind::Router]).unwrap();
    assert_eq!(next[0], "tm-router-0000000000000003");
    draft
        .catalog
        .as_mut()
        .unwrap()
        .routers
        .push(router(next[0].clone()));
    store.commit(1, draft).unwrap();
    drop(store);
    let store = RegistryStore::open(dir.path()).unwrap();
    assert_eq!(store.document().unwrap().next_id, 4);
}

#[test]
fn subscriber_allocation_skips_catalog_reservations() {
    use crate::topology_manager::model::{SourceKind, SourceScope};
    use crate::topology_manager::observation::{
        AccountObservation, Endpoint, ObservationGate, SourceRead, reconcile,
    };
    let mut doc = RegistryDocument::default();
    reserve_ids(&mut doc, 0, &[IdKind::Node, IdKind::Source]).unwrap();
    let scope = SourceScope {
        router_id: "router".into(),
        source_id: "service".into(),
        kind: SourceKind::Ppp,
    };
    let snapshot = ObservationGate::default()
        .accept(
            1,
            std::slice::from_ref(&scope),
            vec![SourceRead::Complete {
                scope: scope.clone(),
                accounts: vec![AccountObservation {
                    account_ref: "verified-account".into(),
                    policy: Default::default(),
                    endpoints: vec![Endpoint {
                        session_id: "session".into(),
                        ips: vec!["192.0.2.1".parse().unwrap()],
                        mac: None,
                    }],
                }],
            }],
        )
        .unwrap();
    let result = reconcile(&doc, snapshot).unwrap();
    assert_eq!(result.active[0].subscriber_id, "tm-sub-0000000000000003");
}

#[test]
fn a_large_id_batch_uses_one_checked_reservation() {
    let mut doc = RegistryDocument::default();
    let ids = reserve_ids(&mut doc, 0, &vec![IdKind::Node; 10_000]).unwrap();
    assert_eq!(ids.len(), 10_000);
    assert_eq!(doc.next_id, 10_001);
    assert_eq!(ids.last().unwrap(), "tm-node-0000000000002710");
    assert_eq!(
        ids.iter().collect::<std::collections::HashSet<_>>().len(),
        10_000
    );
}
