use super::*;
use crate::topology_manager::publication::{prepare_with_static, tests::input};

fn shared_input(
    count: u32,
) -> (
    super::super::identity::AuthorizedReconciliation,
    serde_json::Value,
) {
    let mut observed = input(count);
    for subscriber in &mut observed.reconciliation.registry.subscribers {
        subscriber.policy.pin = Some("group1".into());
    }
    observed.reconciliation.registry.export_ownership = Some(ExportOwnership {
        protected_roots: vec!["static-root".into()],
        shared_attachments: vec![StaticAttachment {
            root_id: "static-root".into(),
            parent_id: "wan1".into(),
        }],
        ..Default::default()
    });
    (
        observed,
        serde_json::json!({"Management":{"id":"static-root","type":"site",
        "downloadBandwidthMbps":20,"uploadBandwidthMbps":10,"children":{}}}),
    )
}

#[tokio::test(start_paused = true)]
async fn explicit_static_attachment_uses_the_managed_parent_alias_without_changing_static_policy() {
    let (mut observed, network) = shared_input(1);
    observed
        .reconciliation
        .registry
        .export_ownership
        .as_mut()
        .unwrap()
        .nodes
        .push(NodeAlias {
            node_id: "wan1".into(),
            id: "legacy-wan".into(),
            name: "WAN A".into(),
        });
    let pair = super::super::publication::prepare_with_static(
        observed,
        Some(StaticInput {
            rows: &[],
            network: &network,
        }),
    )
    .unwrap();
    assert!(pair.network().get("Management").is_none());
    assert_eq!(
        pair.network()["WAN A"]["children"]["Management"],
        network["Management"]
    );
    assert_eq!(pair.network().as_object().unwrap().len(), 2);
    assert_eq!(pair.rows()[0].download_max, "50");
    let (canonical, shaping) = super::super::admission::tests::artifacts(&pair);
    super::super::admission::check_compiled(pair, &canonical, &shaping).unwrap();
}

#[tokio::test(start_paused = true)]
async fn attached_static_guarantees_share_the_wan_budget_with_managed_minimums() {
    let (mut observed, mut network) = shared_input(1);
    let catalog = observed.reconciliation.registry.catalog.as_mut().unwrap();
    catalog.nodes[0].minimum.down = Some(20_000_000);
    // The static branch needs 20 Mbps; the managed circuit also needs its minimum.
    network["Management"]["uploadBandwidthMbps"] = serde_json::json!(10);
    assert!(
        super::super::publication::prepare_with_static(
            observed,
            Some(StaticInput {
                rows: &[],
                network: &network
            })
        )
        .is_err()
    );
}

#[tokio::test(start_paused = true)]
async fn attachment_policy_rejects_missing_duplicate_and_subscriber_group_parents() {
    for case in 0..4 {
        let (mut observed, _) = shared_input(1);
        let ownership = observed
            .reconciliation
            .registry
            .export_ownership
            .as_mut()
            .unwrap();
        match case {
            0 => ownership.shared_attachments[0].parent_id = "missing".into(),
            1 => ownership.shared_attachments[0].parent_id = "group1".into(),
            2 => ownership.shared_attachments[0].root_id = "unprotected".into(),
            _ => ownership
                .shared_attachments
                .push(ownership.shared_attachments[0].clone()),
        }
        assert!(observed.reconciliation.registry.validate().is_err());
    }
}

