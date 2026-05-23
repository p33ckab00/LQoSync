use crate::apply_manifest::build_apply_manifest_payload;
use crate::diff::diff_files_payload;
use crate::diff::diff_network_text;
use crate::protocol::Diagnostic;
use crate::routeros_live_read_adapter::run_routeros_live_read_adapter_pilot_payload;
use crate::routeros_plan::build_routeros_collector_plan_payload;
use crate::routeros_shadow_bundle::build_routeros_shadow_collector_bundle_payload;
use crate::rust_network_json_shadow::build_rust_network_json_shadow_payload;
use crate::shaped_devices::{parse_csv_text, render_csv_text, ShapedDeviceRow, FIELDNAMES};
use crate::sync_plan::evaluate_sync_plan_payload;
use crate::validators::validate_files_payload;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::time::Instant;

pub(crate) fn merge_diags(target: &mut Vec<Diagnostic>, mut source: Vec<Diagnostic>) {
    target.append(&mut source);
}

pub(crate) fn response_envelope(
    op: &str,
    result: Value,
    errors: &[Diagnostic],
    warnings: &[Diagnostic],
) -> Value {
    json!({
        "op": op,
        "ok": errors.is_empty(),
        "available": true,
        "result": result,
        "errors": errors,
        "warnings": warnings
    })
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
    let Some(value) = value else {
        return;
    };
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

pub(crate) fn supplied_read_results(payload: &Value) -> Vec<Value> {
    let mut results = Vec::new();
    append_result_entries(&mut results, payload.get("results"));
    append_result_entries(&mut results, payload.get("read_results"));
    append_result_entries(&mut results, payload.get("live_read_results"));
    append_live_read_result(&mut results, payload.get("live_read_result"));
    append_live_read_result(&mut results, payload.get("read_result"));
    append_live_read_result(&mut results, payload.get("live_read_adapter"));
    append_live_read_result(&mut results, payload.get("adapter_result"));
    results
}

pub(crate) fn empty_csv_text() -> String {
    let mut text = FIELDNAMES.join(",");
    text.push('\n');
    text
}

fn value_field(value: &Value, field: &str) -> String {
    match value.get(field) {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn boolish(value: &Value, key: &str) -> bool {
    match value.get(key) {
        Some(Value::Bool(v)) => *v,
        Some(Value::String(s)) => matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        _ => false,
    }
}

fn config_network_mode(config: &Value) -> String {
    let mode = config
        .get("network_mode")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    match mode {
        "router_children" | "flat_router_root" | "flat_no_parent" | "deep_hierarchy"
        | "custom_hierarchy" => mode.to_string(),
        _ => {
            if boolish(config, "flat_network") && boolish(config, "no_parent") {
                "flat_no_parent".to_string()
            } else if boolish(config, "flat_network") {
                "flat_router_root".to_string()
            } else {
                "router_children".to_string()
            }
        }
    }
}

fn default_router_name(config: &Value) -> String {
    config
        .get("routers")
        .and_then(Value::as_array)
        .and_then(|routers| {
            routers
                .iter()
                .find_map(|router| router.get("name").and_then(Value::as_str))
        })
        .unwrap_or("")
        .trim()
        .to_string()
}

fn meta_router_by_circuit(shadow_result: &Value) -> BTreeMap<String, String> {
    let mut routers = BTreeMap::new();
    let Some(items) = shadow_result.get("meta").and_then(Value::as_array) else {
        return routers;
    };
    for item in items {
        let router = value_field(item, "router");
        if router.is_empty() {
            continue;
        }
        for key in ["circuit_name", "username", "hostname"] {
            let circuit = value_field(item, key);
            if !circuit.is_empty() {
                routers.insert(circuit, router.clone());
            }
        }
    }
    routers
}

fn apply_flat_parent_overrides(config: &Value, shadow_result: &Value, rows: &mut [Value]) {
    let mode = config_network_mode(config);
    if !matches!(mode.as_str(), "flat_no_parent" | "flat_router_root") {
        return;
    }
    let routers = meta_router_by_circuit(shadow_result);
    let default_router = default_router_name(config);
    for row in rows {
        let key = row_key(row);
        let parent = if mode == "flat_no_parent" {
            String::new()
        } else {
            routers
                .get(&key)
                .cloned()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| default_router.clone())
        };
        if let Value::Object(fields) = row {
            fields.insert("Parent Node".to_string(), Value::String(parent));
        }
    }
}

fn row_key(row: &Value) -> String {
    for field in ["Circuit Name", "Circuit ID", "Device ID", "Device Name"] {
        let value = value_field(row, field);
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

fn row_map(rows: &[Value]) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for row in rows {
        let key = row_key(row);
        if !key.is_empty() {
            out.insert(key, row.clone());
        }
    }
    out
}

fn diff_counts(current_rows: &[Value], proposed_rows: &[Value]) -> Value {
    let compare_fields = [
        "Parent Node",
        "MAC",
        "IPv4",
        "IPv6",
        "Download Min Mbps",
        "Upload Min Mbps",
        "Download Max Mbps",
        "Upload Max Mbps",
        "Comment",
    ];
    let current = row_map(current_rows);
    let proposed = row_map(proposed_rows);

    let mut updated = 0u64;
    for key in current.keys() {
        if let (Some(lhs), Some(rhs)) = (current.get(key), proposed.get(key)) {
            if compare_fields
                .iter()
                .any(|field| value_field(lhs, field) != value_field(rhs, field))
            {
                updated += 1;
            }
        }
    }

    let added = proposed
        .keys()
        .filter(|key| !current.contains_key(*key))
        .count() as u64;
    let removed = current
        .keys()
        .filter(|key| !proposed.contains_key(*key))
        .count() as u64;
    json!({
        "added": added,
        "updated": updated,
        "removed": removed
    })
}

fn parse_csv_rows(text: &str) -> Result<Vec<Value>, csv::Error> {
    Ok(parse_csv_text(text)?
        .into_iter()
        .map(|row| json!(row.fields))
        .collect())
}

pub(crate) fn rows_to_csv_text(rows: &[Value]) -> Result<String, csv::Error> {
    let normalized: Vec<ShapedDeviceRow> = rows
        .iter()
        .filter_map(Value::as_object)
        .map(|row| {
            let mut fields = BTreeMap::new();
            let keys: BTreeSet<String> = row
                .keys()
                .cloned()
                .chain(FIELDNAMES.iter().map(|field| field.to_string()))
                .collect();
            for key in keys {
                let value = row
                    .get(&key)
                    .map(|item| match item {
                        Value::String(s) => s.trim().to_string(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => String::new(),
                    })
                    .unwrap_or_default();
                fields.insert(key, value);
            }
            ShapedDeviceRow { fields }
        })
        .collect();
    render_csv_text(&normalized)
}

pub(crate) fn sha256_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

pub(crate) fn load_current_rows(
    payload: &Value,
    warnings: &mut Vec<Diagnostic>,
) -> (Vec<Value>, &'static str) {
    for key in ["python_rows", "current_rows", "current_csv_rows"] {
        if let Some(rows) = payload.get(key).and_then(Value::as_array) {
            let filtered = rows.iter().filter(|row| row.is_object()).cloned().collect();
            return (filtered, "payload_rows");
        }
    }

    if let Some(text) = payload.get("current_csv_text").and_then(Value::as_str) {
        return match parse_csv_rows(text) {
            Ok(rows) => (rows, "payload_text"),
            Err(err) => {
                warnings.push(Diagnostic::warning(
                    "rust_native_dry_run_current_csv_parse_failed",
                    Some("current_csv_text".to_string()),
                    format!("Rust native dry-run preview could not parse current ShapedDevices.csv text: {err}"),
                ));
                match parse_csv_rows(&empty_csv_text()) {
                    Ok(rows) => (rows, "empty_fallback"),
                    Err(_) => (Vec::new(), "empty_fallback"),
                }
            }
        };
    }

    let path = payload
        .get("current_csv_path")
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .get("config")
                .and_then(|v| v.get("paths"))
                .and_then(|v| v.get("shaped_devices_csv"))
                .and_then(Value::as_str)
        })
        .unwrap_or("");

    if path.is_empty() {
        return match parse_csv_rows(&empty_csv_text()) {
            Ok(rows) => (rows, "empty_fallback"),
            Err(_) => (Vec::new(), "empty_fallback"),
        };
    }

    match fs::read_to_string(path) {
        Ok(text) => match parse_csv_rows(&text) {
            Ok(rows) => (rows, "file"),
            Err(err) => {
                warnings.push(Diagnostic::warning(
                    "rust_native_dry_run_current_csv_parse_failed",
                    Some("config.paths.shaped_devices_csv".to_string()),
                    format!("Rust native dry-run preview could not parse current ShapedDevices.csv at {path}: {err}"),
                ));
                match parse_csv_rows(&empty_csv_text()) {
                    Ok(rows) => (rows, "empty_fallback"),
                    Err(_) => (Vec::new(), "empty_fallback"),
                }
            }
        },
        Err(err) => {
            warnings.push(Diagnostic::warning(
                "rust_native_dry_run_current_csv_read_failed",
                Some("config.paths.shaped_devices_csv".to_string()),
                format!("Rust native dry-run preview could not read current ShapedDevices.csv at {path}: {err}"),
            ));
            match parse_csv_rows(&empty_csv_text()) {
                Ok(rows) => (rows, "empty_fallback"),
                Err(_) => (Vec::new(), "empty_fallback"),
            }
        }
    }
}

/// Build a Rust-native dry-run preview for the WebUI without file writes.
///
/// This operation keeps the route read-only, but moves the preview orchestration
/// itself into Rust: collector plan generation, optional live-read execution,
/// shadow bundling, current-CSV parity, and top-level dry-run status shaping.
/// Python remains a transport/UI shell until the scheduler and authoritative
/// run-cycle are cut over.
pub fn build_rust_native_dry_run_preview_payload(
    payload: &Value,
) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let started = Instant::now();
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let config = payload.get("config").unwrap_or(payload);

    let (plan_result, plan_errors, plan_warnings) = build_routeros_collector_plan_payload(payload);
    merge_diags(&mut errors, plan_errors.clone());
    merge_diags(&mut warnings, plan_warnings.clone());
    let plan_envelope = response_envelope(
        "build-routeros-collector-plan",
        plan_result.clone(),
        &plan_errors,
        &plan_warnings,
    );

    let commands: Vec<Value> = plan_result
        .get("commands")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter(|v| v.is_object()).cloned().collect())
        .unwrap_or_default();

    let mut live_read_responses: Vec<Value> = Vec::new();
    let mut live_read_results = supplied_read_results(payload);
    let input_mode = if live_read_results.is_empty() {
        "live_read_adapter"
    } else {
        "supplied_results"
    };

    if live_read_results.is_empty() {
        for command in &commands {
            let mut adapter_payload = payload.clone();
            if let Value::Object(ref mut map) = adapter_payload {
                map.insert(
                    "router".to_string(),
                    command.get("router").cloned().unwrap_or_else(|| json!("")),
                );
                map.insert(
                    "source".to_string(),
                    command.get("source").cloned().unwrap_or_else(|| json!("")),
                );
                map.insert(
                    "path".to_string(),
                    command.get("path").cloned().unwrap_or_else(|| json!("")),
                );
                map.insert(
                    "fields".to_string(),
                    command.get("fields").cloned().unwrap_or_else(|| json!([])),
                );
                map.entry("adapter".to_string())
                    .or_insert_with(|| json!("live"));
                map.entry("mode".to_string())
                    .or_insert_with(|| json!("live_read"));
                map.entry("execute".to_string())
                    .or_insert_with(|| json!(true));
            }

            let (adapter_result, adapter_errors, adapter_warnings) =
                run_routeros_live_read_adapter_pilot_payload(&adapter_payload);
            merge_diags(&mut errors, adapter_errors.clone());
            merge_diags(&mut warnings, adapter_warnings.clone());
            let adapter_envelope = response_envelope(
                "run-routeros-live-read-adapter-pilot",
                adapter_result.clone(),
                &adapter_errors,
                &adapter_warnings,
            );
            live_read_responses.push(json!({
                "router": command.get("router").cloned().unwrap_or_else(|| json!("")),
                "source": command.get("source").cloned().unwrap_or_else(|| json!("")),
                "path": command.get("path").cloned().unwrap_or_else(|| json!("")),
                "response": adapter_envelope
            }));

            if let Some(read_result) = adapter_result.get("read_result").filter(|v| v.is_object()) {
                live_read_results.push(read_result.clone());
            } else if adapter_result.get("path").is_some() && adapter_result.get("rows").is_some() {
                live_read_results.push(adapter_result.clone());
            }
        }
    }

    let (current_rows, current_csv_source) = load_current_rows(payload, &mut warnings);

    let mut shadow_payload = payload.clone();
    if let Value::Object(ref mut map) = shadow_payload {
        map.insert("plan".to_string(), plan_result.clone());
        map.insert(
            "results".to_string(),
            Value::Array(live_read_results.clone()),
        );
        map.insert(
            "python_rows".to_string(),
            Value::Array(current_rows.clone()),
        );
    }
    let (shadow_result, shadow_errors, shadow_warnings) =
        build_routeros_shadow_collector_bundle_payload(&shadow_payload);
    merge_diags(&mut errors, shadow_errors.clone());
    merge_diags(&mut warnings, shadow_warnings.clone());
    let shadow_envelope = response_envelope(
        "build-routeros-shadow-collector-bundle",
        shadow_result.clone(),
        &shadow_errors,
        &shadow_warnings,
    );

    let mut network_payload = payload.clone();
    if let Value::Object(ref mut map) = network_payload {
        map.insert("shadow_bundle".to_string(), shadow_result.clone());
    }
    let (network_result, network_errors, network_warnings) =
        build_rust_network_json_shadow_payload(&network_payload);
    merge_diags(&mut errors, network_errors.clone());
    merge_diags(&mut warnings, network_warnings.clone());
    let network_envelope = response_envelope(
        "build-rust-network-json-shadow",
        network_result.clone(),
        &network_errors,
        &network_warnings,
    );

    let mut proposed_rows: Vec<Value> = shadow_result
        .get("normalized_rows")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter(|v| v.is_object()).cloned().collect())
        .unwrap_or_default();
    apply_flat_parent_overrides(config, &shadow_result, &mut proposed_rows);
    let proposed_csv_text = match rows_to_csv_text(&proposed_rows) {
        Ok(text) => text,
        Err(err) => {
            errors.push(Diagnostic::error(
                "rust_native_dry_run_proposed_csv_render_failed",
                Some("shadow_bundle.normalized_rows".to_string()),
                format!("Rust native dry-run preview could not render proposed ShapedDevices.csv text: {err}"),
            ));
            empty_csv_text()
        }
    };

    let csv_counts = diff_counts(&current_rows, &proposed_rows);
    let csv_changed = csv_counts
        .as_object()
        .map(|counts| counts.values().any(|v| v.as_u64().unwrap_or(0) > 0))
        .unwrap_or(false);
    let current_network_text = network_result
        .get("current_network_text")
        .and_then(Value::as_str)
        .unwrap_or("{}\n");
    let proposed_network_text = network_result
        .get("network_text")
        .and_then(Value::as_str)
        .unwrap_or("{}\n");
    let (network_diff, network_diff_errors, network_diff_warnings) =
        diff_network_text(current_network_text, proposed_network_text);
    merge_diags(&mut errors, network_diff_errors);
    merge_diags(&mut warnings, network_diff_warnings);
    let network_changed = network_diff
        .get("changed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let network_counts = json!({
        "added": network_diff.get("added_node_count").cloned().unwrap_or_else(|| json!(0)),
        "updated": network_diff.get("updated_node_count").cloned().unwrap_or_else(|| json!(0)),
        "removed": network_diff.get("removed_node_count").cloned().unwrap_or_else(|| json!(0))
    });
    let (validation_result, validation_errors, validation_warnings) =
        validate_files_payload(&json!({
            "config": config,
            "csv_text": proposed_csv_text,
            "network_text": proposed_network_text,
        }));
    merge_diags(&mut errors, validation_errors.clone());
    merge_diags(&mut warnings, validation_warnings.clone());
    let validation_envelope = response_envelope(
        "validate-files",
        validation_result.clone(),
        &validation_errors,
        &validation_warnings,
    );
    let (diff_result, diff_errors, diff_warnings) = diff_files_payload(&json!({
        "current_csv_text": rows_to_csv_text(&current_rows).unwrap_or_else(|_| empty_csv_text()),
        "proposed_csv_text": proposed_csv_text.clone(),
        "current_network_text": current_network_text,
        "proposed_network_text": proposed_network_text,
    }));
    merge_diags(&mut errors, diff_errors.clone());
    merge_diags(&mut warnings, diff_warnings.clone());
    let diff_envelope = response_envelope(
        "diff-files",
        diff_result.clone(),
        &diff_errors,
        &diff_warnings,
    );
    let sync_plan_diff = diff_envelope.clone();
    let sync_plan_validation = validation_envelope.clone();
    let sync_plan_circuit_shadow = shadow_envelope.clone();
    let (sync_plan_result, sync_plan_errors, sync_plan_warnings) =
        evaluate_sync_plan_payload(&json!({
            "mode": "dry_run",
            "files_changed": csv_changed || network_changed,
            "csv_changed": csv_changed,
            "network_changed": network_changed,
            "rust_diff": sync_plan_diff,
            "rust_validation": sync_plan_validation,
            "rust_policy_shadow": {},
            "rust_circuit_shadow": sync_plan_circuit_shadow,
            "collector_trust": [],
            "preflight": {"errors": [], "warnings": []},
            "cleanup": {"removed": 0, "queued": 0, "preserved": 0, "candidates": 0},
        }));
    merge_diags(&mut errors, sync_plan_errors.clone());
    merge_diags(&mut warnings, sync_plan_warnings.clone());
    let sync_plan_envelope = response_envelope(
        "evaluate-sync-plan",
        sync_plan_result.clone(),
        &sync_plan_errors,
        &sync_plan_warnings,
    );
    let apply_manifest_sync_plan = sync_plan_envelope.clone();
    let (apply_manifest_result, apply_manifest_errors, apply_manifest_warnings) =
        build_apply_manifest_payload(&json!({
            "config": config,
            "mode": "dry_run",
            "paths": config.get("paths").cloned().unwrap_or_else(|| json!({})),
            "state": payload.get("state").cloned().unwrap_or_else(|| json!({})),
            "current_csv_text": rows_to_csv_text(&current_rows).unwrap_or_else(|_| empty_csv_text()),
            "proposed_csv_text": proposed_csv_text.clone(),
            "current_network_text": current_network_text,
            "proposed_network_text": proposed_network_text,
            "files_changed": csv_changed || network_changed,
            "csv_changed": csv_changed,
            "network_changed": network_changed,
            "policy_decision": {"write_allowed": true, "apply_allowed": true},
            "rust_sync_plan": apply_manifest_sync_plan,
            "rust_authority_gate": {"should_block": false},
        }));
    merge_diags(&mut errors, apply_manifest_errors.clone());
    merge_diags(&mut warnings, apply_manifest_warnings.clone());
    let apply_manifest_envelope = response_envelope(
        "build-apply-manifest",
        apply_manifest_result.clone(),
        &apply_manifest_errors,
        &apply_manifest_warnings,
    );
    let bundle_status = shadow_result
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let network_status = network_result
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    let status = if !errors.is_empty() {
        "rust_native_dry_run_blocked"
    } else if !commands.is_empty()
        && !live_read_results.is_empty()
        && bundle_status == "shadow_ready"
        && network_status == "shadow_ready"
    {
        "dry_run_complete"
    } else if !commands.is_empty() {
        "rust_native_dry_run_review"
    } else {
        warnings.push(Diagnostic::warning(
            "rust_native_dry_run_no_commands",
            Some("config.routers".to_string()),
            "Rust native dry-run preview found no RouterOS commands to execute.",
        ));
        "rust_native_dry_run_unavailable"
    };

    let elapsed = started.elapsed();
    let duration_seconds = (elapsed.as_secs_f64() * 1000.0).round() / 1000.0;
    let duration_ms = (elapsed.as_secs_f64() * 1000.0 * 1000.0).round() / 1000.0;
    let result = json!({
        "status": status,
        "mode": "dry_run",
        "source": "rust_native_preview",
        "duration_seconds": duration_seconds,
        "csv_changed": csv_changed,
        "network_changed": network_changed,
        "files_changed": csv_changed || network_changed,
        "proposed_rows": proposed_rows.clone(),
        "proposed_csv_text": proposed_csv_text.clone(),
        "diff": {
            "csv": {
                "changed": csv_changed,
                "counts": csv_counts
            },
            "network": {
                "changed": network_changed,
                "counts": network_counts,
                "detail": network_diff
            },
            "rust_core_diff": diff_envelope,
            "rust_core_validation": validation_envelope,
            "rust_sync_plan": sync_plan_envelope,
            "rust_apply_manifest": apply_manifest_envelope,
            "rust_native_preview": {
                "engine": "rust_live_read_shadow_bundle",
                "input_mode": input_mode,
                "current_csv_source": current_csv_source,
                "current_network_source": network_result.get("current_network_source").cloned().unwrap_or_else(|| json!("unknown")),
                "current_csv_row_count": current_rows.len(),
                "command_count": commands.len(),
                "live_read_result_count": live_read_results.len(),
                "proposed_csv_sha256": sha256_text(&proposed_csv_text),
                "proposed_network_sha256": sha256_text(proposed_network_text),
                "plan": plan_envelope,
                "live_reads": live_read_responses,
                "shadow_bundle": shadow_envelope,
                "network_shadow": network_envelope,
                "current_csv_parity": shadow_result.get("parity").cloned().unwrap_or_else(|| json!({})),
                "current_csv_counts": csv_counts
            }
        },
        "node_math": network_result.get("node_math").cloned().unwrap_or_else(|| json!({})),
        "timings": {
            "rust_native_dry_run_preview_ms": duration_ms,
            "rust_routeros_command_count": commands.len(),
            "rust_live_read_result_count": live_read_results.len(),
            "rust_network_node_count": network_result.get("node_count").cloned().unwrap_or_else(|| json!(0))
        }
    });

    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_complete_preview_from_supplied_results() {
        let payload = json!({
            "config": {
                "paths": {
                    "shaped_devices_csv": "/opt/libreqos/src/ShapedDevices.csv",
                    "network_json": "/opt/libreqos/src/network.json"
                },
                "defaults": {"default_pppoe_rate":"10M/10M", "min_rate_percentage":0.5},
                "routers": [{
                    "name":"RB5009",
                    "enabled": true,
                    "root_download_mbps": 115,
                    "root_upload_mbps": 115,
                    "pppoe":{"enabled":true, "per_plan_node":true, "plan_node_name":"{profile}-{router}"}
                }]
            },
            "results": [
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/active", "status":"ok", "rows":[{"name":"juan", "address":"10.0.0.2", "caller-id":"AA:BB:CC:DD:EE:FF"}]},
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/secret", "status":"ok", "rows":[{"name":"juan", "profile":"15M", "comment":"PLAN|15M/15M", "disabled":"false", "inactive":"false"}]},
                {"router":"RB5009", "source":"pppoe", "path":"/ppp/profile", "status":"ok", "rows":[{"name":"15M", "rate-limit":"15M/15M"}]}
            ],
            "python_rows": [
                {"Circuit ID":"juan", "Circuit Name":"juan", "Device ID":"juan", "Device Name":"juan", "Parent Node":"15M-RB5009", "MAC":"AA:BB:CC:DD:EE:FF", "IPv4":"10.0.0.2", "IPv6":"", "Download Min Mbps":"7.5", "Upload Min Mbps":"7.5", "Download Max Mbps":"15", "Upload Max Mbps":"15", "Comment":"PPP"}
            ]
        });
        let (result, errors, warnings) = build_rust_native_dry_run_preview_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            result.get("status").and_then(Value::as_str),
            Some("dry_run_complete")
        );
        assert_eq!(
            result["diff"]["rust_native_preview"]["input_mode"],
            "supplied_results"
        );
        assert_eq!(
            result["diff"]["rust_native_preview"]["command_count"].as_u64(),
            Some(3)
        );
        assert_eq!(
            result["diff"]["rust_native_preview"]["live_read_result_count"].as_u64(),
            Some(3)
        );
        assert_eq!(
            result.get("csv_changed").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            result.get("network_changed").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn blocks_without_live_read_gates_when_results_are_not_supplied() {
        let payload = json!({
            "config": {
                "paths": {
                    "shaped_devices_csv": "/opt/libreqos/src/ShapedDevices.csv",
                    "network_json": "/opt/libreqos/src/network.json"
                },
                "defaults": {"default_pppoe_rate":"10M/10M", "min_rate_percentage":0.5},
                "routers": [{
                    "name":"RB5009",
                    "enabled": true,
                    "root_download_mbps": 115,
                    "root_upload_mbps": 115,
                    "pppoe":{"enabled":true}
                }]
            }
        });
        let (result, errors, _warnings) = build_rust_native_dry_run_preview_payload(&payload);
        assert!(!errors.is_empty());
        assert_eq!(
            result.get("status").and_then(Value::as_str),
            Some("rust_native_dry_run_blocked")
        );
        assert_eq!(
            result["diff"]["rust_native_preview"]["command_count"].as_u64(),
            Some(3)
        );
        assert_eq!(
            result["diff"]["rust_native_preview"]["live_read_result_count"].as_u64(),
            Some(0)
        );
    }
}
