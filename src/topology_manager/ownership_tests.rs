use super::*;
use crate::topology_manager::publication::{prepare, tests::input};

fn policy() -> (
    super::super::identity::AuthorizedReconciliation,
    ExportOwnership,
) {
    let observed = input(1);
    let subscriber = &observed.reconciliation.registry.subscribers[0];
    let ownership = ExportOwnership {
        circuits: vec![CircuitAlias {
            subscriber_id: subscriber.id.clone(),
            id: "legacy-circuit".into(),
            name: "Alice".into(),
        }],
        devices: vec![DeviceAlias {
            identity: subscriber.identities[0].clone(),
            id: "legacy-device".into(),
            name: "Alice device".into(),
        }],
        ..Default::default()
    };
    (observed, ownership)
}
#[tokio::test(start_paused = true)]
async fn aliases_keep_legacy_ids_and_labels_with_current_endpoints() {
    let (mut observed, ownership) = policy();
    observed.reconciliation.registry.export_ownership = Some(ownership);
    let pair = prepare(observed).unwrap();
    assert_eq!(pair.rows()[0].circuit_id, "legacy-circuit");
    assert_eq!(pair.rows()[0].device_id, "legacy-device");
    assert_eq!(pair.rows()[0].circuit_name, "Alice");
    assert_eq!(pair.rows()[0].ipv4, "10.0.0.1");
    assert_eq!(pair.rows()[0].download_max, "50");
}
#[tokio::test(start_paused = true)]
async fn protected_static_ownership_blocks_missing_current_input() {
    let (mut observed, mut ownership) = policy();
    ownership
        .protected_rows
        .push(super::super::migration::RowIdentity {
            circuit_id: "static-circuit".into(),
            device_id: "static-device".into(),
        });
    ownership.protected_roots.push("static-root".into());
    observed.reconciliation.registry.export_ownership = Some(ownership);
    assert!(prepare(observed).is_err());
}
#[test]
fn aliases_cannot_claim_another_subscriber_or_endpoint_fields() {
    let (observed, mut ownership) = policy();
    ownership.devices[0].identity.account_ref = "unknown".into();
    assert!(
        ownership
            .validate(&observed.reconciliation.registry)
            .is_err()
    );
    let (_, ownership) = policy();
    let mut json = serde_json::to_value(ownership).unwrap();
    json["ipv4"] = "192.0.2.1".into();
    assert!(serde_json::from_value::<ExportOwnership>(json).is_err());
}

#[tokio::test(start_paused = true)]
async fn aliases_reject_multiple_current_sessions_for_one_legacy_device() {
    let (mut observed, ownership) = policy();
    observed.reconciliation.registry.export_ownership = Some(ownership);
    let original = &observed.reconciliation.active[0].endpoints[0];
    let endpoint = super::super::observation::ActiveEndpoint {
        identity: original.identity.clone(),
        endpoint: original.endpoint.clone(),
    };
    observed.reconciliation.active[0].endpoints.push(endpoint);
    assert!(prepare(observed).is_err());
}

#[tokio::test(start_paused = true)]
async fn current_static_input_is_preserved_and_missing_or_changed_ownership_blocks() {
    let (mut observed, mut ownership) = policy();
    let row = crate::model::CsvRow {
        circuit_id: "static-circuit".into(),
        device_id: "static-device".into(),
        circuit_name: "Management".into(),
        device_name: "Static".into(),
        parent_node: "Static root".into(),
        parent_node_id: "static-root".into(),
        ipv4: "192.0.2.8".into(),
        download_min: "1".into(),
        upload_min: "1".into(),
        download_max: "5".into(),
        upload_max: "5".into(),
        ..Default::default()
    };
    ownership.protected_rows = vec![super::super::migration::RowIdentity {
        circuit_id: row.circuit_id.clone(),
        device_id: row.device_id.clone(),
    }];
    ownership.protected_roots = vec!["static-root".into()];
    observed.reconciliation.registry.export_ownership = Some(ownership);
    let network = serde_json::json!({"Static root":{"id":"static-root", "type":"site", "downloadBandwidthMbps":20,"uploadBandwidthMbps":20, "children":{}}});
    let rows = vec![row.clone()];
    let pair = super::super::publication::prepare_with_static(
        observed,
        Some(StaticInput {
            rows: &rows,
            network: &network,
        }),
    )
    .unwrap();
    assert!(pair.rows().contains(&row));
    let (canonical, shaping) = super::super::admission::tests::artifacts(&pair);
    super::super::admission::check_compiled(pair, &canonical, &shaping).unwrap();
}