#[tokio::test(start_paused = true)]
async fn multiple_wans_and_independent_roots_keep_their_explicit_static_scope() {
    let (mut observed, mut network) = shared_input(0);
    network["Backup"] = serde_json::json!({"id":"backup-root","type":"site",
        "downloadBandwidthMbps":30,"uploadBandwidthMbps":10,"children":{}});
    network["Independent"] = serde_json::json!({"id":"independent-root","type":"site",
        "downloadBandwidthMbps":5,"uploadBandwidthMbps":5,"children":{}});
    let ownership = observed
        .reconciliation
        .registry
        .export_ownership
        .as_mut()
        .unwrap();
    ownership
        .protected_roots
        .extend(["backup-root".into(), "independent-root".into()]);
    ownership.shared_attachments.push(StaticAttachment {
        root_id: "backup-root".into(),
        parent_id: "wan2".into(),
    });
    let pair = prepare_with_static(
        observed,
        Some(StaticInput {
            rows: &[],
            network: &network,
        }),
    )
    .unwrap();
    let nodes = super::super::migration::nodes(pair.network()).unwrap();
    assert_eq!(
        nodes["wan1"].1["children"]["Management"],
        network["Management"]
    );
    assert_eq!(nodes["wan2"].1["children"]["Backup"], network["Backup"]);
    assert_eq!(pair.network()["Independent"], network["Independent"]);
    assert_eq!(pair.network().as_object().unwrap().len(), 3);
}

#[tokio::test(start_paused = true)]
async fn nested_guarantees_count_once_and_attachments_survive_store_restart() {
    use crate::topology_manager::{
        catalog::{CapacityPolicy, CatalogNode, FactorChoice, Factors},
        layout::NodeKind,
        rates::{NodeMinimum, RateBps},
        store::RegistryStore,
    };
    let (mut observed, mut network) = shared_input(0);
    let registry = &mut observed.reconciliation.registry;
    let catalog = registry.catalog.as_mut().unwrap();
    catalog.nodes[0].minimum.down = Some(50_000_000);
    catalog.nodes.push(CatalogNode {
        id: "regional".into(),
        name: "Regional".into(),
        parent: Some("wan1".into()),
        source_id: None,
        kind: NodeKind::Intermediate,
        capacity: CapacityPolicy::Fixed {
            base: RateBps {
                down: 100_000_000,
                up: 50_000_000,
            },
            factors: FactorChoice::Override(Factors::default()),
        },
        minimum: NodeMinimum {
            down: Some(40_000_000),
            up: None,
        },
    });
    registry
        .export_ownership
        .as_mut()
        .unwrap()
        .shared_attachments[0]
        .parent_id = "regional".into();
    network["Management"]["downloadBandwidthMbpsMin"] = serde_json::json!(20);
    network["Management"]["children"]["Leaf"] = serde_json::json!({"id":"static-leaf","type":"site",
        "downloadBandwidthMbps":20,"uploadBandwidthMbps":10,"downloadBandwidthMbpsMin":15,"children":{}});
    let state = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(state.path()).unwrap();
    store.commit(0, registry.clone()).unwrap();
    drop(store);
    let store = RegistryStore::open(state.path()).unwrap();
    *registry = store.document().unwrap().clone();
    assert_eq!(
        registry
            .export_ownership
            .as_ref()
            .unwrap()
            .shared_attachments[0]
            .parent_id,
        "regional"
    );
    let pair = prepare_with_static(
        observed,
        Some(StaticInput {
            rows: &[],
            network: &network,
        }),
    )
    .unwrap();
    let nodes = super::super::migration::nodes(pair.network()).unwrap();
    assert_eq!(
        nodes["regional"].1["children"]["Management"],
        network["Management"]
    );
    assert_eq!(nodes["wan1"].1["downloadBandwidthMbpsMin"], 50);
}

#[tokio::test(start_paused = true)]
async fn shared_commitments_fit_both_exported_wan_directions_after_quantization() {
    use crate::topology_manager::catalog::{CapacityPolicy, FactorChoice, Factors};
    use crate::topology_manager::rates::RateBps;
    for (down, up) in [(20_900_000, 100_000_000), (310_000_000, 10_900_000)] {
        let (mut observed, network) = shared_input(1);
        observed
            .reconciliation
            .registry
            .catalog
            .as_mut()
            .unwrap()
            .nodes[0]
            .capacity = CapacityPolicy::Fixed {
            base: RateBps { down, up },
            factors: FactorChoice::Override(Factors::default()),
        };
        let error = prepare_with_static(
            observed,
            Some(StaticInput {
                rows: &[],
                network: &network,
            }),
        )
        .err()
        .unwrap();
        assert!(format!("{error:#}").contains("commitments exceed its ceiling"));
    }
}

