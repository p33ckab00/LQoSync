use super::*;
use crate::topology_manager::publication::{prepare, tests::input};

fn old_pair(pair: &PreparedPair) -> (Vec<CsvRow>, Value, MigrationManifest) {
    let mut rows = pair.rows().to_vec();
    let mut mappings = Vec::new();
    for (i, row) in rows.iter_mut().enumerate() {
        let current = RowIdentity {
            circuit_id: row.circuit_id.clone(),
            device_id: row.device_id.clone(),
        };
        row.circuit_id = format!("old-circuit-{i}");
        row.device_id = format!("old-device-{i}");
        row.circuit_name = format!("Account {i}");
        row.device_name = format!("Device {i}");
        mappings.push(RowMapping {
            legacy: RowIdentity {
                circuit_id: row.circuit_id.clone(),
                device_id: row.device_id.clone(),
            },
            current,
            legacy_circuit_name: row.circuit_name.clone(),
            legacy_device_name: row.device_name.clone(),
        });
    }
    let network = pair.network().clone();
    let manifest = MigrationManifest {
        expected_revision: 0,
        generation: 1,
        baseline_digest: fingerprint(&rows, &network),
        candidate_digest: fingerprint(pair.rows(), pair.network()),
        rows: mappings,
        nodes: vec![],
        protected_rows: vec![],
    };
    (rows, network, manifest)
}
#[tokio::test(start_paused = true)]
async fn explicit_mappings_allow_identity_and_label_equivalence_without_mutating_inputs() {
    let pair = prepare(input(2)).unwrap();
    let (rows, network, manifest) = old_pair(&pair);
    let before = rows.clone();
    assert!(
        compare_mapped(&pair, &rows, &network, &manifest)
            .unwrap()
            .is_empty()
    );
    assert_eq!(rows, before);
}
#[tokio::test(start_paused = true)]
async fn fresh_manifest_cannot_hide_changed_rates_or_endpoints() {
    let pair = prepare(input(1)).unwrap();
    for speed in [true, false] {
        let (mut rows, network, mut manifest) = old_pair(&pair);
        if speed {
            rows[0].download_max = "49".into();
        } else {
            rows[0].ipv4 = "192.0.2.200".into();
        }
        manifest.baseline_digest = fingerprint(&rows, &network);
        assert!(
            !compare_mapped(&pair, &rows, &network, &manifest)
                .unwrap()
                .is_empty()
        );
    }
}
#[tokio::test(start_paused = true)]
async fn stale_generations_revisions_or_content_digests_cannot_claim_parity() {
    let pair = prepare(input(1)).unwrap();
    let (rows, network, original) = old_pair(&pair);
    for field in 0..4 {
        let mut manifest = original.clone();
        match field {
            0 => manifest.expected_revision += 1,
            1 => manifest.generation += 1,
            2 => manifest.baseline_digest = "stale".into(),
            _ => manifest.candidate_digest = "stale".into(),
        }
        assert!(compare_mapped(&pair, &rows, &network, &manifest).is_err());
    }
}
#[tokio::test(start_paused = true)]
async fn duplicate_or_many_to_one_identity_maps_are_critical_conflicts() {
    let pair = prepare(input(2)).unwrap();
    let (rows, network, mut manifest) = old_pair(&pair);
    manifest.rows[1].current = manifest.rows[0].current.clone();
    assert!(compare_mapped(&pair, &rows, &network, &manifest).is_err());
    let (rows, network, mut manifest) = old_pair(&pair);
    manifest.rows.push(manifest.rows[0].clone());
    assert!(compare_mapped(&pair, &rows, &network, &manifest).is_err());
}
#[tokio::test(start_paused = true)]
async fn protected_or_unclassified_rows_stay_visible_and_cannot_be_adopted_implicitly() {
    let pair = prepare(input(1)).unwrap();
    let (mut rows, network, mut manifest) = old_pair(&pair);
    let mut static_row = rows[0].clone();
    static_row.circuit_id = "static".into();
    static_row.device_id = "static-device".into();
    rows.push(static_row);
    manifest.baseline_digest = fingerprint(&rows, &network);
    assert!(
        !compare_mapped(&pair, &rows, &network, &manifest)
            .unwrap()
            .is_empty()
    );
    manifest.protected_rows.push(RowIdentity {
        circuit_id: "static".into(),
        device_id: "static-device".into(),
    });
    assert!(
        !compare_mapped(&pair, &rows, &network, &manifest)
            .unwrap()
            .is_empty()
    );
    manifest
        .protected_rows
        .push(manifest.rows[0].legacy.clone());
    assert!(compare_mapped(&pair, &rows, &network, &manifest).is_err());
}

