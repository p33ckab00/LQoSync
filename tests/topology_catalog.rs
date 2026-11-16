//! Public persistence proof for one catalog/registry revision.

use lqosync3::topology_manager::catalog::*;
use lqosync3::topology_manager::layout::NodeKind;
use lqosync3::topology_manager::model::{
    IdentityRef, RegistryDocument, SourceKind, SourceScope, SubscriberPolicy, SubscriberRecord,
};
use lqosync3::topology_manager::rates::{Fraction, RateBps, SpeedOverrides};
use lqosync3::topology_manager::store::RegistryStore;
use std::os::unix::fs::MetadataExt;

#[test]
fn public_scoped_action_keeps_pins_and_persists_through_the_revision_checked_store() {
    use lqosync3::topology_manager::observation::{
        AccountObservation, Endpoint, ObservationGate, ObservedPolicy, ProfileObservation,
        Reconciliation, SourceRead, reconcile,
    };
    use lqosync3::topology_manager::rebalance::{RebalanceRequest, RebalanceScope, rebalance};

    fn collection(registry: &RegistryDocument) -> Reconciliation {
        let accounts = (0..60)
            .map(|i| AccountObservation {
                account_ref: if i == 0 {
                    "account-id".into()
                } else {
                    format!("account-{i}")
                },
                policy: ObservedPolicy {
                    profile: ProfileObservation::Known(Some("provider-profile".into())),
                    plan: None,
                },
                endpoints: vec![Endpoint {
                    session_id: format!("session-{i}"),
                    ips: vec![format!("192.0.2.{}", i + 1).parse().unwrap()],
                    mac: None,
                }],
            })
            .collect();
        let snapshot = ObservationGate::default()
            .accept(
                1,
                &[scope()],
                vec![SourceRead::Complete {
                    scope: scope(),
                    accounts,
                }],
            )
            .unwrap();
        reconcile(registry, snapshot).unwrap()
    }
    let mut seeded = collection(&document()).registry;
    for subscriber in &mut seeded.subscribers {
        subscriber.policy.assignment = Some("g1".into());
    }
    let dir = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(dir.path()).unwrap();
    store.commit(0, seeded).unwrap();
    let result = rebalance(
        collection(store.document().unwrap()),
        RebalanceRequest {
            expected_revision: 1,
            scope: RebalanceScope::SourcePool("subscribers".into()),
        },
    )
    .unwrap();
    assert!(!result.proposed_moves.is_empty());
    assert!(
        !result
            .proposed_moves
            .iter()
            .any(|m| m.subscriber_id == "tm-sub-0000000000000001")
    );
    assert_eq!(store.document().unwrap().revision, 1);
    store
        .commit(1, result.evaluation.candidate.unwrap())
        .unwrap();
    drop(store);
    let store = RegistryStore::open(dir.path()).unwrap();
    let saved = store.document().unwrap();
    assert_eq!(saved.revision, 2);
    let pinned = saved
        .subscribers
        .iter()
        .find(|s| s.id == "tm-sub-0000000000000001")
        .unwrap();
    assert_eq!(pinned.policy.pin.as_deref(), Some("g1"));
    assert_eq!(pinned.policy.assignment, pinned.policy.pin);
    let bytes = std::fs::read_to_string(dir.path().join("topology-manager/registry.json")).unwrap();
    assert!(!bytes.contains("session-"));
    assert!(!bytes.contains("192.0.2."));
    assert!(
        rebalance(
            collection(saved),
            RebalanceRequest {
                expected_revision: 1,
                scope: RebalanceScope::SourcePool("subscribers".into()),
            }
        )
        .is_err()
    );
}

fn rate(down: u64, up: u64) -> RateBps {
    RateBps { down, up }
}
fn fraction(numerator: u64, denominator: u64) -> Fraction {
    Fraction {
        numerator,
        denominator,
    }
}
fn scope() -> SourceScope {
    SourceScope {
        router_id: "distro".into(),
        source_id: "subscriber-reader".into(),
        kind: SourceKind::Ppp,
    }
}

