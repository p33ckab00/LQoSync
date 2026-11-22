use super::*;
use crate::topology_manager::{catalog, observation::*, rates::*};
use std::time::Duration;

pub(crate) fn input(count: u32) -> AuthorizedReconciliation {
    let mut registry = RegistryDocument {
        catalog: Some(catalog::tests::catalog()),
        ..Default::default()
    };
    let catalog = registry.catalog.as_mut().unwrap();
    if let catalog::SourceOrigin::Subscribers { group, .. } = &mut catalog.sources[2].origin {
        group.backstop = None;
    }
    let scope = catalog.required_scopes().unwrap()[0].clone();
    let accounts = (0..count)
        .map(|i| AccountObservation {
            account_ref: format!("account-{i}"),
            endpoints: vec![Endpoint {
                session_id: format!("session-{i}"),
                ips: vec![std::net::IpAddr::V4((0x0a000001 + i).into())],
                mac: None,
            }],
            policy: ObservedPolicy {
                profile: ProfileObservation::Known(Some("provider-profile-id".into())),
                plan: Some(RateBps {
                    down: 50_000_000,
                    up: 10_000_000,
                }),
            },
        })
        .collect();
    let snapshot = ObservationGate::default()
        .accept(
            1,
            std::slice::from_ref(&scope),
            vec![SourceRead::Complete {
                scope: scope.clone(),
                accounts,
            }],
        )
        .unwrap();
    AuthorizedReconciliation {
        reconciliation: reconcile(&registry, snapshot).unwrap(),
        expires: Instant::now() + Duration::from_secs(10),
        router_locations: HashMap::new(),
    }
}

#[tokio::test(start_paused = true)]
async fn fresh_pair_keeps_subscriber_rates_node_ids_and_multiple_wan_roots() {
    let pair = prepare(input(4)).unwrap();
    assert_eq!(pair.rows().len(), 4);
    assert_eq!(pair.network().as_object().unwrap().len(), 2);
    let keys: Vec<_> = pair.network().as_object().unwrap().keys().collect();
    assert!(keys.iter().any(|k| k.contains("wan1")));
    assert!(keys.iter().any(|k| k.contains("wan2")));
    assert!(!pair.network().as_object().unwrap().contains_key("Topology"));
    for row in pair.rows() {
        assert_eq!(row.download_max, "50");
        assert_eq!(row.upload_max, "10");
        assert!(!row.parent_node_id.is_empty());
        assert!(row.device_id.starts_with("tm-device-"));
    }
    assert!(pair.csv().starts_with("Circuit ID,Circuit Name,Device ID"));
    assert_eq!(crate::state::read_csv(pair.csv()).unwrap(), pair.rows());
}

#[tokio::test(start_paused = true)]
async fn inactive_subscriber_intent_does_not_export_old_endpoints_or_groups() {
    let pair = prepare(input(0)).unwrap();
    assert!(pair.rows().is_empty());
    assert_eq!(pair.network().as_object().unwrap().len(), 2);
    for wan in pair.network().as_object().unwrap().values() {
        assert_eq!(wan["children"], serde_json::json!({}));
    }
}

#[tokio::test(start_paused = true)]
async fn expired_observations_cannot_prepare_a_pair() {
    let mut observed = input(1);
    observed.expires = Instant::now();
    assert!(prepare(observed).is_err());
}

#[tokio::test(start_paused = true)]
async fn import_precision_loss_blocks_instead_of_rounding_a_subscriber_maximum() {
    let mut observed = input(1);
    observed
        .reconciliation
        .registry
        .catalog
        .as_mut()
        .unwrap()
        .profiles[0]
        .base = catalog::BaseRateAuthority::Local(RateBps {
        down: 50_000_001,
        up: 10_000_000,
    });
    assert!(prepare(observed).is_err());
}