#[tokio::test(start_paused = true)]
async fn shared_static_children_cannot_exceed_parent_limits_or_hide_invalid_policy() {
    for case in 0..6 {
        let (observed, mut network) = shared_input(0);
        match case {
            0 => network["Management"]["downloadBandwidthMbps"] = serde_json::json!(400),
            1 => network["Management"]["uploadBandwidthMbps"] = serde_json::json!(200),
            2 => {
                network["Management"]["children"]["Leaf"] = serde_json::json!({"id":"leaf","downloadBandwidthMbps":21,"uploadBandwidthMbps":10})
            }
            3 => {
                network["Management"]["children"]["A"] = serde_json::json!({"id":"a","downloadBandwidthMbps":15,"uploadBandwidthMbps":1});
                network["Management"]["children"]["B"] = serde_json::json!({"id":"b","downloadBandwidthMbps":15,"uploadBandwidthMbps":1});
            }
            4 => {
                network["Management"]["children"]["Leaf"] =
                    serde_json::json!({"downloadBandwidthMbps":5,"uploadBandwidthMbps":1})
            }
            _ => network["Management"]["downloadBandwidthMbps"] = serde_json::json!(20.5),
        }
        assert!(
            prepare_with_static(
                observed,
                Some(StaticInput {
                    rows: &[],
                    network: &network
                })
            )
            .is_err(),
            "case {case}"
        );
    }
}

fn static_row(device: &str, ip: &str) -> crate::model::CsvRow {
    crate::model::CsvRow {
        circuit_id: "management-circuit".into(),
        circuit_name: "Management circuit".into(),
        device_id: device.into(),
        device_name: device.into(),
        parent_node: "Management".into(),
        parent_node_id: "static-root".into(),
        ipv4: ip.into(),
        download_min: "1".into(),
        upload_min: "1".into(),
        download_max: "50".into(),
        upload_max: "50".into(),
        ..Default::default()
    }
}
fn protect_rows(
    observed: &mut super::super::identity::AuthorizedReconciliation,
    rows: &[crate::model::CsvRow],
) {
    observed
        .reconciliation
        .registry
        .export_ownership
        .as_mut()
        .unwrap()
        .protected_rows = rows
        .iter()
        .map(|r| RowIdentity {
            circuit_id: r.circuit_id.clone(),
            device_id: r.device_id.clone(),
        })
        .collect();
}
#[tokio::test(start_paused = true)]
async fn static_circuit_minimum_counts_once_and_each_device_keeps_current_source_data() {
    for invalid in 0..3 {
        let (mut observed, mut network) = shared_input(0);
        observed
            .reconciliation
            .registry
            .catalog
            .as_mut()
            .unwrap()
            .nodes[0]
            .minimum
            .down = Some(1_000_000);
        network["Management"]["downloadBandwidthMbpsMin"] = serde_json::json!(1);
        let mut rows = vec![
            static_row("device-a", "198.51.100.1"),
            static_row("device-b", "198.51.100.2"),
        ];
        if invalid == 1 {
            rows[1].circuit_id = "second-circuit".into();
        }
        if invalid == 2 {
            rows[1].download_min = "2".into();
        }
        protect_rows(&mut observed, &rows);
        let result = prepare_with_static(
            observed,
            Some(StaticInput {
                rows: &rows,
                network: &network,
            }),
        );
        if invalid == 0 {
            let pair = result.unwrap();
            assert_eq!(pair.rows().len(), 2);
            assert!(
                pair.rows()
                    .iter()
                    .all(|row| row.download_max == "50" && row.upload_max == "50")
            );
            assert!(pair.rows().iter().any(|r| r.ipv4 == "198.51.100.2"));
            let (canonical, shaping) = super::super::admission::tests::artifacts(&pair);
            super::super::admission::check_compiled(pair, &canonical, &shaping).unwrap();
        } else {
            assert!(result.is_err());
        }
    }
}