fn document() -> RegistryDocument {
    let source = |id: &str, endpoint: &str| CatalogSource {
        id: id.into(),
        name: "WAN candidate".into(),
        origin: SourceOrigin::Wan {
            router_id: "core".into(),
            endpoint_ref: endpoint.into(),
        },
        profile_id: None,
        targets: Vec::new(),
    };
    let wan = |id: &str, source: &str, down| CatalogNode {
        id: id.into(),
        name: id.into(),
        parent: None,
        source_id: Some(source.into()),
        kind: NodeKind::Wan,
        capacity: CapacityPolicy::Fixed {
            base: rate(down, 100_000_000),
            factors: FactorChoice::Override(Factors {
                down: fraction(9, 10),
                up: fraction(1, 1),
            }),
        },
    };
    let group = |id: &str, parent: &str| CatalogNode {
        id: id.into(),
        name: "Shared group".into(),
        parent: Some(parent.into()),
        source_id: Some("subscribers".into()),
        kind: NodeKind::SubscriberGroup,
        capacity: CapacityPolicy::Automatic {
            factors: FactorChoice::Inherit,
        },
    };
    RegistryDocument {
        next_id: 2,
        subscribers: vec![SubscriberRecord {
            id: "tm-sub-0000000000000001".into(),
            identities: vec![IdentityRef {
                scope: scope(),
                account_ref: "account-id".into(),
            }],
            policy: SubscriberPolicy {
                profile_id: Some("plan".into()),
                assignment: Some("g1".into()),
                pin: Some("g1".into()),
            },
        }],
        catalog: Some(TopologyCatalog {
            root_label: "My ISP".into(),
            routers: vec![
                CatalogRouter {
                    id: "core".into(),
                    name: "Core".into(),
                    roles: vec![RouterRole::Wan],
                },
                CatalogRouter {
                    id: "distro".into(),
                    name: "Distro".into(),
                    roles: vec![RouterRole::Hotspot, RouterRole::Ppp],
                },
            ],
            sources: vec![
                source("source-w1", "endpoint-1"),
                source("source-w2", "endpoint-2"),
                CatalogSource {
                    id: "subscribers".into(),
                    name: "Shared source".into(),
                    origin: SourceOrigin::Subscribers {
                        scope: scope(),
                        profile_ref: Some("provider-profile".into()),
                        group: GroupDefaults {
                            floor: None,
                            formula: GroupFormula::PlanSum,
                            factors: Factors {
                                down: fraction(4, 5),
                                up: fraction(1, 2),
                            },
                            backstop: Some(BackstopPolicy {
                                base: rate(500_000_000, 100_000_000),
                                factor: fraction(9, 10),
                            }),
                        },
                    },
                    profile_id: Some("plan".into()),
                    targets: vec!["g1".into(), "g2".into()],
                },
            ],
            profiles: vec![SubscriberProfile {
                id: "plan".into(),
                name: "Local plan".into(),
                base: BaseRateAuthority::Local(rate(50_000_000, 10_000_000)),
                speed: SpeedOverrides::default(),
                eligible_targets: None,
            }],
            nodes: vec![
                wan("w1", "source-w1", 310_000_000),
                wan("w2", "source-w2", 620_000_000),
                group("g1", "w1"),
                group("g2", "w2"),
            ],
            ..TopologyCatalog::default()
        }),
        ..RegistryDocument::default()
    }
}