#[tokio::test(start_paused = true)]
async fn multiple_current_sessions_export_one_circuit_with_distinct_device_ids() {
    let mut observed = input(1);
    let first = &observed.reconciliation.active[0].endpoints[0];
    let mut endpoint = ActiveEndpoint {
        identity: first.identity.clone(),
        endpoint: first.endpoint.clone(),
    };
    endpoint.endpoint.session_id = "second-session".into();
    endpoint.endpoint.ips = vec!["2001:db8::1".parse().unwrap()];
    observed.reconciliation.active[0].endpoints.push(endpoint);
    let pair = prepare(observed).unwrap();
    assert_eq!(pair.rows().len(), 2);
    assert_eq!(pair.rows()[0].circuit_id, pair.rows()[1].circuit_id);
    assert_ne!(pair.rows()[0].device_id, pair.rows()[1].device_id);
}

fn metadata() -> crate::write_receipt::Meta {
    crate::write_receipt::Meta {
        topology: None,
        cycle_seq: 1,
        config_digest: "config".into(),
        notice: None,
    }
}
fn seeded_store(root: &std::path::Path) -> crate::topology_manager::store::RegistryStore {
    let mut store = crate::topology_manager::store::RegistryStore::open(root).unwrap();
    let baseline = RegistryDocument {
        catalog: input(0).reconciliation.registry.catalog,
        ..Default::default()
    };
    store.commit(0, baseline).unwrap();
    store
}

