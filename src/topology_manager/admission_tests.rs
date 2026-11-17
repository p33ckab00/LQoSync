use super::*;
use crate::topology_manager::publication::{prepare, tests::input};
use serde_json::json;

pub(crate) fn artifacts(pair: &PreparedPair) -> (Value, Value) {
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
        nodes.push(json!({"node_id":node["id"],"node_name":name,"node_kind":node["type"],"can_move":false,"allowed_parents":[],"current_attachment_id":parent.map(|(_,id)|id),"current_attachment_name":parent.map(|(name,_)|name),"current_parent_node_id":parent.map(|(_,id)|id),"current_parent_node_name":parent.map(|(name,_)|name),"rate_input":{"intrinsic_download_mbps":node["downloadBandwidthMbps"],"intrinsic_upload_mbps":node["uploadBandwidthMbps"],"source":"imported_network_json"}}));
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
            .map(|s| json!([s, width]))
            .collect::<Vec<_>>()
    };
    let devices:Vec<_>=pair.rows().iter().map(|r|json!({"circuit_id":r.circuit_id,"circuit_name":r.circuit_name,"device_id":r.device_id,"device_name":r.device_name,"parent_node":r.parent_node,"parent_node_id":r.parent_node_id,"anchor_node_id":null,"mac":r.mac,"ipv4":addresses(&r.ipv4,32),"ipv6":addresses(&r.ipv6,128),"download_min_mbps":r.download_min.parse::<f32>().unwrap(),"upload_min_mbps":r.upload_min.parse::<f32>().unwrap(),"download_max_mbps":r.download_max.parse::<f32>().unwrap(),"upload_max_mbps":r.upload_max.parse::<f32>().unwrap(),"comment":r.comment})).collect();
    (
        json!({"schema_version":1,"source":"legacy/manual","ingress_identity":"a".repeat(64),"ingress_kind":"native_integration","nodes":nodes,"compatibility_network_json":pair.network()}),
        json!({"schema_version":1,"source":"legacy/manual","compile_mode":"full","ingress_identity":"a".repeat(64),"shaped_devices":devices,"circuit_anchors":{"schema_version":0,"source":"","anchors":[]},"circuit_matches":{}}),
    )
}
#[tokio::test(start_paused = true)]
async fn exact_compilation_admits_both_active_and_empty_candidates() {
    for count in [0, 2] {
        let pair = prepare(input(count)).unwrap();
        let (c, s) = artifacts(&pair);
        check_compiled(pair, &c, &s).unwrap();
    }
}
#[tokio::test(start_paused = true)]
async fn success_with_clamped_node_or_subscriber_rates_is_rejected() {
    for field in 0..3 {
        let pair = prepare(input(2)).unwrap();
        let (mut c, mut s) = artifacts(&pair);
        match field {
            0 => c["nodes"][0]["rate_input"]["intrinsic_download_mbps"] = 1.into(),
            1 => s["shaped_devices"][0]["download_max_mbps"] = 49.into(),
            _ => s["shaped_devices"][0]["download_min_mbps"] = 2.into(),
        }
        assert!(check_compiled(pair, &c, &s).is_err());
    }
}
#[tokio::test(start_paused = true)]
async fn wrong_ingress_parent_or_missing_and_duplicate_objects_are_rejected() {
    for field in 0..5 {
        let pair = prepare(input(2)).unwrap();
        let (mut c, mut s) = artifacts(&pair);
        match field {
            0 => s["ingress_identity"] = "b".repeat(64).into(),
            1 => c["nodes"][0]["current_parent_node_id"] = "other".into(),
            2 => {
                c["nodes"].as_array_mut().unwrap().pop();
            }
            3 => {
                let n = c["nodes"][0].clone();
                c["nodes"].as_array_mut().unwrap().push(n);
            }
            _ => {
                s["shaped_devices"].as_array_mut().unwrap().pop();
            }
        }
        assert!(check_compiled(pair, &c, &s).is_err());
    }
}
#[tokio::test(start_paused = true)]
async fn another_pair_or_expired_source_cannot_use_compilation_evidence() {
    let original = prepare(input(1)).unwrap();
    let (c, s) = artifacts(&original);
    assert!(check_compiled(prepare(input(2)).unwrap(), &c, &s).is_err());
    tokio::time::advance(std::time::Duration::from_secs(11)).await;
    assert!(check_compiled(original, &c, &s).is_err());
}

