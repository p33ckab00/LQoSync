//! Regression test: the `full` and `full2` compile modes must keep the node kind and the
//! node rate of a `libreqos_network.json` (schema 2) input. LibreQoS 3.0.7 loses both:
//! the compiled compatibility tree gets 1 Mbps for each node.

use lqos_topology_compile::{TopologyCompileMode, compile_topology, parse_libreqos_network};
use serde_json::Value;

const INPUT: &str = r#"{
  "schema_version": 2,
  "match": {"mode": "ip"},
  "topology": [
    {"id": "repro-site-a", "name": "SITE-A", "type": "site", "parent": null,
     "rate": {"commit_down_mbps": 200.0, "commit_up_mbps": 100.0, "ceil_down_mbps": 200.0, "ceil_up_mbps": 100.0}},
    {"id": "repro-node-a", "name": "PLAN-A", "type": "node", "parent": "repro-site-a",
     "rate": {"commit_down_mbps": 50.0, "commit_up_mbps": 25.0, "ceil_down_mbps": 50.0, "ceil_up_mbps": 25.0}}
  ],
  "circuits": [
    {"id": "REPROC1", "display_name": "repro-circuit", "parent": "repro-node-a",
     "plan": {"commit_down_mbps": 5.0, "commit_up_mbps": 2.5, "ceil_down_mbps": 50.0, "ceil_up_mbps": 25.0},
     "match": {"prefixes": ["10.255.0.2/32"]},
     "devices": [{"id": "REPROD1", "name": "repro-circuit", "mac": "02:00:00:00:00:01",
                  "ipv4": ["10.255.0.2"], "ipv6": [], "comment": ""}]}
  ]
}"#;

fn find<'a>(tree: &'a Value, id: &str) -> Option<&'a Value> {
    tree.as_object()?.values().find_map(|node| {
        if node.get("id").and_then(Value::as_str) == Some(id) {
            Some(node)
        } else {
            find(node.get("children")?, id)
        }
    })
}

fn check(mode: TopologyCompileMode) {
    let imported = parse_libreqos_network(INPUT)
        .expect("valid schema 2 input")
        .into_imported_bundle("libreqos_network")
        .expect("import");
    // The import keeps the policy: the defect is in the compile step.
    let native = imported.native_canonical.as_ref().expect("native canonical");
    let plan = native.nodes.iter().find(|n| n.node_id == "repro-node-a").unwrap();
    assert_eq!(plan.node_kind, "node");
    assert_eq!(plan.rate_input.intrinsic_download_mbps, Some(50));

    let compiled = compile_topology(imported, mode).expect("compile");
    let node = compiled.canonical.nodes.iter().find(|n| n.node_id == "repro-node-a").unwrap();
    assert_eq!(node.node_kind, "node", "{mode:?}: compiled canonical node kind");
    assert_eq!(
        (node.rate_input.intrinsic_download_mbps, node.rate_input.intrinsic_upload_mbps),
        (Some(50), Some(25)),
        "{mode:?}: compiled canonical node rate"
    );
    for (id, down, up) in [("repro-site-a", 200.0, 100.0), ("repro-node-a", 50.0, 25.0)] {
        let n = find(&compiled.compatibility_network_json, id).expect("node in compatibility tree");
        assert_eq!(
            (n["downloadBandwidthMbps"].as_f64(), n["uploadBandwidthMbps"].as_f64()),
            (Some(down), Some(up)),
            "{mode:?}: compatibility tree rate of {id}"
        );
    }
}

#[test]
fn full_keeps_native_node_kind_and_rate() {
    check(TopologyCompileMode::Full);
}

#[test]
fn full2_keeps_native_node_kind_and_rate() {
    check(TopologyCompileMode::Full2);
}