#[tokio::test(start_paused = true)]
async fn registry_precedes_network_first_receipt_and_unchanged_content_keeps_write_identity() {
    let root = tempfile::tempdir().unwrap();
    let live = root.path().join("live");
    let hub = crate::system_health::Hub::default();
    let mut store = seeded_store(root.path());
    let mut tracker = crate::write_receipt::Tracker::load(root.path()).unwrap();
    for generation in 1..=2 {
        let mut observed = input(1);
        observed.reconciliation.generation = generation;
        if generation == 1 {
            observed.reconciliation.registry.revision = 1;
        } else {
            observed.reconciliation.registry = store.document().unwrap().clone();
        }
        let pair = admitted(prepare(observed).unwrap());
        let before = tracker.current().map(|r| r.write_id.clone());
        let result = pair
            .publish(
                &mut store,
                &mut tracker,
                PublishRequest {
                    directory: &live,
                    meta: metadata(),
                    now: generation,
                    hub: &hub,
                },
            )
            .unwrap();
        assert!(result.error.is_none());
        assert_eq!(result.changed, generation == 1);
        let receipt = tracker.current().unwrap();
        assert_eq!(receipt.files[0].path.file_name().unwrap(), "network.json");
        assert_eq!(
            receipt.files[1].path.file_name().unwrap(),
            "ShapedDevices.csv"
        );
        let context = receipt.meta.topology.as_ref().unwrap();
        assert_eq!(context.policy_revision, 2);
        assert_eq!(context.observation_generation, 1);
        assert_eq!(
            context.registry_digest,
            crate::write_receipt::sha256(&serde_json::to_vec(store.document().unwrap()).unwrap())
        );
        if let Some(before) = before {
            assert_eq!(receipt.write_id, before);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn stale_registry_and_expired_pair_leave_shaping_files_untouched() {
    for expired in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let live = root.path().join("live");
        let hub = crate::system_health::Hub::default();
        let mut store = seeded_store(root.path());
        let mut tracker = crate::write_receipt::Tracker::load(root.path()).unwrap();
        let mut observed = input(1);
        observed.reconciliation.registry.revision = 1;
        let pair = admitted(prepare(observed).unwrap());
        if expired {
            tokio::time::advance(Duration::from_secs(11)).await;
        } else {
            let mut changed = store.document().unwrap().clone();
            changed.catalog.as_mut().unwrap().root_label = "Updated policy".into();
            store.commit(1, changed).unwrap();
        }
        assert!(
            pair.publish(
                &mut store,
                &mut tracker,
                PublishRequest {
                    directory: &live,
                    meta: metadata(),
                    now: 10,
                    hub: &hub
                }
            )
            .is_err()
        );
        assert!(!live.join("network.json").exists());
        assert!(!live.join("ShapedDevices.csv").exists());
        assert!(tracker.current().is_none());
    }
}

#[tokio::test(start_paused = true)]
async fn fractional_rates_survive_csv_export_without_the_legacy_three_decimal_rounding() {
    let mut observed = input(1);
    observed
        .reconciliation
        .registry
        .catalog
        .as_mut()
        .unwrap()
        .profiles[0]
        .base = catalog::BaseRateAuthority::Local(RateBps {
        down: 1_234_567,
        up: 2_500_000,
    });
    if let catalog::SourceOrigin::Subscribers { group, .. } = &mut observed
        .reconciliation
        .registry
        .catalog
        .as_mut()
        .unwrap()
        .sources[2]
        .origin
    {
        group.factors = catalog::Factors::default();
    }
    let pair = prepare(observed).unwrap();
    assert_eq!(pair.rows()[0].download_max, "1.234567");
    assert_eq!(pair.rows()[0].upload_max, "2.5");
}

#[tokio::test(start_paused = true)]
async fn intent_storage_failure_keeps_the_registry_but_never_replaces_shaping_files() {
    let root = tempfile::tempdir().unwrap();
    let mut store = seeded_store(root.path());
    let mut tracker = crate::write_receipt::Tracker::load(root.path()).unwrap();
    std::fs::create_dir(root.path().join("write-receipt.json")).unwrap();
    let live = root.path().join("live");
    let mut observed = input(1);
    observed.reconciliation.registry.revision = 1;
    let pair = admitted(prepare(observed).unwrap());
    let hub = crate::system_health::Hub::default();
    let result = pair
        .publish(
            &mut store,
            &mut tracker,
            PublishRequest {
                directory: &live,
                meta: metadata(),
                now: 10,
                hub: &hub,
            },
        )
        .unwrap();
    assert!(result.error.is_some());
    assert_eq!(store.document().unwrap().subscribers.len(), 1);
    assert!(!live.join("network.json").exists());
    assert!(!live.join("ShapedDevices.csv").exists());
}

#[tokio::test(start_paused = true)]
async fn exported_child_ceilings_stay_within_wan_capacity_without_clamping_subscriber_maxima() {
    let mut observed = input(20);
    if let catalog::SourceOrigin::Subscribers { group, .. } = &mut observed
        .reconciliation
        .registry
        .catalog
        .as_mut()
        .unwrap()
        .sources[2]
        .origin
    {
        group.factors = catalog::Factors {
            down: Fraction {
                numerator: 2,
                denominator: 1,
            },
            up: Fraction {
                numerator: 2,
                denominator: 1,
            },
        };
    }
    let pair = prepare(observed).unwrap();
    for wan in pair.network().as_object().unwrap().values() {
        for child in wan["children"].as_object().unwrap().values() {
            assert!(
                child["downloadBandwidthMbps"].as_f64().unwrap()
                    <= wan["downloadBandwidthMbps"].as_f64().unwrap()
            );
            assert!(
                child["uploadBandwidthMbps"].as_f64().unwrap()
                    <= wan["uploadBandwidthMbps"].as_f64().unwrap()
            );
        }
    }
    assert!(
        pair.rows()
            .iter()
            .all(|row| row.download_max == "50" && row.upload_max == "10")
    );
}

#[tokio::test(start_paused = true)]
async fn excessive_hierarchy_depth_is_a_conflict_before_recursive_json_work() {
    let mut observed = input(1);
    let catalog = observed.reconciliation.registry.catalog.as_mut().unwrap();
    let mut parent = "wan1".to_string();
    for i in 0..64 {
        let id = format!("manual-{i}");
        catalog.nodes.push(catalog::CatalogNode {
            router_id: None,
            geo: None,
            minimum: Default::default(),
            id: id.clone(),
            name: id.clone(),
            parent: Some(parent),
            source_id: None,
            kind: crate::topology_manager::layout::NodeKind::Intermediate,
            capacity: catalog::CapacityPolicy::Inherit,
        });
        parent = id;
    }
    catalog
        .nodes
        .iter_mut()
        .find(|node| node.id == "group1")
        .unwrap()
        .parent = Some(parent);
    let error = prepare(observed).err().unwrap();
    assert!(error.to_string().contains("depth limit"));
}

fn admitted(pair: PreparedPair) -> super::super::admission::AdmittedPair {
    let (canonical, shaping) = super::super::admission::tests::artifacts(&pair);
    super::super::admission::check_compiled(pair, &canonical, &shaping).unwrap()
}

#[tokio::test(start_paused = true)]
async fn fractional_node_ceiling_uses_supported_whole_mbps_without_rounding_up() {
    let mut observed = input(1);
    let catalog = observed.reconciliation.registry.catalog.as_mut().unwrap();
    catalog.sources[2].targets = vec!["group1".into()];
    catalog
        .nodes
        .iter_mut()
        .find(|n| n.id == "group1")
        .unwrap()
        .capacity = catalog::CapacityPolicy::Fixed {
        base: RateBps {
            down: 100_500_000,
            up: 10_500_000,
        },
        factors: catalog::FactorChoice::Override(catalog::Factors::default()),
    };
    let pair = prepare(observed).unwrap();
    let group = pair
        .network()
        .as_object()
        .unwrap()
        .values()
        .flat_map(|wan| wan["children"].as_object().unwrap().values())
        .find(|node| node["id"] == "group1")
        .unwrap();
    assert_eq!(group["downloadBandwidthMbps"], 100.0);
    assert_eq!(group["uploadBandwidthMbps"], 10.0);
    assert_eq!(pair.rows()[0].download_max, "50");
}

#[tokio::test(start_paused = true)]
async fn export_quantization_cannot_erase_a_subscriber_minimum_commitment() {
    let mut observed = input(1);
    let catalog = observed.reconciliation.registry.catalog.as_mut().unwrap();
    catalog.sources[2].targets = vec!["group1".into()];
    catalog.defaults.down_min = RateRule::FixedBps(1_100_000);
    catalog
        .nodes
        .iter_mut()
        .find(|n| n.id == "group1")
        .unwrap()
        .capacity = catalog::CapacityPolicy::Fixed {
        base: RateBps {
            down: 1_500_000,
            up: 10_000_000,
        },
        factors: catalog::FactorChoice::Override(catalog::Factors::default()),
    };
    assert!(prepare(observed).is_err());
}

#[tokio::test(start_paused = true)]
async fn export_adjustments_are_visible_and_cannot_undercut_a_configured_floor() {
    let mut observed = input(1);
    let catalog = observed.reconciliation.registry.catalog.as_mut().unwrap();
    catalog.sources[2].targets = vec!["group1".into()];
    if let catalog::SourceOrigin::Subscribers { group, .. } = &mut catalog.sources[2].origin {
        group.floor = Some(RateBps {
            down: 100_500_000,
            up: 10_000_000,
        });
    }
    assert!(prepare(observed).is_err());
    let mut observed = input(1);
    let catalog = observed.reconciliation.registry.catalog.as_mut().unwrap();
    catalog.sources[2].targets = vec!["group1".into()];
    catalog
        .nodes
        .iter_mut()
        .find(|node| node.id == "group1")
        .unwrap()
        .capacity = catalog::CapacityPolicy::Fixed {
        base: RateBps {
            down: 100_500_000,
            up: 10_000_000,
        },
        factors: catalog::FactorChoice::Override(catalog::Factors::default()),
    };
    let pair = prepare(observed).unwrap();
    let adjustment = pair
        .node_export_adjustments()
        .iter()
        .find(|a| a.node_id == "group1")
        .unwrap();
    assert_eq!(adjustment.calculated_ceiling.down, 100_500_000);
    assert_eq!(adjustment.exported_ceiling.down, 100_000_000);
}

#[tokio::test(start_paused = true)]
async fn explicit_minimums_export_only_the_declared_direction_and_preserve_ceiling() {
    let mut observed = input(2);
    for node in &mut observed
        .reconciliation
        .registry
        .catalog
        .as_mut()
        .unwrap()
        .nodes
    {
        if node.source_id.as_deref() == Some("profile-source") {
            let mut value = serde_json::to_value(&*node).unwrap();
            value["minimum"] = serde_json::json!({"down":10_000_000});
            *node = serde_json::from_value(value).unwrap();
        }
    }
    let pair = prepare(observed).unwrap();
    let mut groups = 0;
    for wan in pair.network().as_object().unwrap().values() {
        for group in wan["children"].as_object().unwrap().values() {
            groups += 1;
            assert_eq!(group["downloadBandwidthMbpsMin"], 10);
            assert!(group.get("uploadBandwidthMbpsMin").is_none());
            assert!(group["downloadBandwidthMbps"].as_f64().unwrap() >= 10.0);
        }
    }
    assert!(groups > 0);
    let (native, alerts) =
        crate::writer::v3network::render_libreqos_network(&[], pair.network()).unwrap();
    assert!(alerts.is_empty());
    let native: serde_json::Value = serde_json::from_str(&native).unwrap();
    for node in native["topology"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| {
            node["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("group"))
        })
    {
        assert_eq!(node["rate"]["commit_down_mbps"], 10.0);
        assert_eq!(node["rate"]["commit_up_mbps"], node["rate"]["ceil_up_mbps"]);
    }
    assert!(pair.rows().iter().all(|r| r.download_max == "50"));
}

#[tokio::test(start_paused = true)]
async fn quantized_parent_ceiling_must_cover_child_guarantees_and_subscriber_minima() {
    let mut observed = input(1);
    let c = observed.reconciliation.registry.catalog.as_mut().unwrap();
    c.defaults.down_min = RateRule::FixedBps(600_000);
    c.defaults.up_min = RateRule::FixedBps(100_000);
    c.sources[2].targets = vec!["group2".into()];
    c.nodes[0].capacity = catalog::CapacityPolicy::Fixed {
        base: RateBps {
            down: 10_900_000,
            up: 100_000_000,
        },
        factors: catalog::FactorChoice::Override(catalog::Factors::default()),
    };
    c.nodes[2].source_id = None;
    c.nodes[2].minimum.down = Some(10_000_000);
    c.nodes[2].capacity = catalog::CapacityPolicy::Fixed {
        base: RateBps {
            down: 10_000_000,
            up: 5_000_000,
        },
        factors: catalog::FactorChoice::Override(catalog::Factors::default()),
    };
    c.nodes[3].parent = Some("wan1".into());
    c.nodes[3].capacity = catalog::CapacityPolicy::Fixed {
        base: RateBps {
            down: 1_000_000,
            up: 5_000_000,
        },
        factors: catalog::FactorChoice::Override(catalog::Factors::default()),
    };
    let error = prepare(observed).err().unwrap();
    assert!(
        error
            .to_string()
            .contains("quantization violates minimum commitments"),
        "{error:#}"
    );
}

#[tokio::test(start_paused = true)]
async fn static_revision_conflicts_block_before_registry_or_output_changes() {
    use crate::topology_manager::{ownership::ExportOwnership, static_input::CurrentStatic};
    let root = tempfile::tempdir().unwrap();
    let live = root.path().join("live");
    let hub = crate::system_health::Hub::default();
    let mut store = seeded_store(root.path());
    let mut tracker = crate::write_receipt::Tracker::load(root.path()).unwrap();
    let source_path = root.path().join("topology-manager/static-input.json");
    for (generation, revision, capacity, succeeds) in [
        (1, 2, 20, true),
        (2, 1, 30, false),
        (3, 2, 30, false),
        (4, 3, 30, true),
        (5, 4, 40, false),
    ] {
        let document = serde_json::json!({"schema_version":1,"revision":revision,"rows":[],
            "network":{"Static root":{"id":"static-root","type":"site",
                "downloadBandwidthMbps":capacity,"uploadBandwidthMbps":20,"children":{}}}});
        crate::files::write_secret(&source_path, &serde_json::to_vec(&document).unwrap()).unwrap();
        let source = CurrentStatic::load(root.path()).unwrap();
        let mut observed = input(0);
        observed.reconciliation.registry = store.document().unwrap().clone();
        observed.reconciliation.generation = generation;
        observed.reconciliation.registry.export_ownership = Some(ExportOwnership {
            protected_roots: vec!["static-root".into()],
            ..Default::default()
        });
        let mut pair = prepare_with_static(observed, Some(source.input())).unwrap();
        pair.static_source = Some(source.binding);
        let pair = admitted(pair);
        if generation == 5 {
            let mut changed = document.clone();
            changed["revision"] = serde_json::json!(5);
            crate::files::write_secret(&source_path, &serde_json::to_vec(&changed).unwrap())
                .unwrap();
        }
        let before = store.document().unwrap().revision;
        let old_output = std::fs::read(live.join("network.json")).ok();
        let result = pair.publish(
            &mut store,
            &mut tracker,
            PublishRequest {
                directory: &live,
                meta: metadata(),
                now: generation,
                hub: &hub,
            },
        );
        assert_eq!(result.is_ok(), succeeds);
        if succeeds {
            assert!(result.unwrap().error.is_none());
            assert_eq!(
                tracker
                    .current()
                    .unwrap()
                    .meta
                    .topology
                    .as_ref()
                    .unwrap()
                    .static_source
                    .as_ref()
                    .unwrap()
                    .revision,
                revision
            );
        } else {
            assert_eq!(store.document().unwrap().revision, before);
            assert_eq!(std::fs::read(live.join("network.json")).ok(), old_output);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn unbound_static_preview_cannot_publish_or_commit_registry_intent() {
    use crate::topology_manager::ownership::{ExportOwnership, StaticInput};
    let root = tempfile::tempdir().unwrap();
    let mut store = seeded_store(root.path());
    let mut tracker = crate::write_receipt::Tracker::load(root.path()).unwrap();
    let mut observed = input(0);
    observed.reconciliation.registry = store.document().unwrap().clone();
    observed.reconciliation.registry.export_ownership = Some(ExportOwnership {
        protected_roots: vec!["static-root".into()],
        ..Default::default()
    });
    let network = serde_json::json!({"Static root":{"id":"static-root","type":"site",
        "downloadBandwidthMbps":20,"uploadBandwidthMbps":20,"children":{}}});
    let pair = prepare_with_static(
        observed,
        Some(StaticInput {
            rows: &[],
            network: &network,
        }),
    )
    .unwrap();
    let pair = admitted(pair);
    let before = store.document().unwrap().revision;
    assert!(
        pair.publish(
            &mut store,
            &mut tracker,
            PublishRequest {
                directory: &root.path().join("live"),
                meta: metadata(),
                now: 1,
                hub: &crate::system_health::Hub::default()
            }
        )
        .is_err()
    );
    assert_eq!(store.document().unwrap().revision, before);
    assert!(tracker.current().is_none());
    assert!(!root.path().join("live/network.json").exists());
}

#[tokio::test(start_paused = true)]
async fn router_location_goes_only_on_the_nodes_from_that_router() {
    let mut observed = input(2);
    let catalog = observed.reconciliation.registry.catalog.as_mut().unwrap();
    catalog.nodes.push(catalog::CatalogNode {
        router_id: None,
        geo: None,
        minimum: Default::default(),
        id: "manual".into(),
        name: "manual".into(),
        parent: Some("wan1".into()),
        source_id: None,
        kind: crate::topology_manager::layout::NodeKind::Intermediate,
        capacity: catalog::CapacityPolicy::Inherit,
    });
    catalog
        .nodes
        .iter_mut()
        .find(|node| node.id == "group1")
        .unwrap()
        .parent = Some("manual".into());
    // Distro has a location. Core (the WAN router) has none.
    observed.router_locations = [("Distro".to_string(), (14.5995, 120.9842))].into();
    let pair = prepare(observed).unwrap();
    let mut nodes = HashMap::new();
    let mut work: Vec<&Value> = pair.network().as_object().unwrap().values().collect();
    while let Some(node) = work.pop() {
        nodes.insert(node["id"].as_str().unwrap().to_string(), node.clone());
        work.extend(node["children"].as_object().unwrap().values());
    }
    for id in ["group1", "group2"] {
        assert_eq!(
            (
                nodes[id]["latitude"].as_f64(),
                nodes[id]["longitude"].as_f64()
            ),
            (Some(14.5995), Some(120.9842)),
            "{id}"
        );
    }
    for id in ["wan1", "wan2", "manual"] {
        assert!(
            nodes[id].get("latitude").is_none() && nodes[id].get("longitude").is_none(),
            "{id}"
        );
    }
    // The controlled compiler admission accepts a pair whose nodes carry a location.
    admitted(pair);
}

#[tokio::test(start_paused = true)]
async fn explicit_node_type_and_full_coordinates_win_over_the_router_location() {
    use web_contract::topology::{GeoType, Location, NodeGeo};
    let mut observed = input(2);
    let catalog = observed.reconciliation.registry.catalog.as_mut().unwrap();
    let geo = |geo_type, lat: &str, lon: &str| {
        Some(NodeGeo {
            geo_type,
            location: Some(Location {
                latitude: lat.into(),
                longitude: lon.into(),
            }),
        })
    };
    fn node<'a>(c: &'a mut catalog::TopologyCatalog, id: &str) -> &'a mut Option<NodeGeo> {
        &mut c.nodes.iter_mut().find(|n| n.id == id).unwrap().geo
    }
    *node(catalog, "group1") = geo(GeoType::Ap, "12.236199459585354", "122.02419129509887");
    *node(catalog, "wan1") = geo(GeoType::Site, "-33.8688197", "151.20929550000002");
    // A type without a location keeps the router fallback location.
    *node(catalog, "group2") = Some(NodeGeo {
        geo_type: GeoType::Ap,
        location: None,
    });
    observed.router_locations = [("Distro".to_string(), (14.5995, 120.9842))].into();
    let pair = prepare(observed).unwrap();
    let text = pair.network_text();
    assert!(
        text.contains("12.236199459585354") && text.contains("122.02419129509887"),
        "{text}"
    );
    assert!(text.contains("151.20929550000002"), "{text}");
    let mut nodes = HashMap::new();
    let mut work: Vec<&Value> = pair.network().as_object().unwrap().values().collect();
    while let Some(node) = work.pop() {
        nodes.insert(node["id"].as_str().unwrap().to_string(), node.clone());
        work.extend(node["children"].as_object().unwrap().values());
    }
    assert_eq!(nodes["group1"]["type"], "ap");
    assert_eq!(
        nodes["group1"]["latitude"].as_f64(),
        Some(12.236199459585354)
    );
    assert_eq!(nodes["wan1"]["type"], "site");
    assert_eq!(
        nodes["wan1"]["longitude"].as_f64(),
        Some(151.20929550000002)
    );
    assert_eq!(nodes["group2"]["type"], "ap");
    assert_eq!(nodes["group2"]["latitude"].as_f64(), Some(14.5995));
    assert_eq!(nodes["wan2"]["type"], "site");
    admitted(pair);
}

#[test]
fn flat_pair_has_empty_parent_and_an_empty_network() {
    let mut input = input(2);
    let c = input.reconciliation.registry.catalog.as_mut().unwrap();
    c.nodes.clear();
    for source in &mut c.sources {
        source.targets.clear();
    }
    let pair = prepare(input).unwrap();
    assert_eq!(pair.network(), &serde_json::json!({}));
    assert_eq!(pair.rows().len(), 2);
    assert!(
        pair.rows()
            .iter()
            .all(|r| r.parent_node.is_empty() && r.parent_node_id.is_empty())
    );
}
