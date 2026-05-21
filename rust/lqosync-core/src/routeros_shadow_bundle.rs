use crate::collector_bundle::build_collector_circuit_bundle_payload;
use crate::collector_parity::compare_collector_bundle_parity_payload;
use crate::protocol::Diagnostic;
use crate::routeros_plan::build_routeros_collector_plan_payload;
use crate::routeros_results::validate_routeros_read_results_payload;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn as_rows(value: Option<&Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(rows)) => rows.iter().filter(|v| v.is_object()).cloned().collect(),
        _ => Vec::new(),
    }
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

fn result_entries(payload: &Value) -> Vec<Value> {
    let mut entries = Vec::new();
    if let Some(results) = payload.get("results") {
        append_result_entries(&mut entries, Some(results));
    }
    if let Some(results) = payload.get("read_results") {
        append_result_entries(&mut entries, Some(results));
    }
    if let Some(results) = payload
        .get("routeros_results")
        .and_then(|v| v.get("results"))
    {
        append_result_entries(&mut entries, Some(results));
    }
    append_result_entries(&mut entries, payload.get("live_read_results"));
    append_live_read_result(&mut entries, payload.get("live_read_result"));
    append_live_read_result(&mut entries, payload.get("read_result"));
    append_live_read_result(&mut entries, payload.get("live_read_adapter"));
    append_live_read_result(&mut entries, payload.get("adapter_result"));
    entries
}

fn merge_diags(target: &mut Vec<Diagnostic>, mut source: Vec<Diagnostic>) {
    target.append(&mut source);
}

fn command_key(router: &str, source: &str, path: &str) -> String {
    format!("{router}|{source}|{path}")
}

fn router_name(router: &Value) -> String {
    router
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("Router")
        .to_string()
}

fn config_from_payload(payload: &Value) -> &Value {
    payload.get("config").unwrap_or(payload)
}

fn routers_from_config(config: &Value) -> Vec<Value> {
    config
        .get("routers")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter(|v| v.is_object()).cloned().collect())
        .unwrap_or_default()
}

fn rows_by_command(results: &[Value]) -> BTreeMap<String, Vec<Value>> {
    let mut out: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for item in results {
        let router = item
            .get("router")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let source = item
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let path = item.get("path").and_then(Value::as_str).unwrap_or("");
        if path.is_empty() {
            continue;
        }
        let status = item.get("status").and_then(Value::as_str).unwrap_or("ok");
        if status != "ok" && status != "zero_valid" {
            continue;
        }
        out.insert(command_key(router, source, path), as_rows(item.get("rows")));
    }
    out
}

fn command_rows(
    grouped: &BTreeMap<String, Vec<Value>>,
    router: &str,
    source: &str,
    path: &str,
) -> Vec<Value> {
    grouped
        .get(&command_key(router, source, path))
        .cloned()
        .unwrap_or_default()
}

fn router_has_any_result(results: &[Value], router: &str) -> bool {
    results
        .iter()
        .any(|r| r.get("router").and_then(Value::as_str).unwrap_or("") == router)
}

fn source_counts_add(target: &mut serde_json::Map<String, Value>, source_counts: &Value) {
    if let Some(map) = source_counts.as_object() {
        for (key, value) in map {
            let current = target.get(key).and_then(Value::as_u64).unwrap_or(0);
            target.insert(key.clone(), json!(current + value.as_u64().unwrap_or(0)));
        }
    }
}

