use crate::network::{collect_node_names, parse_network_text, validate_network};
use crate::protocol::{Diagnostic, Severity};
use crate::shaped_devices::{parse_csv_text, validate_rows};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

const VALID_NETWORK_MODES: &[&str] = &[
    "flat_no_parent",
    "flat_router_root",
    "router_children",
    "deep_hierarchy",
    "custom_hierarchy",
];

const CLEANUP_ACTIONS: &[&str] = &[
    "preserve_rows",
    "warn_only",
    "cleanup_immediate",
    "cleanup_next_run",
    "require_confirm_immediate",
    "require_confirm_next_run",
    "block_cleanup",
    "block_apply",
];

pub fn validate_config_value(config: &Value) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors = Vec::new();
    let warnings = Vec::new();

    let mode = config.get("network_mode").and_then(Value::as_str).unwrap_or("router_children");
    if !VALID_NETWORK_MODES.contains(&mode) {
        errors.push(Diagnostic::error(
            "invalid_network_mode",
            Some("network_mode".to_string()),
            format!("network_mode invalid: {mode}"),
        )
        .with_value(json!(mode)));
    }

    for key in ["shaped_devices_csv", "network_json"] {
        let path = config.pointer(&format!("/paths/{key}")).and_then(Value::as_str).unwrap_or("");
        if path.trim().is_empty() {
            errors.push(Diagnostic::error(
                "missing_required_path",
                Some(format!("paths.{key}")),
                format!("paths.{key} is required"),
            ));
        }
    }

    if let Some(policies) = config.get("policies") {
        validate_policy_actions(policies, "policies", &mut errors);
        let policy_mode = policies.get("mode").and_then(Value::as_str).unwrap_or("singularity");
        if !["singularity", "custom", "conservative", "balanced", "aggressive"].contains(&policy_mode) {
            errors.push(Diagnostic::error(
                "invalid_policy_mode",
                Some("policies.mode".to_string()),
                format!("policies.mode invalid: {policy_mode}; expected singularity or custom"),
            )
            .with_value(json!(policy_mode)));
        }
    }

    if let Some(routers) = config.get("routers").and_then(Value::as_array) {
        let mut names = HashSet::new();
        for (idx, router) in routers.iter().enumerate() {
            let name = router.get("name").and_then(Value::as_str).unwrap_or("Router");
            if !names.insert(name.to_string()) {
                errors.push(Diagnostic::error(
                    "duplicate_router_name",
                    Some(format!("routers[{idx}].name")),
                    format!("duplicate router name: {name}"),
                )
                .with_value(json!(name)));
            }
            for key in ["root_download_mbps", "root_upload_mbps"] {
                if router.get(key).and_then(Value::as_f64).is_none() {
                    errors.push(Diagnostic::error(
                        "invalid_router_bandwidth",
                        Some(format!("routers[{idx}].{key}")),
                        format!("{name}: {key} must be numeric"),
                    ));
                }
            }
        }
    }

    (errors, warnings)
}

fn validate_policy_actions(value: &Value, path: &str, errors: &mut Vec<Diagnostic>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_path = format!("{path}.{key}");
                if key.ends_with("_action") || key == "action" || key.ends_with("action") {
                    if let Some(action) = child.as_str() {
                        if !CLEANUP_ACTIONS.contains(&action) {
                            errors.push(Diagnostic::error(
                                "invalid_policy_action",
                                Some(child_path.clone()),
                                format!("policy action invalid: {action}"),
                            )
                            .with_value(json!(action)));
                        }
                    }
                }
                validate_policy_actions(child, &child_path, errors);
            }
        }
        Value::Array(items) => {
            for (idx, child) in items.iter().enumerate() {
                validate_policy_actions(child, &format!("{path}[{idx}]"), errors);
            }
        }
        _ => {}
    }
}

