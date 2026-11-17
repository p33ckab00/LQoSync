use super::*;
use crate::model::CsvRow;
use crate::topology_manager::{
    migration::{MigrationManifest, NodeMapping, RowIdentity, RowMapping, fingerprint},
    ownership::StaticAttachment,
    publication::{PreparedPair, prepare, prepare_with_static, tests::input},
};
use serde_json::{Value, json};

pub(crate) struct Fixture {
    pub pair: PreparedPair,
    pub rows: Vec<CsvRow>,
    pub network: Value,
    pub manifest: MigrationManifest,
    pub roots: Vec<String>,
    pub attachments: Vec<StaticAttachment>,
}
impl Fixture {
    fn adopt(&self) -> anyhow::Result<StaticAdoption> {
        adopt(
            &self.pair,
            &self.rows,
            &self.network,
            &self.manifest,
            &self.roots,
            &self.attachments,
        )
    }
    fn refresh(&mut self) {
        self.manifest.baseline_digest = fingerprint(&self.rows, &self.network);
    }
}
pub(crate) fn fixture() -> Fixture {
    fixture_minimum(None)
}
fn fixture_minimum(minimum: Option<u64>) -> Fixture {
    let mut observed = input(1);
    observed.reconciliation.registry.subscribers[0].policy.pin = Some("group1".into());
    observed
        .reconciliation
        .registry
        .catalog
        .as_mut()
        .unwrap()
        .nodes[0]
        .minimum
        .down = minimum;
    let pair = prepare(observed).unwrap();
    let nodes = crate::topology_manager::migration::nodes(pair.network()).unwrap();
    let wan_label = nodes["wan1"].0.to_string();
    let group_label = nodes["group1"].0.to_string();
    let mut network = pair.network().clone();
    let mut wan = network.as_object_mut().unwrap().remove(&wan_label).unwrap();
    wan["id"] = json!("legacy-wan");
    let children = wan["children"].as_object_mut().unwrap();
    let mut group = children.remove(&group_label).unwrap();
    group["id"] = json!("legacy-group");
    children.insert("Legacy subscribers".into(), group);
    children.insert(
        "Management".into(),
        json!({"id":"static-root","type":"site",
        "downloadBandwidthMbps":20,"uploadBandwidthMbps":10,"children":{}}),
    );
    network
        .as_object_mut()
        .unwrap()
        .insert("Legacy WAN".into(), wan);
    let mut rows = pair.rows().to_vec();
    rows[0].circuit_id = "legacy-circuit".into();
    rows[0].device_id = "legacy-device".into();
    rows[0].circuit_name = "Legacy account".into();
    rows[0].device_name = "Legacy device".into();
    rows[0].parent_node = "Legacy subscribers".into();
    rows[0].parent_node_id = "legacy-group".into();
    let managed = rows[0].clone();
    rows.push(CsvRow {
        circuit_id: "static-circuit".into(),
        device_id: "static-device".into(),
        circuit_name: "Management circuit".into(),
        device_name: "Management device".into(),
        parent_node: "Management".into(),
        parent_node_id: "static-root".into(),
        ipv4: "198.51.100.9".into(),
        download_min: "1".into(),
        upload_min: "1".into(),
        download_max: "50".into(),
        upload_max: "50".into(),
        ..Default::default()
    });
    let manifest = MigrationManifest {
        expected_revision: pair.expected_revision(),
        generation: pair.generation(),
        baseline_digest: fingerprint(&rows, &network),
        candidate_digest: fingerprint(pair.rows(), pair.network()),
        rows: vec![RowMapping {
            legacy: RowIdentity {
                circuit_id: managed.circuit_id,
                device_id: managed.device_id,
            },
            current: RowIdentity {
                circuit_id: pair.rows()[0].circuit_id.clone(),
                device_id: pair.rows()[0].device_id.clone(),
            },
            legacy_circuit_name: managed.circuit_name,
            legacy_device_name: managed.device_name,
        }],
        nodes: vec![
            NodeMapping {
                legacy_id: "legacy-wan".into(),
                legacy_name: "Legacy WAN".into(),
                current_id: "wan1".into(),
            },
            NodeMapping {
                legacy_id: "legacy-group".into(),
                legacy_name: "Legacy subscribers".into(),
                current_id: "group1".into(),
            },
        ],
        protected_rows: vec![RowIdentity {
            circuit_id: "static-circuit".into(),
            device_id: "static-device".into(),
        }],
    };
    Fixture {
        pair,
        rows,
        network,
        manifest,
        roots: vec!["static-root".into()],
        attachments: vec![StaticAttachment {
            root_id: "static-root".into(),
            parent_id: "wan1".into(),
        }],
    }
}