#[test]
fn reopen_move_and_discovery_keep_ids_pins_and_explicit_target_sets() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(dir.path()).unwrap();
    store.commit(0, document()).unwrap();
    let expected = store.document().unwrap().clone();
    drop(store);
    let mut store = RegistryStore::open(dir.path()).unwrap();
    assert_eq!(store.document().unwrap(), &expected);
    let catalog = store.document().unwrap().catalog.as_ref().unwrap();
    assert_eq!(catalog.root_label, "My ISP");
    let wan = catalog.nodes.iter().find(|n| n.id == "w1").unwrap();
    assert!(matches!(
        wan.capacity,
        CapacityPolicy::Fixed {
            base: RateBps {
                down: 310_000_000,
                ..
            },
            factors: FactorChoice::Override(Factors {
                down: Fraction {
                    numerator: 9,
                    denominator: 10
                },
                ..
            })
        }
    ));
    let mut edit = store.document().unwrap().clone();
    let catalog = edit.catalog.as_mut().unwrap();
    catalog.nodes.push(CatalogNode {
        id: "branch".into(),
        name: "Branch".into(),
        parent: Some("w2".into()),
        source_id: None,
        kind: NodeKind::Intermediate,
        capacity: CapacityPolicy::Inherit,
    });
    let group = catalog.nodes.iter_mut().find(|n| n.id == "g1").unwrap();
    group.name = "Renamed group".into();
    group.parent = Some("branch".into());
    store.commit(1, edit).unwrap();
    assert_eq!(
        store.document().unwrap().subscribers[0]
            .policy
            .pin
            .as_deref(),
        Some("g1")
    );
    assert_eq!(
        store.document().unwrap().subscribers[0].id,
        "tm-sub-0000000000000001"
    );
    let mut edit = store.document().unwrap().clone();
    let catalog = edit.catalog.as_mut().unwrap();
    let target_ids = catalog
        .sources
        .iter()
        .find(|s| s.id == "subscribers")
        .unwrap()
        .targets
        .clone();
    let instances = catalog.nodes.clone();
    catalog.sources.push(CatalogSource {
        id: "unselected-wan".into(),
        name: "WAN 3".into(),
        origin: SourceOrigin::Wan {
            router_id: "core".into(),
            endpoint_ref: "endpoint-3".into(),
        },
        profile_id: None,
        targets: Vec::new(),
    });
    store.commit(2, edit).unwrap();
    let catalog = store.document().unwrap().catalog.as_ref().unwrap();
    assert_eq!(
        catalog
            .sources
            .iter()
            .find(|s| s.id == "subscribers")
            .unwrap()
            .targets,
        target_ids
    );
    assert_eq!(catalog.nodes, instances);
    assert_eq!(catalog.required_scopes().unwrap(), vec![scope()]);
    let group = catalog.nodes.iter().find(|n| n.id == "g1").unwrap();
    assert_eq!(
        (group.name.as_str(), group.parent.as_deref()),
        ("Renamed group", Some("branch"))
    );

    let path = dir.path().join("topology-manager/registry.json");
    let bytes = std::fs::read(&path).unwrap();
    let inode = std::fs::metadata(&path).unwrap().ino();
    let mut reordered = store.document().unwrap().clone();
    let catalog = reordered.catalog.as_mut().unwrap();
    catalog.routers.reverse();
    catalog.sources.reverse();
    catalog.profiles.reverse();
    catalog.nodes.reverse();
    for router in &mut catalog.routers {
        router.roles.reverse();
    }
    for source in &mut catalog.sources {
        source.targets.reverse();
    }
    assert!(!store.commit(3, reordered).unwrap().changed);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(std::fs::metadata(&path).unwrap().ino(), inode);
}

#[test]
fn an_orphaning_or_stale_edit_cannot_replace_the_valid_document() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(dir.path()).unwrap();
    store.commit(0, document()).unwrap();
    let path = dir.path().join("topology-manager/registry.json");
    let bytes = std::fs::read(&path).unwrap();
    let mut orphan = store.document().unwrap().clone();
    let catalog = orphan.catalog.as_mut().unwrap();
    catalog.nodes.retain(|node| node.id != "g1");
    catalog
        .sources
        .iter_mut()
        .find(|source| source.id == "subscribers")
        .unwrap()
        .targets
        .retain(|target| target != "g1");
    assert!(store.commit(1, orphan).is_err());
    assert!(store.commit(0, document()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(store.document().unwrap().revision, 1);
}