#[tokio::test(start_paused = true)]
async fn durable_adoption_requires_exact_snapshot_policy() {
    let pair = prepare(input(1)).unwrap();
    let mut rows = pair.rows().to_vec();
    let current = super::super::migration::RowIdentity {
        circuit_id: rows[0].circuit_id.clone(),
        device_id: rows[0].device_id.clone(),
    };
    rows[0].circuit_id = "old-circuit".into();
    rows[0].device_id = "old-device".into();
    rows[0].circuit_name = "Alice".into();
    rows[0].device_name = "Alice device".into();
    let mut manifest = super::super::migration::MigrationManifest {
        expected_revision: pair.expected_revision(),
        generation: pair.generation(),
        baseline_digest: super::super::migration::fingerprint(&rows, pair.network()),
        candidate_digest: super::super::migration::fingerprint(pair.rows(), pair.network()),
        rows: vec![super::super::migration::RowMapping {
            legacy: super::super::migration::RowIdentity {
                circuit_id: rows[0].circuit_id.clone(),
                device_id: rows[0].device_id.clone(),
            },
            current,
            legacy_circuit_name: rows[0].circuit_name.clone(),
            legacy_device_name: rows[0].device_name.clone(),
        }],
        nodes: vec![],
        protected_rows: vec![],
    };
    let adopted = adopt_legacy(&pair, &rows, pair.network(), &manifest, &[]).unwrap();
    assert_eq!(
        adopted.export_ownership.as_ref().unwrap().devices[0].id,
        "old-device"
    );
    assert!(
        !serde_json::to_string(&adopted)
            .unwrap()
            .contains("10.0.0.1")
    );
    rows[0].download_max = "49".into();
    manifest.baseline_digest = super::super::migration::fingerprint(&rows, pair.network());
    assert!(adopt_legacy(&pair, &rows, pair.network(), &manifest, &[]).is_err());
}

#[tokio::test(start_paused = true)]
async fn offline_and_reconnect_keep_only_fresh_endpoints_and_durable_aliases() {
    let (mut first, ownership) = policy();
    first.reconciliation.registry.export_ownership = Some(ownership);
    let saved = serde_json::to_vec(&first.reconciliation.registry).unwrap();
    let mut offline = input(0);
    offline.reconciliation.registry = serde_json::from_slice(&saved).unwrap();
    assert!(prepare(offline).unwrap().rows().is_empty());
    let mut returned = input(1);
    returned.reconciliation.registry = serde_json::from_slice(&saved).unwrap();
    let endpoint = &mut returned.reconciliation.active[0].endpoints[0].endpoint;
    endpoint.session_id = "new-session".into();
    endpoint.ips = vec!["192.0.2.75".parse().unwrap()];
    endpoint.mac = Some("02:00:00:00:00:75".into());
    let pair = prepare(returned).unwrap();
    assert_eq!(pair.rows()[0].device_id, "legacy-device");
    assert_eq!(pair.rows()[0].ipv4, "192.0.2.75");
    assert!(!pair.csv().contains("10.0.0.1"));
}

#[tokio::test(start_paused = true)]
async fn node_aliases_keep_parent_references_and_compiled_policy_consistent() {
    let (mut observed, mut ownership) = policy();
    ownership.nodes.push(NodeAlias {
        node_id: "wan1".into(),
        id: "old-wan".into(),
        name: "WAN one".into(),
    });
    observed.reconciliation.registry.export_ownership = Some(ownership);
    let pair = prepare(observed).unwrap();
    assert_eq!(pair.network()["WAN one"]["id"], "old-wan");
    let (canonical, shaping) = super::super::admission::tests::artifacts(&pair);
    super::super::admission::check_compiled(pair, &canonical, &shaping).unwrap();
}

