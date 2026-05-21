use crate::protocol::Diagnostic;
use crate::routeros_live_read_adapter::run_routeros_live_read_adapter_pilot_payload;
use crate::routeros_shadow_bundle::build_routeros_shadow_collector_bundle_payload;
use serde_json::{json, Value};

fn bool_value(v: Option<&Value>, default: bool) -> bool {
    v.and_then(Value::as_bool).unwrap_or(default)
}

fn str_value<'a>(v: Option<&'a Value>, default: &'a str) -> &'a str {
    v.and_then(Value::as_str).unwrap_or(default)
}

fn merge_diags(target: &mut Vec<Diagnostic>, mut source: Vec<Diagnostic>) {
    target.append(&mut source);
}

fn as_result_entries(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items.iter().filter(|v| v.is_object()).cloned().collect(),
        Value::Object(map) => map.values().filter(|v| v.is_object()).cloned().collect(),
        _ => Vec::new(),
    }
}

fn append_result_entries(target: &mut Vec<Value>, value: Option<&Value>) {
    if let Some(value) = value {
        target.extend(as_result_entries(value));
    }
}

fn append_live_read_result(target: &mut Vec<Value>, value: Option<&Value>) {
    let Some(value) = value else { return; };
    if let Some(result) = value.get("result") {
        append_live_read_result(target, Some(result));
    }
    if let Some(read_result) = value.get("read_result").filter(|v| v.is_object()) {
        target.push(read_result.clone());
        return;
    }
    if value.get("path").is_some() && value.get("rows").is_some() {
        target.push(value.clone());
    }
}

fn supplied_live_adapter(payload: &Value) -> Option<Value> {
    payload
        .get("live_read_adapter")
        .or_else(|| payload.get("adapter_result"))
        .and_then(|value| {
            value
                .get("result")
                .filter(|v| v.is_object())
                .or(Some(value))
        })
        .filter(|v| v.is_object())
        .cloned()
}

fn collect_read_results(payload: &Value, live_adapter: &Value) -> Vec<Value> {
    let mut results = Vec::new();
    append_result_entries(&mut results, payload.get("results"));
    append_result_entries(&mut results, payload.get("read_results"));
    append_result_entries(&mut results, payload.get("live_read_results"));
    append_live_read_result(&mut results, payload.get("live_read_result"));
    append_live_read_result(&mut results, payload.get("read_result"));
    append_live_read_result(&mut results, Some(live_adapter));
    results
}