#[tokio::test(start_paused = true)]
async fn shared_adoption_extracts_only_explicit_static_policy_and_keeps_managed_aliases() {
    let f = fixture();
    let result = f.adopt().unwrap();
    let source = result.static_input();
    assert_eq!(source.rows, &f.rows[1..]);
    assert_eq!(
        source.network,
        &json!({"Management":f.network["Legacy WAN"]["children"]["Management"]})
    );
    assert_eq!(
        result
            .registry
            .export_ownership
            .as_ref()
            .unwrap()
            .shared_attachments,
        f.attachments
    );
    let document = result.source_document(1).unwrap();
    let text = std::str::from_utf8(&document).unwrap();
    assert!(!text.contains(&f.rows[0].ipv4));
    assert!(!text.contains("legacy-device"));
    let registry = serde_json::to_string(&result.registry).unwrap();
    assert!(!registry.contains("198.51.100.9"));
    assert!(!registry.contains(&f.rows[0].ipv4));
    assert!(result.source_document(0).is_err());
    let mut observed = input(1);
    observed.reconciliation.registry = result.registry.clone();
    let regenerated = prepare_with_static(observed, Some(result.static_input())).unwrap();
    assert_eq!(regenerated.network(), &f.network);
    assert_eq!(
        regenerated
            .rows()
            .iter()
            .find(|r| r.device_id == "legacy-device")
            .unwrap(),
        &f.rows[0]
    );
}

#[tokio::test(start_paused = true)]
async fn adoption_rejects_stale_evidence_and_fresh_snapshot_policy_or_endpoint_drift() {
    for case in 0..5 {
        let mut f = fixture();
        match case {
            0 => f.manifest.generation += 1,
            1 => f.manifest.baseline_digest = "bad".into(),
            2 => {
                f.rows[0].ipv4 = "192.0.2.99".into();
                f.refresh();
            }
            3 => {
                f.network["Legacy WAN"]["downloadBandwidthMbps"] = json!(309);
                f.refresh();
            }
            _ => f.manifest.nodes[0].legacy_name = "Wrong label".into(),
        }
        assert!(f.adopt().is_err(), "case {case}");
    }
}

#[tokio::test(start_paused = true)]
async fn selected_roots_cannot_move_between_wans_during_adoption() {
    for case in 0..6 {
        let mut f = fixture();
        match case {
            0 => f.attachments[0].parent_id = "wan2".into(),
            1 => f.attachments.clear(),
            2 => f.attachments.push(f.attachments[0].clone()),
            3 => f.roots.push(f.roots[0].clone()),
            4 => {
                f.network["Office"] = json!({"id":"office-root","type":"site","downloadBandwidthMbps":5,"uploadBandwidthMbps":5,"children":{}});
                f.roots.push("office-root".into());
                f.attachments.push(StaticAttachment {
                    root_id: "office-root".into(),
                    parent_id: "wan1".into(),
                });
                f.refresh();
            }
            _ => {
                let branch = f.network["Legacy WAN"]["children"]
                    .as_object_mut()
                    .unwrap()
                    .remove("Management")
                    .unwrap();
                f.network
                    .as_object_mut()
                    .unwrap()
                    .values_mut()
                    .find(|n| n["id"] == "wan2")
                    .unwrap()["children"]["Management"] = branch;
                f.refresh();
            }
        }
        assert!(f.adopt().is_err(), "case {case}");
    }
}

