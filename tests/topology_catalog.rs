//! Public persistence proof for one catalog/registry revision.

use lqosync3::topology_manager::catalog::*;
use lqosync3::topology_manager::layout::NodeKind;
use lqosync3::topology_manager::model::{
    IdentityRef, RegistryDocument, SourceKind, SourceScope, SubscriberPolicy, SubscriberRecord,
};
use lqosync3::topology_manager::rates::{Fraction, RateBps, SpeedOverrides};
use lqosync3::topology_manager::store::RegistryStore;
use std::os::unix::fs::MetadataExt;

#[tokio::test(start_paused = true)]
async fn fresh_authorized_return_restores_pins_after_restart_and_reaches_the_capacity_pipeline() {
    use lqosync3::config::{RateLimitOrder, UnitlessUnit};
    use lqosync3::model::RosRow;
    use lqosync3::routeros::{ReadError, RouterSession};
    use lqosync3::topology_manager::aggregate::evaluate;
    use lqosync3::topology_manager::identity::{
        AuthorizedReconciliation, ReferenceAuthority, SourceSelection, authorize_configured,
    };
    use lqosync3::topology_manager::normalize::normalize;
    use lqosync3::topology_manager::observation::ObservationGate;
    use lqosync3::topology_manager::reader::{ReadGuard, ReadPlan};
    use lqosync3::topology_manager::resolver::resolve;
    use lqosync3::topology_manager::settings::SourceSettings;
    use lqosync3::topology_manager::source_policy::{ProfileBinding, RateRead, SourcePolicy};
    use lqosync3::topology_manager::source_rate::RateFormat;
    use std::time::Duration;

    fn row(fields: &[(&str, &str)]) -> RosRow {
        fields.iter().map(|&(k, v)| (k.into(), v.into())).collect()
    }
    struct Session {
        online: bool,
        missing_rate: bool,
    }
    impl RouterSession for Session {
        async fn print(&mut self, path: &str, _: &str) -> Result<Vec<RosRow>, ReadError> {
            Ok(match path {
                "/ppp/profile" => vec![row(&[
                    (".id", "*A1"),
                    ("name", "gold"),
                    ("rate-limit", if self.missing_rate { "" } else { "10M/50M" }),
                ])],
                "/ppp/secret" => vec![row(&[
                    (".id", "*secret"),
                    ("name", "renamed-account"),
                    ("profile", "gold"),
                    ("disabled", "false"),
                    ("comment", "[lqosync-id:acct-1]"),
                ])],
                "/ppp/active" if self.online => vec![row(&[
                    (".id", "*current-session"),
                    ("name", "renamed-account"),
                    ("address", "203.0.113.88"),
                    ("caller-id", "AA:BB:CC:DD:EE:88"),
                    ("service", "pppoe"),
                ])],
                _ => vec![],
            })
        }
    }
    async fn observed(
        registry: &RegistryDocument,
        generation: u64,
        online: bool,
        missing_rate: bool,
    ) -> AuthorizedReconciliation {
        let plan = ReadPlan::for_router(registry.catalog.as_ref().unwrap(), "distro").unwrap();
        let read = ReadGuard::new("distro".into())
            .unwrap()
            .read(
                &mut Some(Session {
                    online,
                    missing_rate,
                }),
                &plan,
                generation,
                Duration::from_secs(1),
                Duration::from_secs(10),
            )
            .await
            .unwrap();
        let current = normalize(&read).unwrap();
        authorize_configured(registry, registry.revision, generation, vec![current])
            .unwrap()
            .reconcile(&mut ObservationGate::default())
            .unwrap()
    }
    for read_rates in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut store = RegistryStore::open(dir.path()).unwrap();
        let mut seeded = document();
        if read_rates {
            seeded.catalog.as_mut().unwrap().profiles[0].base = BaseRateAuthority::SourceRead;
        }
        seeded.source_settings = Some(SourceSettings {
            selections: vec![SourceSelection {
                scope: scope(),
                service_names: vec![],
                authority: ReferenceAuthority::OperatorManagedComment,
                policy: SourcePolicy {
                    profiles: vec![ProfileBinding {
                        current_name: "gold".into(),
                        source_ref: "provider-profile".into(),
                        rate: if read_rates {
                            RateRead::RateLimit(RateFormat {
                                order: RateLimitOrder::RouterosRxTx,
                                unitless: UnitlessUnit::Bps,
                            })
                        } else {
                            RateRead::Disabled
                        },
                    }],
                    ..Default::default()
                },
            }],
        });
        seeded.subscribers[0].identities[0].account_ref = "operator-comment/v1/acct-1".into();
        store.commit(0, seeded).unwrap();
        let offline = observed(store.document().unwrap(), 1, false, false).await;
        assert!(offline.reconciliation.active.is_empty());
        assert_eq!(
            offline.reconciliation.registry.subscribers[0]
                .policy
                .pin
                .as_deref(),
            Some("g1")
        );
        let evaluation = evaluate(resolve(offline.reconciliation).unwrap()).unwrap();
        assert!(evaluation.placements.is_empty());
        assert!(
            !store
                .commit(1, evaluation.candidate.unwrap())
                .unwrap()
                .changed
        );
        drop(store);
        let mut store = RegistryStore::open(dir.path()).unwrap();
        let returned = observed(store.document().unwrap(), 2, true, false).await;
        let expires = returned.expires;
        assert_eq!(returned.reconciliation.active.len(), 1);
        assert_eq!(
            returned.reconciliation.active[0].subscriber_id,
            "tm-sub-0000000000000001"
        );
        assert_eq!(
            returned.reconciliation.active[0].endpoints[0].endpoint.ips[0].to_string(),
            "203.0.113.88"
        );
        let evaluation = evaluate(resolve(returned.reconciliation).unwrap()).unwrap();
        assert!(evaluation.conflicts.is_empty());
        assert_eq!(evaluation.placements[0].target_id, "g1");
        assert_eq!(evaluation.placements[0].rates.max.down, 50_000_000);
        assert_eq!(evaluation.placements[0].rates.max.up, 10_000_000);
        assert!(tokio::time::Instant::now() < expires);
        assert!(
            !store
                .commit(1, evaluation.candidate.unwrap())
                .unwrap()
                .changed
        );
        let saved =
            std::fs::read_to_string(dir.path().join("topology-manager/registry.json")).unwrap();
        for current_data in [
            "203.0.113.88",
            "AA:BB:CC:DD:EE:88",
            "renamed-account",
            "current-session",
        ] {
            assert!(!saved.contains(current_data));
        }
        if read_rates {
            let missing = observed(store.document().unwrap(), 3, true, true).await;
            let blocked = resolve(missing.reconciliation).unwrap();
            assert!(blocked.candidate.is_none());
            assert!(!blocked.conflicts.is_empty());
        }
        let fresh = observed(store.document().unwrap(), 4, true, false).await;
        let pair = lqosync3::topology_manager::publication::prepare(fresh).unwrap();
        assert_eq!(pair.rows()[0].download_max, "50");
        assert_eq!(pair.rows()[0].ipv4, "203.0.113.88");
        let live = dir.path().join("isolated-pair");
        let hub = lqosync3::system_health::Hub::default();
        let mut tracker = lqosync3::write_receipt::Tracker::load(dir.path()).unwrap();
        let (canonical, shaping) = compilation_artifacts(&pair);
        let pair =
            lqosync3::topology_manager::admission::check_compiled(pair, &canonical, &shaping)
                .unwrap();
        let written = pair
            .publish(
                &mut store,
                &mut tracker,
                lqosync3::topology_manager::publication::PublishRequest {
                    directory: &live,
                    meta: lqosync3::write_receipt::Meta {
                        topology: None,
                        cycle_seq: 4,
                        config_digest: "isolated-test".into(),
                        notice: None,
                    },
                    now: 10,
                    hub: &hub,
                },
            )
            .unwrap();
        assert!(written.changed && written.error.is_none());
        let receipt = tracker.current().unwrap();
        assert_eq!(
            receipt
                .meta
                .topology
                .as_ref()
                .unwrap()
                .observation_generation,
            4
        );
        assert_eq!(
            receipt.meta.topology.as_ref().unwrap().policy_revision,
            store.document().unwrap().revision
        );
        lqosync3::apply_evidence::prepare(receipt).unwrap();
    }
}

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
        router_id: None,
        geo: None,
        minimum: Default::default(),
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
        router_id: None,
        geo: None,
        minimum: Default::default(),
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
                            allocation: Default::default(),
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
        router_id: None,
        geo: None,
        minimum: Default::default(),
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