/// Build Rust shadow collector rows from RouterOS read results.
///
/// This operation is the bridge between the RouterOS read contract and the Rust
/// collector bundle builder. It validates command-level read results, groups
/// trusted PPPoE/DHCP/Hotspot snapshots by router, builds normalized
/// ShapedDevices rows in shadow mode, and optionally compares them with Python
/// authoritative rows. It never opens sockets and never becomes cleanup/write
/// authority.
pub fn build_routeros_shadow_collector_bundle_payload(
    payload: &Value,
) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let config = config_from_payload(payload);

    let plan = payload.get("plan").cloned().unwrap_or_else(|| {
        let (plan, plan_errors, plan_warnings) = build_routeros_collector_plan_payload(payload);
        merge_diags(&mut errors, plan_errors);
        merge_diags(&mut warnings, plan_warnings);
        plan
    });

    let results = result_entries(payload);
    let validation_payload = json!({
        "plan": plan,
        "results": results.clone(),
        "previous_counts": payload.get("previous_counts").cloned().unwrap_or_else(|| json!({})),
        "slow_ms_threshold": payload.get("slow_ms_threshold").cloned().unwrap_or_else(|| json!(2000.0)),
        "strict": payload.get("strict_read_results").cloned().unwrap_or_else(|| json!(false))
    });
    let (read_validation, read_errors, read_warnings) =
        validate_routeros_read_results_payload(&validation_payload);
    merge_diags(&mut errors, read_errors);
    merge_diags(&mut warnings, read_warnings);

    let grouped = rows_by_command(&results);
    let defaults = config.get("defaults").cloned().unwrap_or_else(|| json!({}));
    let mut normalized_rows: Vec<Value> = Vec::new();
    let mut router_bundles: Vec<Value> = Vec::new();
    let mut aggregate_source_counts = serde_json::Map::new();
    let mut bundle_count = 0u64;

    for router in routers_from_config(config) {
        let name = router_name(&router);
        if !router_has_any_result(&results, &name) {
            continue;
        }
        let bundle_payload = json!({
            "router": router,
            "defaults": defaults.clone(),
            "pppoe": {
                "active": command_rows(&grouped, &name, "pppoe", "/ppp/active"),
                "secrets": command_rows(&grouped, &name, "pppoe", "/ppp/secret"),
                "profiles": command_rows(&grouped, &name, "pppoe", "/ppp/profile")
            },
            "dhcp": {
                "leases": command_rows(&grouped, &name, "dhcp", "/ip/dhcp-server/lease"),
                "servers": command_rows(&grouped, &name, "dhcp", "/ip/dhcp-server")
            },
            "hotspot": {
                "active": command_rows(&grouped, &name, "hotspot", "/ip/hotspot/active"),
                "users": command_rows(&grouped, &name, "hotspot", "/ip/hotspot/user"),
                "profiles": command_rows(&grouped, &name, "hotspot", "/ip/hotspot/user/profile")
            }
        });
        let (bundle, bundle_errors, bundle_warnings) =
            build_collector_circuit_bundle_payload(&bundle_payload);
        merge_diags(&mut errors, bundle_errors);
        merge_diags(&mut warnings, bundle_warnings);
        source_counts_add(
            &mut aggregate_source_counts,
            bundle.get("source_counts").unwrap_or(&Value::Null),
        );
        if let Some(rows) = bundle.get("normalized_rows").and_then(Value::as_array) {
            normalized_rows.extend(rows.iter().cloned());
        }
        router_bundles.push(json!({
            "router": name,
            "normalized_count": bundle.get("normalized_count").cloned().unwrap_or_else(|| json!(0)),
            "source_counts": bundle.get("source_counts").cloned().unwrap_or_else(|| json!({})),
            "warning_count": bundle.get("warning_count").cloned().unwrap_or_else(|| json!(0)),
            "mode": bundle.get("mode").cloned().unwrap_or_else(|| json!("shadow"))
        }));
        bundle_count += 1;
    }

    if bundle_count == 0 && !results.is_empty() {
        warnings.push(Diagnostic::warning(
            "routeros_shadow_bundle_no_router_match",
            Some("config.routers".to_string()),
            "RouterOS read results were present, but no configured router name matched them for Rust shadow bundling.",
        ));
    }

    let parity = if payload.get("python_rows").is_some() {
        let (parity, parity_errors, parity_warnings) =
            compare_collector_bundle_parity_payload(&json!({
                "python_rows": payload.get("python_rows").cloned().unwrap_or_else(|| json!([])),
                "rust_rows": normalized_rows.clone(),
                "strict": payload.get("strict_parity").cloned().unwrap_or_else(|| json!(false)),
                "compare_fields": payload.get("compare_fields").cloned().unwrap_or(Value::Null)
            }));
        merge_diags(&mut errors, parity_errors);
        merge_diags(&mut warnings, parity_warnings);
        parity
    } else {
        json!({"mode":"collector_bundle_parity_shadow", "verdict":"not_run", "reason":"python_rows_not_supplied"})
    };

    let read_safe = read_validation
        .get("safe_for_cleanup")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && errors.is_empty();
    let status = if !errors.is_empty() {
        "blocked"
    } else if normalized_rows.is_empty() {
        "shadow_empty"
    } else {
        "shadow_ready"
    };

    let normalized_count = normalized_rows.len();
    let result = json!({
        "mode": "routeros_shadow_collector_bundle",
        "status": status,
        "authoritative": false,
        "python_authoritative": true,
        "full_rust_backend": false,
        "live_transport_supported": false,
        "connection_attempt_count": 0,
        "read_validation": read_validation,
        "read_safe_for_cleanup": read_safe,
        "safe_for_cleanup": false,
        "cleanup_authority": "python_authoritative",
        "router_bundle_count": bundle_count,
        "normalized_count": normalized_count,
        "source_counts": Value::Object(aggregate_source_counts),
        "normalized_rows": normalized_rows,
        "router_bundles": router_bundles,
        "parity": parity,
        "next_stage": "rust_routeros_live_read_shadow_parity",
        "note": "Rust built collector rows from RouterOS read results in shadow mode only. Python collectors remain authoritative until live-read and bundle parity gates pass."
    });

    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_shadow_bundle_from_routeros_read_results() {
        let payload = json!({
            "config": {
                "defaults": {"default_pppoe_rate":"10M/10M", "default_dhcp_per_client_mbps":15, "default_hotspot_per_client_mbps":10, "min_rate_percentage":0.5},
                "routers": [{
                    "name":"RB5009",
                    "enabled": true,
                    "pppoe":{"enabled":true, "per_plan_node":true, "plan_node_name":"{profile}-{router}"},
                    "dhcp":{"enabled":true, "servers":[{"name":"LAN", "enabled":true, "download_limit_mbps":20, "upload_limit_mbps":10}]},
                    "hotspot":{"enabled":true, "download_limit_mbps":5, "upload_limit_mbps":5, "include_mac":true}
                }]
            },
            "results": [
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/active", "status":"ok", "rows":[{"name":"juan", "address":"10.0.0.2", "caller-id":"AA:BB:CC:DD:EE:FF"}]},
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/secret", "status":"ok", "rows":[{"name":"juan", "profile":"15M", "comment":"PLAN|15M/15M", "disabled":"false", "inactive":"false"}]},
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/profile", "status":"ok", "rows":[{"name":"15M", "rate-limit":"15M/15M"}]},
                {"router":"RB5009", "source":"dhcp", "path":"/ip/dhcp-server/lease", "status":"ok", "rows":[{"server":"LAN", "host-name":"phone", "mac-address":"11:22:33:44:55:66", "active-address":"192.168.1.20", "address":"192.168.1.20"}]},
                {"router":"RB5009", "source":"dhcp", "path":"/ip/dhcp-server", "status":"ok", "rows":[{"name":"LAN", "interface":"bridge"}]},
                {"router":"RB5009", "source":"hotspot", "path":"/ip/hotspot/active", "status":"ok", "rows":[{"user":"guest", "address":"172.16.0.10", "mac-address":"AA-BB-CC-00-11-22"}]},
                {"router":"RB5009", "source":"hotspot", "path":"/ip/hotspot/user", "status":"ok", "rows":[]},
                {"router":"RB5009", "source":"hotspot", "path":"/ip/hotspot/user/profile", "status":"ok", "rows":[]}
            ]
        });
        let (result, errors, warnings) = build_routeros_shadow_collector_bundle_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            result.get("status").and_then(Value::as_str),
            Some("shadow_ready")
        );
        assert_eq!(
            result.get("normalized_count").and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            result.get("safe_for_cleanup").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            result.get("python_authoritative").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(result["source_counts"]["PPP"], 1);
        assert_eq!(result["source_counts"]["DHCP"], 1);
        assert_eq!(result["source_counts"]["HS"], 1);
        assert_eq!(result["normalized_rows"][1]["IPv4"], "192.168.1.20");
    }

    #[test]
    fn blocks_shadow_bundle_when_required_routeros_read_is_missing() {
        let payload = json!({
            "config": {"routers": [{"name":"RB5009", "enabled": true, "pppoe":{"enabled":true}}]},
            "results": [
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/active", "status":"ok", "rows":[{"name":"juan", "address":"10.0.0.2"}]}
            ]
        });
        let (result, errors, _warnings) = build_routeros_shadow_collector_bundle_payload(&payload);
        assert!(!errors.is_empty());
        assert_eq!(
            result.get("status").and_then(Value::as_str),
            Some("blocked")
        );
        assert!(errors
            .iter()
            .any(|e| e.code == "routeros_required_read_missing"));
        assert_eq!(
            result.get("safe_for_cleanup").and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn accepts_live_read_adapter_result_as_shadow_input() {
        let python_rows = json!([
            {"Circuit ID":"JUAN", "Circuit Name":"juan", "Device ID":"JUANDEV", "Device Name":"juan", "Parent Node":"15M-RB5009", "MAC":"AA:BB:CC:DD:EE:FF", "IPv4":"10.0.0.2", "IPv6":"", "Download Min Mbps":"7.5", "Upload Min Mbps":"7.5", "Download Max Mbps":"15", "Upload Max Mbps":"15", "Comment":"PPP"}
        ]);
        let payload = json!({
            "config": {
                "defaults": {"default_pppoe_rate":"10M/10M", "min_rate_percentage":0.5},
                "routers": [{"name":"RB5009", "enabled": true, "pppoe":{"enabled":true, "per_plan_node":true}}]
            },
            "live_read_adapter": {"result": {"status":"live_read_adapter_read_complete", "read_result": {"router":"RB5009", "source":"pppoe", "path":"/ppp/active", "status":"ok", "rows":[{"name":"juan", "address":"10.0.0.2", "caller-id":"AA:BB:CC:DD:EE:FF"}]}}},
            "live_read_results": [
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/secret", "status":"ok", "rows":[{"name":"juan", "profile":"15M", "comment":"PLAN|15M/15M", "disabled":"false", "inactive":"false"}]},
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/profile", "status":"ok", "rows":[{"name":"15M", "rate-limit":"15M/15M"}]}
            ],
            "python_rows": python_rows
        });
        let (result, errors, _warnings) = build_routeros_shadow_collector_bundle_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("shadow_ready"));
        assert_eq!(result.get("normalized_count").and_then(Value::as_u64), Some(1));
        assert_eq!(result["parity"]["verdict"], "parity_pass");
        assert_eq!(result["normalized_rows"][0]["Circuit Name"], "juan");
    }
}