#[tokio::test(start_paused = true)]
async fn independent_and_nested_static_roots_share_one_exact_source_draft() {
    let mut f = fixture();
    f.network["Office"] = json!({"id":"office-root","type":"site","downloadBandwidthMbps":5,"uploadBandwidthMbps":5,"children":{}});
    f.roots.push("office-root".into());
    f.refresh();
    let result = f.adopt().unwrap();
    assert_eq!(result.static_input().network.as_object().unwrap().len(), 2);
    assert_eq!(result.static_input().network["Office"], f.network["Office"]);
    let mut observed = input(1);
    observed.reconciliation.registry = result.registry.clone();
    let pair = prepare_with_static(observed, Some(result.static_input())).unwrap();
    assert_eq!(pair.network(), &f.network);
}

#[tokio::test(start_paused = true)]
async fn extraction_rejects_incomplete_classification_overlaps_and_managed_identity_collisions() {
    for case in 0..7 {
        let mut f = fixture();
        match case {
            0 => f.manifest.protected_rows.clear(),
            1 => {
                f.rows[1].parent_node = "Legacy WAN".into();
                f.rows[1].parent_node_id = "legacy-wan".into();
            }
            2 => f.roots[0] = "missing-root".into(),
            3 => {
                f.network["Legacy WAN"]["children"]["Management"]["children"]["Nested"] =
                    json!({"id":"nested","downloadBandwidthMbps":5,"uploadBandwidthMbps":5});
                f.roots.push("nested".into());
                f.attachments.push(StaticAttachment {
                    root_id: "nested".into(),
                    parent_id: "wan1".into(),
                });
            }
            4 => {
                f.roots[0] = "legacy-group".into();
                f.attachments[0].root_id = "legacy-group".into();
            }
            5 => {
                f.network["Legacy WAN"]["children"]["Management"]["children"]["Collision"] =
                    json!({"id":"legacy-group","downloadBandwidthMbps":5,"uploadBandwidthMbps":5})
            }
            _ => {
                f.network["Legacy WAN"]["children"]["Management"]["children"]["Dormant collision"] =
                    json!({"id":"group2","downloadBandwidthMbps":5,"uploadBandwidthMbps":5})
            }
        }
        f.refresh();
        assert!(f.adopt().is_err(), "case {case}");
    }
}

#[tokio::test(start_paused = true)]
async fn migration_cannot_bypass_static_child_or_shared_guarantee_limits() {
    let f = fixture_minimum(Some(20_000_000));
    assert!(f.adopt().is_err());
    let mut f = fixture();
    f.network["Legacy WAN"]["children"]["Management"]["downloadBandwidthMbps"] = json!(400);
    f.refresh();
    assert!(f.adopt().is_err());
}

#[tokio::test(start_paused = true)]
async fn repeated_proposals_are_stable_and_expired_evidence_cannot_be_reused() {
    let f = fixture();
    let first = f.adopt().unwrap();
    let second = f.adopt().unwrap();
    assert_eq!(
        serde_json::to_value(&first.registry).unwrap(),
        serde_json::to_value(&second.registry).unwrap()
    );
    assert_eq!(
        first.source_document(1).unwrap(),
        second.source_document(1).unwrap()
    );
    tokio::time::advance(std::time::Duration::from_secs(11)).await;
    assert!(f.adopt().is_err());
}

#[tokio::test(start_paused = true)]
async fn restart_and_disconnect_never_recover_dynamic_endpoints_from_the_static_draft() {
    use crate::topology_manager::{static_input::CurrentStatic, store::RegistryStore};
    let f = fixture();
    let result = f.adopt().unwrap();
    let state = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(state.path()).unwrap();
    let path = state.path().join("topology-manager/static-input.json");
    crate::files::write_secret(&path, &result.source_document(1).unwrap()).unwrap();
    store.commit(0, result.registry.clone()).unwrap();
    drop(store);
    let store = RegistryStore::open(state.path()).unwrap();
    let source = CurrentStatic::load(state.path()).unwrap();
    let mut offline = input(0);
    offline.reconciliation.registry = store.document().unwrap().clone();
    let offline = prepare_with_static(offline, Some(source.input())).unwrap();
    assert_eq!(offline.rows(), &f.rows[1..]);
    let mut online = input(1);
    online.reconciliation.registry = offline.candidate.clone();
    online.reconciliation.active[0].endpoints[0].endpoint.ips = vec!["192.0.2.99".parse().unwrap()];
    online.reconciliation.active[0].endpoints[0].endpoint.mac = Some("AA:BB:CC:DD:EE:99".into());
    let online = prepare_with_static(online, Some(source.input())).unwrap();
    let row = online
        .rows()
        .iter()
        .find(|r| r.device_id == "legacy-device")
        .unwrap();
    assert_eq!(row.circuit_id, "legacy-circuit");
    assert_eq!(row.ipv4, "192.0.2.99");
    assert_eq!(row.mac, "AA:BB:CC:DD:EE:99");
    assert!(
        !std::fs::read_to_string(path)
            .unwrap()
            .contains("192.0.2.99")
    );
}