#[tokio::test(start_paused = true)]
async fn identical_export_ids_do_not_replace_explicit_ownership_or_mapping() {
    let pair = prepare(input(1)).unwrap();
    let mut manifest = MigrationManifest {
        expected_revision: 0,
        generation: 1,
        baseline_digest: fingerprint(pair.rows(), pair.network()),
        candidate_digest: fingerprint(pair.rows(), pair.network()),
        rows: vec![],
        nodes: vec![],
        protected_rows: vec![],
    };
    assert!(
        !compare_mapped(&pair, pair.rows(), pair.network(), &manifest)
            .unwrap()
            .is_empty()
    );
    manifest.protected_rows.push(RowIdentity {
        circuit_id: pair.rows()[0].circuit_id.clone(),
        device_id: pair.rows()[0].device_id.clone(),
    });
    assert!(
        !compare_mapped(&pair, pair.rows(), pair.network(), &manifest)
            .unwrap()
            .is_empty()
    );
}

#[tokio::test(start_paused = true)]
async fn explicit_node_maps_preserve_ancestry_and_never_hide_rate_or_metadata_changes() {
    let pair = prepare(input(2)).unwrap();
    let (mut rows, network, mut manifest) = old_pair(&pair);
    fn legacy(tree: &Value, mappings: &mut Vec<NodeMapping>) -> Value {
        let mut old = serde_json::Map::new();
        for (name, node) in tree.as_object().unwrap() {
            let id = node["id"].as_str().unwrap();
            let old_id = format!("legacy-{id}");
            let old_name = format!("Legacy {name}");
            mappings.push(NodeMapping {
                legacy_id: old_id.clone(),
                legacy_name: old_name.clone(),
                current_id: id.into(),
            });
            let mut node = node.clone();
            node["id"] = old_id.into();
            node["children"] = legacy(&node["children"], mappings);
            old.insert(old_name, node);
        }
        Value::Object(old)
    }
    let network = legacy(&network, &mut manifest.nodes);
    for row in &mut rows {
        let mapping = manifest
            .nodes
            .iter()
            .find(|mapping| mapping.current_id == row.parent_node_id)
            .unwrap();
        row.parent_node_id = mapping.legacy_id.clone();
        row.parent_node = mapping.legacy_name.clone();
    }
    manifest.baseline_digest = fingerprint(&rows, &network);
    assert!(
        compare_mapped(&pair, &rows, &network, &manifest)
            .unwrap()
            .is_empty()
    );
    let mut changed = network.clone();
    let first = changed
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap();
    first["latitude"] = 1.into();
    manifest.baseline_digest = fingerprint(&rows, &changed);
    assert!(
        !compare_mapped(&pair, &rows, &changed, &manifest)
            .unwrap()
            .is_empty()
    );
    manifest.baseline_digest = fingerprint(&rows, &network);
    manifest.nodes[1].current_id = manifest.nodes[0].current_id.clone();
    assert!(compare_mapped(&pair, &rows, &network, &manifest).is_err());
}

#[tokio::test(start_paused = true)]
async fn manifest_json_has_no_endpoint_or_session_fields_and_rejects_unknown_input() {
    let pair = prepare(input(1)).unwrap();
    let (_, _, manifest) = old_pair(&pair);
    let mut value = serde_json::to_value(&manifest).unwrap();
    serde_json::from_value::<MigrationManifest>(value.clone()).unwrap();
    value["rows"][0]["ip"] = "192.0.2.1".into();
    assert!(serde_json::from_value::<MigrationManifest>(value).is_err());
}