fn value_count(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// Build a non-authoritative live-read shadow parity bundle.
///
/// This composes the gated read-only live adapter with the existing RouterOS
/// shadow collector bundle. It may consume supplied live-read results or run the
/// live adapter if requested, but it never transfers cleanup/apply/collector
/// authority away from Python.
pub fn build_routeros_live_read_shadow_parity_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let execute = bool_value(payload.get("execute"), false);
    let mode = str_value(payload.get("mode"), "shadow_parity");
    let supplied_results_present = !collect_read_results(payload, &json!({})).is_empty();
    let supplied_adapter = supplied_live_adapter(payload);

    let live_adapter = if let Some(adapter) = supplied_adapter {
        adapter
    } else if supplied_results_present {
        json!({
            "status": "supplied_live_read_results",
            "collector_authority": "python_authoritative",
            "safe_for_cleanup": false,
            "connection_attempt_count": 0,
            "authentication_attempt_count": 0,
            "api_sentence_write_count": 0,
            "api_reply_read_count": 0
        })
    } else {
        let (adapter, adapter_errors, adapter_warnings) = run_routeros_live_read_adapter_pilot_payload(payload);
        merge_diags(&mut errors, adapter_errors);
        merge_diags(&mut warnings, adapter_warnings);
        adapter
    };

    let read_results = collect_read_results(payload, &live_adapter);
    let live_read_result_count = read_results.len() as u64;
    let mut shadow_bundle = json!({
        "mode": "routeros_shadow_collector_bundle",
        "status": "not_run",
        "normalized_count": 0,
        "parity": {"verdict":"not_run", "parity_score":0.0},
        "reason": "live_read_results_not_supplied"
    });

    if !read_results.is_empty() {
        let mut bundle_payload = payload.clone();
        if let Value::Object(ref mut map) = bundle_payload {
            map.insert("results".to_string(), Value::Array(read_results.clone()));
        }
        let (bundle, bundle_errors, bundle_warnings) = build_routeros_shadow_collector_bundle_payload(&bundle_payload);
        shadow_bundle = bundle;
        merge_diags(&mut errors, bundle_errors);
        merge_diags(&mut warnings, bundle_warnings);
    } else if execute {
        warnings.push(Diagnostic::warning(
            "live_shadow_parity_no_read_results",
            Some("live_read_results".to_string()),
            "Live-read shadow parity was requested, but no trusted RouterOS read results were available to bundle.",
        ));
    }

    let live_adapter_status = live_adapter.get("status").and_then(Value::as_str).unwrap_or("not_run").to_string();
    let live_read_complete = live_adapter_status == "live_read_adapter_read_complete"
        || live_adapter_status == "supplied_live_read_results";
    let shadow_status = shadow_bundle.get("status").and_then(Value::as_str).unwrap_or("not_run").to_string();
    let shadow_ready = shadow_status == "shadow_ready";
    let parity = shadow_bundle.get("parity").cloned().unwrap_or_else(|| json!({"verdict":"not_run", "parity_score":0.0}));
    let parity_verdict = parity.get("verdict").and_then(Value::as_str).unwrap_or("not_run");
    let parity_pass = parity_verdict == "parity_pass";
    let normalized_count = shadow_bundle.get("normalized_count").cloned().unwrap_or_else(|| json!(0));
    let connection_attempt_count = value_count(&live_adapter, "connection_attempt_count");
    let authentication_attempt_count = value_count(&live_adapter, "authentication_attempt_count");
    let api_sentence_write_count = value_count(&live_adapter, "api_sentence_write_count");
    let api_reply_read_count = value_count(&live_adapter, "api_reply_read_count");

    let status = if !errors.is_empty() {
        "blocked"
    } else if live_read_complete && shadow_ready && parity_pass {
        "live_read_shadow_parity_pass"
    } else if live_read_complete && shadow_ready {
        "live_read_shadow_parity_review"
    } else if live_read_result_count > 0 {
        "live_read_shadow_bundle_partial"
    } else if live_adapter_status == "live_read_adapter_contract_ready" {
        "live_read_shadow_contract_ready"
    } else {
        "live_read_shadow_waiting_for_results"
    };

    let result = json!({
        "mode": "routeros_live_read_shadow_parity",
        "status": status,
        "requested_mode": mode,
        "execute_requested": execute,
        "authoritative": false,
        "collector_authority": "python_authoritative",
        "production_authority": "python_collector",
        "full_rust_backend": false,
        "live_adapter_status": live_adapter_status,
        "live_adapter": live_adapter,
        "live_read_result_count": live_read_result_count,
        "shadow_bundle_ready": shadow_ready,
        "shadow_bundle": shadow_bundle,
        "normalized_count": normalized_count,
        "parity": parity,
        "parity_ready": parity_pass,
        "collector_output_can_drive_cleanup": false,
        "collector_output_can_drive_apply": false,
        "safe_for_cleanup": false,
        "write_allowed": false,
        "apply_allowed": false,
        "connection_attempt_count": connection_attempt_count,
        "authentication_attempt_count": authentication_attempt_count,
        "api_sentence_write_count": api_sentence_write_count,
        "api_reply_read_count": api_reply_read_count,
        "next_stage": "rust_live_read_shadow_run_cycle_integration",
        "note": "Rust can now turn gated live-read pilot results into shadow collector rows and parity evidence. Python collectors remain authoritative until repeated parity gates pass."
    });

    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn full_pppoe_payload() -> Value {
        let python_rows = json!([
            {"Circuit ID":"JUAN", "Circuit Name":"juan", "Device ID":"JUANDEV", "Device Name":"juan", "Parent Node":"15M-RB5009", "MAC":"AA:BB:CC:DD:EE:FF", "IPv4":"10.0.0.2", "IPv6":"", "Download Min Mbps":"7.5", "Upload Min Mbps":"7.5", "Download Max Mbps":"15", "Upload Max Mbps":"15", "Comment":"PPP"}
        ]);
        json!({
            "config": {
                "defaults": {"default_pppoe_rate":"10M/10M", "min_rate_percentage":0.5},
                "routers": [{"name":"RB5009", "enabled": true, "pppoe":{"enabled":true, "per_plan_node":true}}]
            },
            "live_read_adapter": {"result": {"status":"live_read_adapter_read_complete", "router":"RB5009", "source":"pppoe", "path":"/ppp/active", "read_result": {"router":"RB5009", "source":"pppoe", "path":"/ppp/active", "status":"ok", "rows":[{"name":"juan", "address":"10.0.0.2", "caller-id":"AA:BB:CC:DD:EE:FF"}]}, "connection_attempt_count":1, "authentication_attempt_count":1, "api_sentence_write_count":2, "api_reply_read_count":2}},
            "live_read_results": [
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/secret", "status":"ok", "rows":[{"name":"juan", "profile":"15M", "comment":"PLAN|15M/15M", "disabled":"false", "inactive":"false"}]},
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/profile", "status":"ok", "rows":[{"name":"15M", "rate-limit":"15M/15M"}]}
            ],
            "python_rows": python_rows
        })
    }

    #[test]
    fn defaults_to_contract_without_network() {
        let leaked_password = "do-not-leak-live-shadow-password";
        let payload = json!({
            "router": {"name":"R1", "address":"10.0.0.1", "username":"admin", "password": leaked_password},
            "adapter": "contract",
            "mode": "contract",
            "execute": false,
            "fixture_reply_words": ["!done"]
        });
        let (result, errors, _warnings) = build_routeros_live_read_shadow_parity_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("live_read_shadow_contract_ready"));
        assert_eq!(result.get("connection_attempt_count").and_then(Value::as_u64), Some(0));
        let text = serde_json::to_string(&result).unwrap();
        assert!(!text.contains(leaked_password));
    }

    #[test]
    fn builds_shadow_parity_from_live_adapter_result() {
        let payload = full_pppoe_payload();
        let (result, errors, _warnings) = build_routeros_live_read_shadow_parity_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("live_read_shadow_parity_pass"));
        assert_eq!(result.get("live_read_result_count").and_then(Value::as_u64), Some(3));
        assert_eq!(result.get("normalized_count").and_then(Value::as_u64), Some(1));
        assert_eq!(result["parity"]["verdict"], "parity_pass");
        assert_eq!(result.get("collector_authority").and_then(Value::as_str), Some("python_authoritative"));
        assert_eq!(result.get("safe_for_cleanup").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn blocks_when_required_live_read_results_are_missing() {
        let payload = json!({
            "config": {"routers": [{"name":"RB5009", "enabled": true, "pppoe":{"enabled":true}}]},
            "live_read_adapter": {"result": {"status":"live_read_adapter_read_complete", "read_result": {"router":"RB5009", "source":"pppoe", "path":"/ppp/active", "status":"ok", "rows":[{"name":"juan", "address":"10.0.0.2"}]}}}
        });
        let (result, errors, _warnings) = build_routeros_live_read_shadow_parity_payload(&payload);
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
        assert!(errors.iter().any(|e| e.code == "routeros_required_read_missing"));
        assert_eq!(result.get("collector_authority").and_then(Value::as_str), Some("python_authoritative"));
    }
}