#[test]
fn extraction_accepts_the_supported_depth_and_rejects_deeper_nonempty_trees() {
    let mut tree = json!({});
    for i in (0..63).rev() {
        tree = json!({format!("Node {i}"):{"id":format!("node-{i}"),"children":tree}});
    }
    let mut selected = HashSet::new();
    selected.insert("node-62");
    let mut source = serde_json::Map::new();
    let mut parents = HashMap::new();
    extract(&tree, &selected, None, &mut source, &mut parents, 0).unwrap();
    assert_eq!(parents["node-62"].as_deref(), Some("node-61"));
    let too_deep = json!({"Extra":{"id":"extra","children":tree}});
    assert!(
        extract(
            &too_deep,
            &selected,
            None,
            &mut serde_json::Map::new(),
            &mut HashMap::new(),
            0
        )
        .is_err()
    );
    extract(
        &json!({}),
        &HashSet::new(),
        None,
        &mut serde_json::Map::new(),
        &mut HashMap::new(),
        63,
    )
    .unwrap();
}

#[tokio::test]
#[ignore = "requires explicit installed-binding scratch compilation"]
async fn installed_compiler_preserves_the_migrated_shared_legacy_snapshot() {
    use crate::topology_manager::{compiler::ControlledCompiler, static_input::CurrentStatic};
    use std::os::unix::fs::DirBuilderExt;
    use std::path::Path;
    let f = fixture();
    let result = f.adopt().unwrap();
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("topology-manager");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .unwrap();
    crate::files::write_secret(
        &directory.join("static-input.json"),
        &result.source_document(1).unwrap(),
    )
    .unwrap();
    let source = CurrentStatic::load(root.path()).unwrap();
    let mut observed = input(1);
    observed.reconciliation.registry = result.registry;
    let mut pair = prepare_with_static(observed, Some(source.input())).unwrap();
    pair.static_source = Some(source.binding);
    let library =
        std::env::var("LQOSYNC3_RUNTIME_BINDING").expect("set LQOSYNC3_RUNTIME_BINDING explicitly");
    let config = root.path().join("lqos.conf");
    std::fs::write(
        &config,
        std::fs::read(Path::new(&library).join("lqos.example")).unwrap(),
    )
    .unwrap();
    let native: toml::Table = toml::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
    let cfg = crate::config::Libreqos {
        directory: library,
        state_directory: native["state_directory"].as_str().unwrap().into(),
        v3_config: config.to_str().unwrap().into(),
        ..Default::default()
    };
    let mut compiler = ControlledCompiler::open(&cfg, &root.path().join("scratch")).unwrap();
    let compiled = compiler.compile(pair).await.unwrap();
    compiled.check().unwrap();
}

#[test]
fn source_drafts_cannot_exceed_current_reader_limits() {
    let mut proposal = StaticAdoption {
        registry: Default::default(),
        rows: vec![],
        network: json!({"oversized":"x".repeat(crate::topology_manager::admission::MAX_PAIR_BYTES)}),
    };
    assert!(proposal.source_document(1).is_err());
    proposal.network = json!({});
    proposal.rows =
        vec![CsvRow::default(); crate::topology_manager::static_input::MAX_STATIC_ROWS + 1];
    assert!(proposal.source_document(1).is_err());
}
