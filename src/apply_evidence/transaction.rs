//! Schema-6 applied-world comparison. Only independent, unique upsert/map operations may reorder.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn matches(want: &Value, got: &Value) -> bool {
    let Some(_expected_capacity) = want["circuit_capacity"].as_u64().filter(|v| *v > 0) else {
        return false;
    };
    // Allocation capacity is independent from policy content. Preserve it as health evidence.
    let keys = [
        "schema_version",
        "generation",
        "apply_mode",
        "circuit_capacity",
        "inner_ops",
        "leaf_ops",
        "mapping_ops",
        "lpm_delta",
        "steering_vlans",
    ];
    let Some(w) = want.as_object() else {
        return false;
    };
    let Some(g) = got.as_object() else {
        return false;
    };
    if w.len() != keys.len()
        || keys.iter().any(|k| !w.contains_key(*k))
        || g.keys().any(|k| !keys.contains(&k.as_str()))
    {
        return false;
    }
    if want["schema_version"] != 6 || got["schema_version"] != 6 {
        return false;
    }
    for key in ["generation", "apply_mode", "lpm_delta", "steering_vlans"] {
        if !g.contains_key(key) || want[key] != got[key] {
            return false;
        }
    }
    for (key, tag, identity) in [
        ("inner_ops", "upsert", &["node_id"][..]),
        ("leaf_ops", "upsert", &["circuit_id"][..]),
        ("mapping_ops", "map", &["ip", "prefix_len"][..]),
    ] {
        let (Some(a), Some(b)) = (want[key].as_array(), got[key].as_array()) else {
            return false;
        };
        if a.len() != b.len() {
            return false;
        }
        // Preserve sequence for other operation types; do not invent a fold for duplicates.
        if let (Some(left), Some(right)) =
            (independent(a, tag, identity), independent(b, tag, identity))
        {
            if left != right {
                return false;
            }
        } else {
            let tags: BTreeSet<_> = a.iter().map(|v| v["op"].as_str()).collect();
            if tags.len() == 1 && tags.contains(&Some(tag)) {
                return false;
            }
            if a != b {
                return false;
            }
        }
    }
    true
}

fn independent<'a>(
    values: &'a [Value],
    tag: &str,
    identity: &[&str],
) -> Option<BTreeMap<String, &'a Value>> {
    let mut result = BTreeMap::new();
    for item in values {
        if item["op"] != tag {
            return None;
        }
        let mut parts = Vec::new();
        for key in identity {
            let value = item.get(*key)?;
            if value.is_null() {
                return None;
            }
            parts.push(value);
        }
        let key = serde_json::to_string(&parts).ok()?;
        if result.insert(key, item).is_some() {
            return None;
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn generated() -> Value {
        json!({"schema_version":6,"generation":"g1","apply_mode":"incremental","circuit_capacity":1000,
        "inner_ops":[{"op":"upsert","node_id":"n1"},{"op":"upsert","node_id":"n2"}],
        "leaf_ops":[{"op":"upsert","circuit_id":"c1","download":20},{"op":"upsert","circuit_id":"c2","download":10}],
        "mapping_ops":[{"op":"map","circuit_id":"c1","device_id":"d1","ip":"100.64.0.1","prefix_len":32},
            {"op":"map","circuit_id":"c2","device_id":"d2","ip":"100.64.0.2","prefix_len":32}],"lpm_delta":[],"steering_vlans":[]})
    }
    fn observed() -> Value {
        let mut v = generated();
        v.as_object_mut().unwrap().remove("circuit_capacity");
        for k in ["inner_ops", "leaf_ops", "mapping_ops"] {
            v[k].as_array_mut().unwrap().reverse();
        }
        v
    }
    #[test]
    #[ignore = "requires an operator-owned private native capture"]
    fn offline_private_capture_policy_comparison() {
        let capture = std::env::var("LQOSYNC_NATIVE_CAPTURE").expect("capture path required");
        let generated =
            std::env::var("LQOSYNC_GENERATED_TRANSACTION").expect("generated path required");
        let got: Value = serde_json::from_slice(&std::fs::read(capture).unwrap()).unwrap();
        let want: Value = serde_json::from_slice(&std::fs::read(generated).unwrap()).unwrap();
        assert!(
            matches(&want, &got["transaction"]),
            "captured policy comparison failed"
        );
    }
    #[test]
    fn applied_order_and_separate_capacity_match_current_schema() {
        assert!(matches(&generated(), &observed()));
    }
    #[test]
    fn changed_rate_generation_and_classification_cannot_match() {
        for (key, value) in [("generation", json!("old")), ("schema_version", json!(7))] {
            let mut v = observed();
            v[key] = value;
            assert!(!matches(&generated(), &v));
        }
        let mut v = observed();
        v["leaf_ops"][0]["download"] = json!(30);
        assert!(!matches(&generated(), &v));
        let mut v = observed();
        v["mapping_ops"][0]["ip"] = json!("100.64.0.9");
        assert!(!matches(&generated(), &v));
    }
    #[test]
    fn duplicate_operation_identities_and_unknown_fields_fail_closed() {
        for key in ["leaf_ops", "mapping_ops", "inner_ops"] {
            let mut v = observed();
            let item = v[key][0].clone();
            v[key][1] = item;
            assert!(!matches(&generated(), &v));
        }
        let mut v = observed();
        v["future_field"] = json!(true);
        assert!(!matches(&generated(), &v));
    }
}
