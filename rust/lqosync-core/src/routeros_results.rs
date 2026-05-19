use crate::protocol::Diagnostic;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

fn as_bool(value: Option<&Value>, default: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(default)
}

fn as_f64(value: Option<&Value>, default: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(default)
}

fn as_rows(value: Option<&Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(rows)) => rows.clone(),
        _ => Vec::new(),
    }
}

fn command_key(router: &str, source: &str, path: &str) -> String {
    format!("{}|{}|{}", router, source, path)
}

fn str_field<'a>(obj: &'a serde_json::Map<String, Value>, key: &str, default: &'a str) -> &'a str {
    obj.get(key).and_then(Value::as_str).unwrap_or(default)
}

fn normalize_result_entries(results_value: &Value) -> Vec<Value> {
    match results_value {
        Value::Array(items) => items.clone(),
        Value::Object(map) => map.values().cloned().collect(),
        _ => Vec::new(),
    }
}

fn snapshot_name(source: &str, path: &str) -> String {
    match (source, path) {
        ("pppoe", "/ppp/active") => "pppoe.active".to_string(),
        ("pppoe", "/ppp/secret") => "pppoe.secrets".to_string(),
        ("pppoe", "/ppp/profile") => "pppoe.profiles".to_string(),
        ("dhcp", "/ip/dhcp-server/lease") => "dhcp.leases".to_string(),
        ("dhcp", "/ip/dhcp-server") => "dhcp.servers".to_string(),
        ("hotspot", "/ip/hotspot/active") => "hotspot.active".to_string(),
        ("hotspot", "/ip/hotspot/user") => "hotspot.users".to_string(),
        ("hotspot", "/ip/hotspot/user/profile") => "hotspot.profiles".to_string(),
        _ => format!("{}.{}", source, path.trim_matches('/').replace('/', ".")),
    }
}