fn compilation_artifacts(
    pair: &lqosync3::topology_manager::publication::PreparedPair,
) -> (serde_json::Value, serde_json::Value) {
    let mut nodes = Vec::new();
    let mut work: Vec<_> = pair
        .network()
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, node)| (name, node, None::<(&str, &str)>))
        .collect();
    let mut cursor = 0;
    while cursor < work.len() {
        let (name, node, parent) = work[cursor];
        nodes.push(serde_json::json!({"node_id":node["id"],"node_name":name,"node_kind":node["type"],"can_move":false,"allowed_parents":[],"current_attachment_id":parent.map(|(_,id)|id),"current_attachment_name":parent.map(|(name,_)|name),"current_parent_node_id":parent.map(|(_,id)|id),"current_parent_node_name":parent.map(|(name,_)|name),"rate_input":{"intrinsic_download_mbps":node["downloadBandwidthMbps"],"intrinsic_upload_mbps":node["uploadBandwidthMbps"],"source":"imported_network_json"}}));
        work.extend(
            node["children"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(child, n)| {
                    (
                        child,
                        n,
                        Some((name.as_str(), node["id"].as_str().unwrap())),
                    )
                }),
        );
        cursor += 1;
    }
    let addresses = |text: &str, width: u8| {
        text.split(',')
            .filter(|s| !s.is_empty())
            .map(|s| serde_json::json!([s, width]))
            .collect::<Vec<_>>()
    };
    let devices:Vec<_>=pair.rows().iter().map(|r|serde_json::json!({"circuit_id":r.circuit_id,"circuit_name":r.circuit_name,"device_id":r.device_id,"device_name":r.device_name,"parent_node":r.parent_node,"parent_node_id":r.parent_node_id,"anchor_node_id":null,"mac":r.mac,"ipv4":addresses(&r.ipv4,32),"ipv6":addresses(&r.ipv6,128),"download_min_mbps":r.download_min.parse::<f32>().unwrap(),"upload_min_mbps":r.upload_min.parse::<f32>().unwrap(),"download_max_mbps":r.download_max.parse::<f32>().unwrap(),"upload_max_mbps":r.upload_max.parse::<f32>().unwrap(),"comment":r.comment})).collect();
    (
        serde_json::json!({"schema_version":1,"source":"legacy/manual","ingress_identity":"a".repeat(64),"ingress_kind":"native_integration","nodes":nodes,"compatibility_network_json":pair.network()}),
        serde_json::json!({"schema_version":1,"source":"legacy/manual","compile_mode":"full","ingress_identity":"a".repeat(64),"shaped_devices":devices,"circuit_anchors":{"schema_version":0,"source":"","anchors":[]},"circuit_matches":{}}),
    )
}
