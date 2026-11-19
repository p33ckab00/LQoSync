use super::*;
use crate::model::CsvRow;
use serde_json::json;
use std::path::Path;

fn row(id: &str, parent: &str, min: &str, max: &str) -> CsvRow {
    CsvRow {
        circuit_id: id.into(),
        device_id: format!("dev-{id}"),
        parent_node: parent.into(),
        download_min: min.into(),
        upload_min: min.into(),
        download_max: max.into(),
        upload_max: max.into(),
        ..CsvRow::default()
    }
}

fn site(id: &str, mbps: f64, extra: Value) -> Value {
    let mut v = json!({"id": id, "type": "site", "downloadBandwidthMbps": mbps,
        "uploadBandwidthMbps": mbps, "children": {}});
    if let (Some(o), Some(e)) = (v.as_object_mut(), extra.as_object()) {
        o.extend(e.clone());
    }
    v
}

fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/live_state")
}

fn fixture_expected() -> Expected {
    let net =
        json!({"Site-A": site("n-a", 100.0, json!({})), "Site-B": site("n-b", 225.75, json!({}))});
    let rows = [
        row("c1", "Site-A", "3.5", "20"),
        row("c2", "Site-A", "5", "200"),
    ];
    expected(&rows, &net).unwrap()
}

#[test]
fn implicit_node_commit_is_the_floored_child_sum() {
    let net = json!({"Top": site("top", 500.0, json!({"children": {
        "Mid": site("mid", 300.0, json!({}))}}))});
    let rows = [row("c1", "Mid", "3.5", "10"), row("c2", "Mid", "2.5", "10")];
    let e = expected(&rows, &net).unwrap();
    // LibreQoS floors the sum of the raw circuit minimums: 3.5 + 2.5 gives 6 Mbps.
    assert_eq!(e.nodes["mid"].download.commit_kbps, 6000);
    assert_eq!(e.nodes["top"].download.commit_kbps, 6000);
    let e = expected(&[row("c1", "Mid", "3.5", "10")], &net).unwrap();
    assert_eq!(e.circuits["c1"].download.commit_kbps, 4000);
    assert_eq!(e.nodes["mid"].download.commit_kbps, 3000);
    let net = json!({"Top": site("top", 500.0, json!({"downloadBandwidthMbpsMin": 50.0,
        "uploadBandwidthMbpsMin": 60.0}))});
    let e = expected(&[], &net).unwrap();
    assert_eq!(e.nodes["top"].download.commit_kbps, 50000);
    assert_eq!(e.nodes["top"].upload.commit_kbps, 60000);
}

#[test]
fn circuit_minimum_and_node_ceiling_round_up() {
    let e = fixture_expected();
    assert_eq!(e.circuits["c1"].download.commit_kbps, 4000);
    assert_eq!(e.nodes["n-b"].download.ceil_kbps, 226000);
}

#[test]
fn a_circuit_above_its_parent_is_clamped_with_a_warning() {
    let e = fixture_expected();
    assert_eq!(e.circuits["c2"].download.ceil_kbps, 100000);
    assert_eq!(e.circuits["c1"].download.ceil_kbps, 20000);
    assert_eq!(e.warnings.len(), 2, "one per direction: {:?}", e.warnings);
}

#[test]
fn admission_names_a_node_whose_commitments_exceed_its_parent() {
    let net = json!({"Site-A": site("n-a", 10.0, json!({}))});
    let rows = [
        row("c1", "Site-A", "6", "10"),
        row("c2", "Site-A", "6", "10"),
    ];
    let e = expected(&rows, &net).unwrap();
    let got = admission(&e);
    assert!(
        got.contains(&"Site-A: commitments 12000 kbps exceed ceiling 10000 kbps".to_string()),
        "{got:?}"
    );
    assert!(admission(&fixture_expected()).is_empty());
}

#[test]
fn the_parser_reads_only_fresh_artifacts() {
    let o = read_observed(&fixture()).unwrap();
    assert!(!o.circuits.contains_key("stale-only"));
    assert_eq!(o.circuits.len(), 2);
    assert_eq!(o.nodes.len(), 2);
    assert_eq!(o.circuits["c1"].download.commit_kbps, 4000);
    assert_eq!(o.nodes["n-b"].download.ceil_kbps, 226000);
}

#[test]
fn compare_finds_missing_unexpected_and_changed_entries() {
    let mut o = read_observed(&fixture()).unwrap();
    o.circuits.remove("c1");
    o.nodes.get_mut("n-b").unwrap().download.ceil_kbps = 1;
    o.circuits.insert("extra".into(), o.circuits["c2"].clone());
    let d = compare(&fixture_expected(), &o);
    let all = d.differences.join("\n");
    assert!(all.contains("missing circuit c1"), "{all}");
    assert!(all.contains("unexpected circuit extra"), "{all}");
    assert!(all.contains("node n-b"), "{all}");
}

#[test]
fn a_matching_state_has_no_differences() {
    let d = compare(&fixture_expected(), &read_observed(&fixture()).unwrap());
    assert!(d.differences.is_empty(), "{:?}", d.differences);
    assert_eq!(d.warnings.len(), 2);
}

/// Write one state file of a hand-written LibreQoS state.
fn put(state: &Path, rel: &str, v: Value) {
    let path = state.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, v.to_string()).unwrap();
}

/// The numbers that production showed on 2026-11-19: one 3.5 Mbps circuit gives the circuit
/// commitment 4000 kbps and the node commitment 3000 kbps. The state is written by hand, not by
/// the model, so it catches model errors.
#[test]
fn the_model_matches_the_production_proven_numbers() {
    let net = json!({"Site-P": site("n-p", 100.0, json!({}))});
    let dir = tempfile::tempdir().unwrap();
    let s = dir.path();
    put(
        s,
        "topology/topology_runtime_status.json",
        json!({"ready": true}),
    );
    put(
        s,
        "topology/topology_import.json",
        json!({"imported": {"compatibility_network_json": net}}),
    );
    put(
        s,
        "shaping/topology_compiled_shaping.json",
        json!({"shaped_devices": [{"circuit_id": "c-p"}]}),
    );
    put(
        s,
        "shaping/shaping_inputs.json",
        json!({"shaping_generation": "g", "circuits": [{"circuit_id": "c-p"}]}),
    );
    let r = |ceil: u64, commit: u64| json!({"ceil_kbps": ceil, "commit_kbps": commit});
    put(
        s,
        "shaping/dataplane_transaction.json",
        json!({"generation": "g",
            "inner_ops": [{"node_id": "n-p", "parent_node_id": "root",
                "download": r(100000, 3000), "upload": r(100000, 3000)}],
            "leaf_ops": [{"circuit_id": "c-p", "parent_node_id": "n-p",
                "download": r(10000, 4000), "upload": r(10000, 4000)}]}),
    );
    let want = expected(&[row("c-p", "Site-P", "3.5", "10")], &net).unwrap();
    let d = compare(&want, &read_observed(s).unwrap());
    assert!(d.differences.is_empty(), "{:?}", d.differences);
}