#[tokio::test(start_paused = true)]
async fn shared_nodes_cannot_mix_direct_static_or_dynamic_circuits_with_children() {
    let (mut observed, mut network) = shared_input(0);
    let rows = vec![static_row("device-a", "198.51.100.1")];
    protect_rows(&mut observed, &rows);
    network["Management"]["children"]["Leaf"] =
        serde_json::json!({"id":"leaf","downloadBandwidthMbps":5,"uploadBandwidthMbps":1});
    assert!(
        prepare_with_static(
            observed,
            Some(StaticInput {
                rows: &rows,
                network: &network
            })
        )
        .is_err()
    );
    let (mut observed, network) = shared_input(1);
    observed.reconciliation.registry.subscribers[0].policy.pin = Some("wan1".into());
    let catalog = observed.reconciliation.registry.catalog.as_mut().unwrap();
    catalog.nodes.truncate(2);
    catalog.sources[2].targets = vec!["wan1".into(), "wan2".into()];
    assert!(
        prepare_with_static(
            observed,
            Some(StaticInput {
                rows: &[],
                network: &network
            })
        )
        .is_err()
    );
}

#[test]
fn empty_attachment_policy_keeps_the_previous_ownership_document_format() {
    let value = serde_json::to_value(ExportOwnership::default()).unwrap();
    assert!(value.get("shared_attachments").is_none());
    let old: ExportOwnership = serde_json::from_value(value).unwrap();
    assert!(old.shared_attachments.is_empty());
}

#[tokio::test(start_paused = true)]
async fn attachment_collisions_and_circuit_parent_splits_block_the_whole_candidate() {
    for case in 0..3 {
        let (mut observed, mut network) = shared_input(0);
        let mut rows = vec![];
        match case {
            0 => {
                observed
                    .reconciliation
                    .registry
                    .export_ownership
                    .as_mut()
                    .unwrap()
                    .nodes
                    .push(NodeAlias {
                        node_id: "wan2".into(),
                        id: "wan2".into(),
                        name: "Management".into(),
                    });
            }
            1 => {
                network["Management"]["children"]["Leaf"] = serde_json::json!({"id":"wan2","downloadBandwidthMbps":5,"uploadBandwidthMbps":1})
            }
            _ => {
                network["Other"] = serde_json::json!({"id":"other-root","type":"site","downloadBandwidthMbps":20,"uploadBandwidthMbps":10,"children":{}});
                observed
                    .reconciliation
                    .registry
                    .export_ownership
                    .as_mut()
                    .unwrap()
                    .protected_roots
                    .push("other-root".into());
                rows = vec![
                    static_row("device-a", "198.51.100.1"),
                    static_row("device-b", "198.51.100.2"),
                ];
                rows[1].parent_node = "Other".into();
                rows[1].parent_node_id = "other-root".into();
                protect_rows(&mut observed, &rows);
            }
        }
        assert!(
            prepare_with_static(
                observed,
                Some(StaticInput {
                    rows: &rows,
                    network: &network
                })
            )
            .is_err()
        );
    }
}

#[tokio::test(start_paused = true)]
async fn compiled_evidence_cannot_move_or_remove_an_attached_static_root() {
    for change in 0..2 {
        let (observed, network) = shared_input(0);
        let pair = prepare_with_static(
            observed,
            Some(StaticInput {
                rows: &[],
                network: &network,
            }),
        )
        .unwrap();
        let (mut canonical, shaping) = super::super::admission::tests::artifacts(&pair);
        if change == 0 {
            let nodes = canonical["nodes"].as_array_mut().unwrap();
            nodes
                .iter_mut()
                .find(|n| n["node_id"] == "static-root")
                .unwrap()["current_parent_node_id"] = serde_json::json!("wan2");
        } else {
            canonical["compatibility_network_json"]
                .as_object_mut()
                .unwrap()
                .values_mut()
                .find(|n| n["id"] == "wan1")
                .unwrap()["children"]
                .as_object_mut()
                .unwrap()
                .remove("Management");
        }
        assert!(super::super::admission::check_compiled(pair, &canonical, &shaping).is_err());
    }
}