pub fn validate_routeros_read_results_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let plan_result = payload
        .get("plan")
        .and_then(|p| p.get("result").or(Some(p)))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let commands = plan_result.get("commands").and_then(Value::as_array).cloned().unwrap_or_default();
    let results_value = payload.get("results").cloned().unwrap_or_else(|| json!([]));
    let results = normalize_result_entries(&results_value);
    let previous_counts = payload.get("previous_counts").cloned().unwrap_or_else(|| json!({}));
    let slow_ms = as_f64(payload.get("slow_ms_threshold"), 2000.0);
    let strict = as_bool(payload.get("strict"), false);

    if commands.is_empty() {
        warnings.push(Diagnostic::warning(
            "routeros_results_no_plan_commands",
            Some("plan.commands".to_string()),
            "No RouterOS collector plan commands were provided; result trust can only be partial.",
        ));
    }

    let mut expected: BTreeMap<String, Value> = BTreeMap::new();
    let mut required_keys = BTreeSet::new();
    for cmd in &commands {
        if let Some(obj) = cmd.as_object() {
            let router = str_field(obj, "router", "unknown");
            let source = str_field(obj, "source", "unknown");
            let path = str_field(obj, "path", "");
            if path.is_empty() { continue; }
            let key = command_key(router, source, path);
            if as_bool(obj.get("required"), true) { required_keys.insert(key.clone()); }
            expected.insert(key, cmd.clone());
        }
    }

    let mut seen = BTreeSet::new();
    let mut command_reports: Vec<Value> = Vec::new();
    let mut snapshots = serde_json::Map::new();
    let mut source_map: BTreeMap<String, (u64, bool, Vec<String>)> = BTreeMap::new();
    let mut failed_count = 0u64;
    let mut missing_count = 0u64;
    let mut slow_count = 0u64;
    let mut suspicious_zero_count = 0u64;
    let mut total_rows = 0u64;

    for item in results {
        let obj = match item.as_object() { Some(o) => o, None => continue };
        let router = str_field(obj, "router", "unknown");
        let source = str_field(obj, "source", "unknown");
        let path = str_field(obj, "path", "");
        let status = str_field(obj, "status", "ok");
        let key = command_key(router, source, path);
        seen.insert(key.clone());
        let expected_cmd = expected.get(&key);
        let required = expected_cmd
            .and_then(Value::as_object)
            .map(|o| as_bool(o.get("required"), true))
            .unwrap_or_else(|| as_bool(obj.get("required"), true));
        let rows = as_rows(obj.get("rows"));
        let row_count = rows.len() as u64;
        total_rows += row_count;
        let duration_ms = as_f64(obj.get("duration_ms"), 0.0);
        let mut trusted = true;
        let mut reasons: Vec<String> = Vec::new();

        if status != "ok" && status != "zero_valid" {
            trusted = false;
            failed_count += 1;
            reasons.push(format!("status={}", status));
            errors.push(Diagnostic::error(
                "routeros_read_failed",
                Some(format!("results.{}.{}", router, path)),
                format!("RouterOS read result for {router}/{source} {path} is not trusted: status={status}."),
            ).with_safe_for_cleanup(false));
        }
        if duration_ms > slow_ms && slow_ms > 0.0 {
            slow_count += 1;
            warnings.push(Diagnostic::warning(
                "routeros_read_slow",
                Some(format!("results.{}.{}", router, path)),
                format!("RouterOS read for {router}/{source} {path} was slow: {duration_ms} ms."),
            ));
        }
        let previous_key = format!("{}:{}:{}", router, source, path);
        let prev = previous_counts.get(&previous_key)
            .or_else(|| previous_counts.get(format!("{}:{}", router, source)))
            .or_else(|| previous_counts.get(source))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if required && row_count == 0 && prev > 0 && status != "zero_valid" {
            trusted = false;
            suspicious_zero_count += 1;
            reasons.push("zero_after_previous_success".to_string());
            warnings.push(Diagnostic::warning(
                "routeros_zero_suspicious",
                Some(format!("results.{}.{}", router, path)),
                format!("RouterOS read for {router}/{source} {path} returned zero rows after previous non-zero result."),
            ).with_safe_for_cleanup(false));
        }

        let snapshot_key = format!("{}.{}", router, snapshot_name(source, path));
        snapshots.insert(snapshot_key.clone(), json!({
            "router": router,
            "source": source,
            "path": path,
            "row_count": row_count,
            "rows": rows,
            "trusted": trusted,
        }));

        let src_key = format!("{}:{}", router, source);
        let entry = source_map.entry(src_key.clone()).or_insert((0, true, Vec::new()));
        entry.0 += row_count;
        entry.1 = entry.1 && trusted;
        if !trusted { entry.2.extend(reasons.clone()); }

        command_reports.push(json!({
            "router": router,
            "source": source,
            "path": path,
            "required": required,
            "status": status,
            "trusted": trusted,
            "row_count": row_count,
            "duration_ms": duration_ms,
            "reasons": reasons,
            "planned": expected_cmd.is_some(),
        }));
    }

    for (key, cmd) in &expected {
        if required_keys.contains(key) && !seen.contains(key) {
            missing_count += 1;
            let obj = cmd.as_object().cloned().unwrap_or_default();
            let router = obj.get("router").and_then(Value::as_str).unwrap_or("unknown");
            let source = obj.get("source").and_then(Value::as_str).unwrap_or("unknown");
            let path = obj.get("path").and_then(Value::as_str).unwrap_or("");
            errors.push(Diagnostic::error(
                "routeros_required_read_missing",
                Some(format!("plan.{}.{}", router, path)),
                format!("Required RouterOS read result is missing for {router}/{source} {path}."),
            ).with_safe_for_cleanup(false));
            command_reports.push(json!({
                "router": router,
                "source": source,
                "path": path,
                "required": true,
                "status": "missing",
                "trusted": false,
                "row_count": 0,
                "duration_ms": 0.0,
                "reasons": ["missing_required_result"],
                "planned": true,
            }));
        }
    }

    let mut source_reports: Vec<Value> = Vec::new();
    for (key, (row_count, safe, reasons)) in source_map {
        let mut parts = key.splitn(2, ':');
        let router = parts.next().unwrap_or("unknown");
        let source = parts.next().unwrap_or("unknown");
        source_reports.push(json!({
            "router": router,
            "source": source,
            "row_count": row_count,
            "safe_for_cleanup": safe,
            "trusted": safe,
            "reasons": reasons,
        }));
    }

    let safe_for_cleanup = errors.is_empty() && suspicious_zero_count == 0;
    let status = if errors.is_empty() && suspicious_zero_count == 0 {
        "trusted"
    } else if missing_count > 0 || failed_count > 0 {
        "failed"
    } else {
        "partial"
    };

    if strict && !safe_for_cleanup && errors.is_empty() {
        errors.push(Diagnostic::error(
            "routeros_results_strict_not_trusted",
            Some("results".to_string()),
            "Strict RouterOS read-result validation requires all planned results to be trusted.",
        ).with_safe_for_cleanup(false));
    }

    let result = json!({
        "mode": "routeros_read_results_contract",
        "status": status,
        "safe_for_cleanup": safe_for_cleanup,
        "trusted": safe_for_cleanup,
        "command_count": command_reports.len(),
        "planned_command_count": expected.len(),
        "received_result_count": seen.len(),
        "missing_required_count": missing_count,
        "failed_count": failed_count,
        "slow_count": slow_count,
        "suspicious_zero_count": suspicious_zero_count,
        "total_row_count": total_rows,
        "commands": command_reports,
        "sources": source_reports,
        "snapshots": Value::Object(snapshots),
        "authority_note": "Python still performs live RouterOS reads. This Rust operation validates command-level read results before future Rust transport authority.",
    });
    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_routeros_read_results_against_plan() {
        let payload = json!({
            "plan": {"commands": [
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/active", "required":true},
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/secret", "required":true}
            ]},
            "results": [
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/active", "status":"ok", "rows":[{"name":"u1"}], "duration_ms": 10.0},
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/secret", "status":"ok", "rows":[{"name":"u1"}], "duration_ms": 12.0}
            ]
        });
        let (result, errors, warnings) = validate_routeros_read_results_payload(&payload);
        assert!(errors.is_empty(), "errors: {:?}", errors);
        assert!(warnings.is_empty(), "warnings: {:?}", warnings);
        assert_eq!(result.get("status").and_then(Value::as_str), Some("trusted"));
        assert_eq!(result.get("safe_for_cleanup").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn blocks_missing_required_read_result() {
        let payload = json!({
            "plan": {"commands": [
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/active", "required":true}
            ]},
            "results": []
        });
        let (result, errors, _warnings) = validate_routeros_read_results_payload(&payload);
        assert!(!errors.is_empty());
        assert_eq!(result.get("safe_for_cleanup").and_then(Value::as_bool), Some(false));
    }
}