#[test]
fn byte_budgets_reject_overflow_and_artifact_encoding_without_unbounded_copies() {
    check_pair_size(MAX_PAIR_BYTES, 0).unwrap();
    assert!(check_pair_size(MAX_PAIR_BYTES, 1).is_err());
    assert!(check_pair_size(usize::MAX, 1).is_err());
    let mut budget = Budget(2);
    assert!(serde_json::to_writer(&mut budget, &json!("too long")).is_err());
}

#[tokio::test(start_paused = true)]
async fn unsupported_schema_or_matching_extensions_cannot_pass_compilation_admission() {
    for field in 0..3 {
        let pair = prepare(input(1)).unwrap();
        let (mut canonical, mut shaping) = artifacts(&pair);
        match field {
            0 => canonical["schema_version"] = 99.into(),
            1 => shaping["circuit_matches"] = json!({"unexpected":true}),
            _ => shaping["circuit_anchors"]["anchors"] = json!(["unexpected"]),
        }
        assert!(check_compiled(pair, &canonical, &shaping).is_err());
    }
}

#[tokio::test(start_paused = true)]
async fn compiler_cannot_enable_unrequested_moves_or_attachments() {
    for field in 0..3 {
        let pair = prepare(input(1)).unwrap();
        let (mut canonical, shaping) = artifacts(&pair);
        match field {
            0 => canonical["nodes"][0]["can_move"] = true.into(),
            1 => canonical["nodes"][0]["allowed_parents"] = json!(["other"]),
            _ => canonical["nodes"][0]["current_attachment_id"] = "other".into(),
        }
        assert!(check_compiled(pair, &canonical, &shaping).is_err());
    }
}

#[tokio::test(start_paused = true)]
async fn offline_diagnostics_reject_duplicate_requested_node_ids() {
    let pair = prepare(input(0)).unwrap();
    let (mut canonical, shaping) = artifacts(&pair);
    let mut network = pair.network().clone();
    let keys: Vec<_> = network.as_object().unwrap().keys().cloned().collect();
    let second_id = network[&keys[1]]["id"].clone();
    network[&keys[0]]["id"] = second_id;
    canonical["compatibility_network_json"] = network.clone();
    canonical["nodes"].as_array_mut().unwrap().remove(0);
    assert!(
        check_artifacts(
            &crate::writer::pair::render_network_json(&network),
            pair.csv(),
            &canonical,
            &shaping
        )
        .is_err()
    );
}

#[tokio::test(start_paused = true)]
async fn dropped_or_changed_explicit_node_minimums_cannot_pass_compiler_admission() {
    for drop_minimum in [true, false] {
        let mut observed = input(1);
        observed
            .reconciliation
            .registry
            .catalog
            .as_mut()
            .unwrap()
            .nodes[0]
            .minimum
            .down = Some(2_000_000);
        let pair = prepare(observed).unwrap();
        let (mut c, s) = artifacts(&pair);
        let root = c["compatibility_network_json"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .find(|node| node.get("downloadBandwidthMbpsMin").is_some())
            .unwrap();
        if drop_minimum {
            root.as_object_mut()
                .unwrap()
                .remove("downloadBandwidthMbpsMin");
        } else {
            root["downloadBandwidthMbpsMin"] = 1.into();
        }
        assert!(check_compiled(pair, &c, &s).is_err());
    }
}