#[tokio::test(start_paused = true)]
async fn a_static_row_cannot_escape_its_owned_root_by_omitting_its_parent() {
    let (mut observed, mut ownership) = policy();
    ownership.protected_rows = vec![super::super::migration::RowIdentity {
        circuit_id: "static".into(),
        device_id: "static-device".into(),
    }];
    ownership.protected_roots = vec!["static-root".into()];
    observed.reconciliation.registry.export_ownership = Some(ownership);
    let row = crate::model::CsvRow {
        circuit_id: "static".into(),
        device_id: "static-device".into(),
        circuit_name: "Static".into(),
        device_name: "Static".into(),
        ipv4: "192.0.2.8".into(),
        download_min: "1".into(),
        upload_min: "1".into(),
        download_max: "5".into(),
        upload_max: "5".into(),
        ..Default::default()
    };
    let network = serde_json::json!({"Static root":{"id":"static-root","type":"site","downloadBandwidthMbps":20,"uploadBandwidthMbps":20,"children":{}}});
    assert!(
        super::super::publication::prepare_with_static(
            observed,
            Some(StaticInput {
                rows: &[row],
                network: &network
            })
        )
        .is_err()
    );
}

#[tokio::test(start_paused = true)]
async fn adopted_static_ownership_and_aliases_survive_store_restart_and_fresh_preparation() {
    use super::super::migration::{MigrationManifest, RowIdentity, RowMapping, fingerprint};
    use super::super::publication::prepare_with_static;
    use super::super::store::RegistryStore;
    let pair = prepare(input(1)).unwrap();
    let mut baseline = pair.rows().to_vec();
    let current = RowIdentity {
        circuit_id: baseline[0].circuit_id.clone(),
        device_id: baseline[0].device_id.clone(),
    };
    baseline[0].circuit_id = "old-circuit".into();
    baseline[0].device_id = "old-device".into();
    baseline[0].circuit_name = "Alice".into();
    baseline[0].device_name = "Alice device".into();
    let static_row = crate::model::CsvRow {
        circuit_id: "static".into(),
        device_id: "static-device".into(),
        circuit_name: "Management".into(),
        device_name: "Management".into(),
        parent_node: "Management root".into(),
        parent_node_id: "static-root".into(),
        ipv4: "192.0.2.9".into(),
        download_min: "1".into(),
        upload_min: "1".into(),
        download_max: "5".into(),
        upload_max: "5".into(),
        ..Default::default()
    };
    let static_tree = serde_json::json!({"Management root":{"id":"static-root","type":"site","downloadBandwidthMbps":20,"uploadBandwidthMbps":20,"children":{}}});
    let mut network = pair.network().clone();
    network
        .as_object_mut()
        .unwrap()
        .extend(static_tree.as_object().unwrap().clone());
    baseline.push(static_row.clone());
    let manifest = MigrationManifest {
        expected_revision: pair.expected_revision(),
        generation: pair.generation(),
        baseline_digest: fingerprint(&baseline, &network),
        candidate_digest: fingerprint(pair.rows(), pair.network()),
        rows: vec![RowMapping {
            legacy: RowIdentity {
                circuit_id: "old-circuit".into(),
                device_id: "old-device".into(),
            },
            current,
            legacy_circuit_name: "Alice".into(),
            legacy_device_name: "Alice device".into(),
        }],
        nodes: vec![],
        protected_rows: vec![RowIdentity {
            circuit_id: "static".into(),
            device_id: "static-device".into(),
        }],
    };
    let adopted = adopt_legacy(
        &pair,
        &baseline,
        &network,
        &manifest,
        &["static-root".into()],
    )
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(root.path()).unwrap();
    store.commit(0, adopted).unwrap();
    drop(store);
    let store = RegistryStore::open(root.path()).unwrap();
    let mut current = input(1);
    current.reconciliation.registry = store.document().unwrap().clone();
    current.reconciliation.active[0].endpoints[0].endpoint.ips =
        vec!["192.0.2.80".parse().unwrap()];
    let pair = prepare_with_static(
        current,
        Some(StaticInput {
            rows: std::slice::from_ref(&static_row),
            network: &static_tree,
        }),
    )
    .unwrap();
    assert!(pair.rows().contains(&static_row));
    let managed = pair
        .rows()
        .iter()
        .find(|r| r.circuit_id == "old-circuit")
        .unwrap();
    assert_eq!(managed.device_id, "old-device");
    assert_eq!(managed.ipv4, "192.0.2.80");
    assert_eq!(managed.download_max, "50");
    let (canonical, shaping) = super::super::admission::tests::artifacts(&pair);
    super::super::admission::check_compiled(pair, &canonical, &shaping).unwrap();
}