pub fn validate_files_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    let config = payload.get("config").cloned().or_else(|| {
        payload.get("config_path").and_then(Value::as_str).and_then(|path| {
            fs::read_to_string(Path::new(path)).ok().and_then(|text| serde_json::from_str(&text).ok())
        })
    }).unwrap_or_else(|| json!({}));
    if !config.is_null() && config.is_object() {
        let (mut cfg_errors, mut cfg_warnings) = validate_config_value(&config);
        errors.append(&mut cfg_errors);
        warnings.append(&mut cfg_warnings);
    }

    let network_mode = config.get("network_mode").and_then(Value::as_str).unwrap_or("router_children");

    let csv_text = match text_from_payload(payload, "csv_text", "shaped_devices_csv_path") {
        Ok(text) => text,
        Err(diag) => {
            errors.push(diag);
            String::new()
        }
    };
    let network_text = match text_from_payload(payload, "network_text", "network_json_path") {
        Ok(text) => text,
        Err(diag) => {
            errors.push(diag);
            "{}".to_string()
        }
    };

    let mut row_count = 0usize;
    let mut node_count = 0usize;

    let parent_nodes = match parse_network_text(&network_text) {
        Ok(network) => {
            let (mut net_errors, mut net_warnings) = validate_network(&network);
            errors.append(&mut net_errors);
            warnings.append(&mut net_warnings);
            let names = collect_node_names(&network);
            node_count = names.len();
            Some(names)
        }
        Err(e) => {
            errors.push(Diagnostic::error(
                "invalid_network_json",
                Some("network_json".to_string()),
                format!("network.json parse failed: {e}"),
            ));
            None
        }
    };

    match parse_csv_text(&csv_text) {
        Ok(rows) => {
            row_count = rows.len();
            let (mut row_errors, mut row_warnings) = validate_rows(&rows, network_mode, parent_nodes.as_ref());
            errors.append(&mut row_errors);
            warnings.append(&mut row_warnings);
        }
        Err(e) => errors.push(Diagnostic::error(
            "invalid_shaped_devices_csv",
            Some("ShapedDevices.csv".to_string()),
            format!("ShapedDevices.csv parse failed: {e}"),
        )),
    }

    let result = json!({
        "row_count": row_count,
        "node_count": node_count,
        "risk_level": risk_level(&errors, &warnings),
        "write_allowed": errors.is_empty(),
        "apply_allowed": errors.is_empty(),
    });

    (result, errors, warnings)
}

pub fn validate_collector_output_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let router = payload.get("router").and_then(Value::as_str).unwrap_or("unknown");
    let source = payload.get("source").and_then(Value::as_str).unwrap_or("unknown");
    let status = payload.get("status").and_then(Value::as_str).unwrap_or("ok");
    let row_count = payload.get("rows").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0);
    let previous_success_count = payload.get("previous_success_count").and_then(Value::as_u64).unwrap_or(0);
    let failed_reads = payload.get("failed_reads").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0);

    let mut safe_for_cleanup = true;

    if status == "failed" || status == "partial" || failed_reads > 0 {
        safe_for_cleanup = false;
        errors.push(Diagnostic::error(
            "collector_not_trusted",
            Some(format!("collector.{router}.{source}")),
            format!("Collector output for {router}/{source} is not trusted: status={status}, failed_reads={failed_reads}"),
        )
        .with_safe_for_cleanup(false));
    }

    if row_count == 0 && previous_success_count > 0 && status != "zero_valid" {
        safe_for_cleanup = false;
        warnings.push(Diagnostic {
            code: "collector_zero_suspicious".to_string(),
            severity: Severity::Warning,
            path: Some(format!("collector.{router}.{source}.rows")),
            message: format!("Collector returned zero rows for {router}/{source} after previous successful non-zero run"),
            value: Some(json!(row_count)),
            safe_for_cleanup: Some(false),
        });
    }

    let result = json!({
        "router": router,
        "source": source,
        "status": status,
        "row_count": row_count,
        "safe_for_cleanup": safe_for_cleanup,
        "write_allowed": errors.is_empty(),
        "apply_allowed": errors.is_empty(),
    });

    (result, errors, warnings)
}

fn text_from_payload(payload: &Value, text_key: &str, path_key: &str) -> Result<String, Diagnostic> {
    if let Some(text) = payload.get(text_key).and_then(Value::as_str) {
        return Ok(text.to_string());
    }
    if let Some(path) = payload.get(path_key).and_then(Value::as_str) {
        return fs::read_to_string(Path::new(path)).map_err(|e| {
            Diagnostic::error(
                "file_read_failed",
                Some(path_key.to_string()),
                format!("Failed to read {path}: {e}"),
            )
            .with_value(json!(path))
        });
    }
    Ok(String::new())
}

fn risk_level(errors: &[Diagnostic], warnings: &[Diagnostic]) -> &'static str {
    if errors.iter().any(|e| e.severity == Severity::Critical) {
        "critical"
    } else if !errors.is_empty() {
        "high"
    } else if !warnings.is_empty() {
        "medium"
    } else {
        "low"
    }
}
