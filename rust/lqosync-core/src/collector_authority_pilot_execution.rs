use crate::collector_authority_switch::build_collector_authority_switch_rehearsal_payload;
use crate::protocol::Diagnostic;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn bool_value(v: Option<&Value>, default: bool) -> bool {
    v.and_then(Value::as_bool).unwrap_or(default)
}

fn str_value<'a>(v: Option<&'a Value>, default: &'a str) -> &'a str {
    v.and_then(Value::as_str).unwrap_or(default)
}

fn number_value(v: Option<&Value>, default: u64) -> u64 {
    v.and_then(Value::as_u64).unwrap_or(default)
}

fn config_value<'a>(payload: &'a Value, key: &str) -> Option<&'a Value> {
    payload
        .get("rust_core")
        .and_then(|v| v.get(key))
        .or_else(|| payload.get("config").and_then(|c| c.get("rust_core")).and_then(|v| v.get(key)))
}

fn pilot_execution_id(value: &Value) -> String {
    let text = serde_json::to_string(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("cape-{}", &digest[..16])
}

/// Build a non-mutating collector-authority pilot execution contract.
///
/// v4.3 is the bridge after the switch rehearsal. It proves that the system
/// has an auditable contract for a future Rust collector authority pilot run,
/// while still refusing to switch production collector authority, drive
/// cleanup, write generated files, or apply LibreQoS in this release.
pub fn build_collector_authority_pilot_execution_contract_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(str_value(payload.get("mode"), "contract"), "execute" | "switch" | "promote" | "authority" | "apply" | "production");
    if requested_execute {
        errors.push(Diagnostic::error(
            "collector_authority_pilot_execution_not_implemented",
            Some("collector_authority_pilot_execution".to_string()),
            "This release only builds a collector authority pilot execution contract. It does not execute a production collector authority switch.",
        ));
    }

    let allow_contract = bool_value(config_value(payload, "allow_collector_authority_pilot_execution_contract"), false);
    let pilot_enabled = bool_value(config_value(payload, "collector_authority_pilot_execution_pilot"), false);
    let execution_mode = str_value(config_value(payload, "collector_authority_pilot_execution_mode"), "contract_only");
    let require_switch_rehearsal = bool_value(config_value(payload, "collector_authority_pilot_execution_require_switch_rehearsal"), true);
    let require_python_fallback = bool_value(config_value(payload, "collector_authority_pilot_execution_require_python_fallback"), true);
    let require_manual_confirmation = bool_value(config_value(payload, "collector_authority_pilot_execution_require_manual_confirmation"), true);
    let require_diagnostic_selection = bool_value(config_value(payload, "collector_authority_pilot_execution_require_diagnostic_selection"), true);
    let max_shadow_age = config_value(payload, "collector_authority_pilot_execution_max_shadow_age_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(900);
    let shadow_age = payload.get("shadow_age_seconds").and_then(Value::as_u64).unwrap_or(0);
    let confirmation = str_value(payload.get("confirmation"), "");
    let confirmation_ok = !require_manual_confirmation || confirmation == "CONFIRM_COLLECTOR_AUTHORITY_PILOT_EXECUTION";

    let switch_value = payload
        .get("collector_authority_switch_rehearsal")
        .and_then(|v| v.get("result"))
        .or_else(|| payload.get("collector_authority_switch_rehearsal"))
        .cloned();

    let switch_confirmation = payload
        .get("collector_authority_switch_confirmation")
        .or_else(|| payload.get("switch_confirmation"))
        .and_then(Value::as_str);

    let mut switch_payload = payload.clone();
    if let Some(token) = switch_confirmation {
        if let Some(obj) = switch_payload.as_object_mut() {
            obj.insert("confirmation".to_string(), json!(token));
        }
    }

    let (switch_rehearsal, switch_errors, mut switch_warnings) = match switch_value {
        Some(v) if v.is_object() => (v, Vec::new(), Vec::new()),
        _ => build_collector_authority_switch_rehearsal_payload(&switch_payload),
    };
    warnings.append(&mut switch_warnings);

    if !switch_errors.is_empty() {
        warnings.push(Diagnostic::warning(
            "collector_authority_pilot_execution_switch_not_clean",
            Some("collector_authority_switch_rehearsal".to_string()),
            "Collector authority switch rehearsal returned errors; pilot execution contract remains blocked.",
        ));
    }

    let switch_status = switch_rehearsal.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let switch_ready = switch_errors.is_empty()
        && switch_status == "collector_authority_switch_rehearsal_ready"
        && switch_rehearsal.get("production_collector_authority_switched").and_then(Value::as_bool) == Some(false)
        && switch_rehearsal.get("collector_authority_switch_executed").and_then(Value::as_bool) == Some(false)
        && switch_rehearsal.get("python_collector_fallback_required").and_then(Value::as_bool) == Some(true);

    let diagnostic_row_authority = str_value(switch_rehearsal.get("diagnostic_row_authority"), "python_authoritative");
    let production_row_authority = str_value(switch_rehearsal.get("production_row_authority"), "python_collector");
    let cleanup_row_authority = str_value(switch_rehearsal.get("cleanup_row_authority"), "python_collector");
    let diagnostic_selection_only = bool_value(switch_rehearsal.get("diagnostic_selection_only"), false);
    let rust_diagnostic_selection_ready = bool_value(switch_rehearsal.get("rust_diagnostic_selection_ready"), false);
    let rust_rows_selected_for_diagnostics = bool_value(switch_rehearsal.get("rust_rows_selected_for_diagnostics"), false);
    let rust_rows_safe_for_observation = bool_value(switch_rehearsal.get("rust_rows_safe_for_observation"), false);
    let diagnostic_selection_ready = switch_ready
        && diagnostic_selection_only
        && rust_diagnostic_selection_ready
        && rust_rows_selected_for_diagnostics
        && rust_rows_safe_for_observation
        && diagnostic_row_authority == "rust_shadow_diagnostics"
        && production_row_authority == "python_collector"
        && cleanup_row_authority == "python_collector";

    if require_switch_rehearsal && !switch_ready {
        warnings.push(Diagnostic::warning(
            "collector_authority_pilot_execution_switch_not_ready",
            Some("collector_authority_switch_rehearsal".to_string()),
            "Collector authority switch rehearsal is not ready; pilot execution contract remains shadow-only.",
        ));
    }
    if require_diagnostic_selection && !diagnostic_selection_ready {
        warnings.push(Diagnostic::warning(
            "collector_authority_pilot_execution_diagnostic_selection_not_ready",
            Some("collector_authority_switch_rehearsal".to_string()),
            "Switch rehearsal has not selected Rust rows for diagnostics-only observation; pilot execution contract remains waiting.",
        ));
    }
    if !require_python_fallback {
        errors.push(Diagnostic::error(
            "collector_authority_pilot_execution_requires_python_fallback",
            Some("rust_core.collector_authority_pilot_execution_require_python_fallback".to_string()),
            "Collector authority pilot execution contract requires Python collector fallback in this release.",
        ));
    }
    if shadow_age > max_shadow_age {
        warnings.push(Diagnostic::warning(
            "collector_authority_pilot_execution_shadow_stale",
            Some("shadow_age_seconds".to_string()),
            "Rust-shadow collector output is older than the configured maximum age; pilot execution contract remains waiting.",
        ));
    }
    if !confirmation_ok {
        warnings.push(Diagnostic::warning(
            "collector_authority_pilot_execution_confirmation_missing",
            Some("collector_authority_pilot_execution.confirmation".to_string()),
            "Manual confirmation token is missing; pilot execution contract remains waiting for confirmation.",
        ));
    }

    let requested = allow_contract && pilot_enabled && execution_mode == "rust_collector_authority_pilot_execution_contract";
    let ready = errors.is_empty()
        && requested
        && (!require_switch_rehearsal || switch_ready)
        && (!require_diagnostic_selection || diagnostic_selection_ready)
        && require_python_fallback
        && confirmation_ok
        && shadow_age <= max_shadow_age;

    let status = if !errors.is_empty() {
        "blocked"
    } else if ready {
        "collector_authority_pilot_execution_contract_ready"
    } else if switch_ready {
        "collector_authority_pilot_execution_waiting_for_gates"
    } else {
        "collector_authority_pilot_execution_shadow_only"
    };

    let rust_row_count = switch_rehearsal.get("rust_row_count").and_then(Value::as_u64).unwrap_or(0);
    let python_row_count = switch_rehearsal.get("python_row_count").and_then(Value::as_u64).unwrap_or(0);
    let live_read_shadow_row_count = number_value(switch_rehearsal.get("live_read_shadow_row_count"), 0);
    let shadow_history_successful_count = number_value(switch_rehearsal.get("shadow_history_successful_count"), 0);
    let runtime_evidence_source = str_value(switch_rehearsal.get("runtime_evidence_source"), "not_ready");
    let live_read_shadow_ready = bool_value(switch_rehearsal.get("live_read_shadow_ready"), false);
    let parity_verdict = str_value(switch_rehearsal.get("parity_verdict"), "not_available");
    let live_read_shadow_parity_verdict = str_value(switch_rehearsal.get("live_read_shadow_parity_verdict"), "not_available");
    let sources = payload.get("sources").cloned().unwrap_or_else(|| json!([]));

    let seed = json!({
        "status": status,
        "switch_status": switch_status,
        "execution_mode": execution_mode,
        "rust_row_count": rust_row_count,
        "runtime_evidence_source": runtime_evidence_source,
        "diagnostic_selection_ready": diagnostic_selection_ready,
        "confirmation_ok": confirmation_ok,
        "shadow_age_seconds": shadow_age,
    });

    // Build the response object incrementally instead of using one very large
    // serde_json::json! block. The large nested object previously exceeded
    // Rust's macro recursion limit on some toolchains even though the runtime
    // structure was valid.
    let target_authority = if ready { "rust_collector_authority_pilot_candidate" } else { "python_authoritative" };
    let mut result_map = serde_json::Map::new();
    result_map.insert("mode".to_string(), json!("collector_authority_pilot_execution_contract"));
    result_map.insert("status".to_string(), json!(status));
    result_map.insert("pilot_execution_contract_id".to_string(), json!(pilot_execution_id(&seed)));
    result_map.insert("collector_authority".to_string(), json!("python_authoritative"));
    result_map.insert("target_authority".to_string(), json!(target_authority));
    result_map.insert("contract_requested".to_string(), json!(requested));
    result_map.insert("allow_pilot_execution_contract".to_string(), json!(allow_contract));
    result_map.insert("pilot_execution_pilot".to_string(), json!(pilot_enabled));
    result_map.insert("pilot_execution_mode".to_string(), json!(execution_mode));
    result_map.insert("require_switch_rehearsal".to_string(), json!(require_switch_rehearsal));
    result_map.insert("require_python_fallback".to_string(), json!(require_python_fallback));
    result_map.insert("require_manual_confirmation".to_string(), json!(require_manual_confirmation));
    result_map.insert("require_diagnostic_selection".to_string(), json!(require_diagnostic_selection));
    result_map.insert("manual_confirmation_ok".to_string(), json!(confirmation_ok));
    result_map.insert("max_shadow_age_seconds".to_string(), json!(max_shadow_age));
    result_map.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    result_map.insert("switch_status".to_string(), json!(switch_status));
    result_map.insert("switch_ready".to_string(), json!(switch_ready));
    result_map.insert("runtime_evidence_source".to_string(), json!(runtime_evidence_source));
    result_map.insert("live_read_shadow_ready".to_string(), json!(live_read_shadow_ready));
    result_map.insert("shadow_history_successful_count".to_string(), json!(shadow_history_successful_count));
    result_map.insert("parity_verdict".to_string(), json!(parity_verdict));
    result_map.insert("live_read_shadow_parity_verdict".to_string(), json!(live_read_shadow_parity_verdict));
    result_map.insert("sources".to_string(), sources);
    result_map.insert("python_row_count".to_string(), json!(python_row_count));
    result_map.insert("rust_row_count".to_string(), json!(rust_row_count));
    result_map.insert("live_read_shadow_row_count".to_string(), json!(live_read_shadow_row_count));
    result_map.insert("production_row_authority".to_string(), json!(production_row_authority));
    result_map.insert("cleanup_row_authority".to_string(), json!(cleanup_row_authority));
    result_map.insert("diagnostic_row_authority".to_string(), json!(diagnostic_row_authority));
    result_map.insert("diagnostic_selection_only".to_string(), json!(diagnostic_selection_only));
    result_map.insert("diagnostic_selection_ready".to_string(), json!(diagnostic_selection_ready));
    result_map.insert("rust_diagnostic_selection_ready".to_string(), json!(rust_diagnostic_selection_ready));
    result_map.insert("rust_rows_selected_for_diagnostics".to_string(), json!(rust_rows_selected_for_diagnostics));
    result_map.insert("rust_rows_safe_for_observation".to_string(), json!(rust_rows_safe_for_observation));
    result_map.insert("rust_rows_may_feed_pilot_observation".to_string(), json!(ready && diagnostic_selection_ready));
    result_map.insert("collector_authority_switch_rehearsal".to_string(), switch_rehearsal);
    result_map.insert("full_rust_backend".to_string(), json!(false));
    result_map.insert("production_collector_authority_switched".to_string(), json!(false));
    result_map.insert("collector_authority_switch_supported".to_string(), json!(false));
    result_map.insert("collector_authority_switch_executed".to_string(), json!(false));
    result_map.insert("collector_authority_pilot_execution_supported".to_string(), json!(false));
    result_map.insert("collector_authority_pilot_execution_executed".to_string(), json!(false));
    result_map.insert("python_collector_fallback_required".to_string(), json!(true));
    result_map.insert("pilot_execution_contract_only".to_string(), json!(true));
    result_map.insert("rust_pilot_may_be_observed".to_string(), json!(ready && (!require_diagnostic_selection || diagnostic_selection_ready)));
    result_map.insert("rust_can_drive_cleanup".to_string(), json!(false));
    result_map.insert("rust_can_drive_apply".to_string(), json!(false));
    result_map.insert("rust_can_write_generated_files".to_string(), json!(false));
    result_map.insert("safe_for_cleanup".to_string(), json!(false));
    result_map.insert("write_allowed".to_string(), json!(false));
    result_map.insert("apply_allowed".to_string(), json!(false));
    result_map.insert("connection_attempt_count".to_string(), json!(0));
    result_map.insert("authentication_attempt_count".to_string(), json!(0));
    result_map.insert("api_sentence_write_count".to_string(), json!(0));
    result_map.insert("api_reply_read_count".to_string(), json!(0));
    result_map.insert("next_stage".to_string(), json!("rust_collector_authority_pilot_observation_window"));
    result_map.insert("note".to_string(), json!("v4.3.3 requires diagnostics-only Rust row selection before pilot observation readiness while preserving non-mutating fail-safe behavior."));

    let result = Value::Object(result_map);

    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn base_payload() -> Value {
        let row = json!({"Circuit ID":"selftest", "Circuit Name":"selftest", "Device ID":"selftest", "Device Name":"selftest", "Parent Node":"15M-RB5009", "MAC":"AA:BB:CC:DD:EE:FF", "IPv4":"10.0.0.2", "IPv6":"", "Download Min Mbps":"7.5", "Upload Min Mbps":"7.5", "Download Max Mbps":"15", "Upload Max Mbps":"15", "Comment":"PPP"});
        json!({
            "router": {"name":"RB5009", "address":"10.0.0.1", "port":8728, "username":"selftest", "password":"pilot-execution-password", "pppoe":{"per_plan_node":true, "plan_node_name":"{profile}-{router}"}},
            "sources": ["pppoe"],
            "defaults": {"default_pppoe_rate":"10M/10M", "min_rate_percentage":0.5},
            "collector_parity": {"parity_score": 100.0, "verdict":"parity_pass"},
            "python_rows": [row],
            "pppoe": {
                "active": [{"name":"selftest", "address":"10.0.0.2", "caller-id":"AA:BB:CC:DD:EE:FF"}],
                "secrets": [{"name":"selftest", "profile":"15M", "comment":"PLAN|15M/15M", "disabled":"false", "inactive":"false"}],
                "profiles": [{"name":"15M", "rate-limit":"15M/15M"}]
            }
        })
    }

    fn enable_all_gates(payload: &mut Value) {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("successful_shadow_cycles".to_string(), json!(3));
            obj.insert("shadow_age_seconds".to_string(), json!(30));
            obj.insert("confirmation".to_string(), json!("CONFIRM_COLLECTOR_AUTHORITY_PILOT_EXECUTION"));
            obj.insert("collector_authority_switch_confirmation".to_string(), json!("CONFIRM_COLLECTOR_AUTHORITY_REHEARSAL"));
            obj.insert("rust_core".to_string(), json!({
                "allow_rust_collector_authority": true,
                "rust_collector_authority_pilot": true,
                "allow_rust_routeros_live_read_adapter": true,
                "routeros_live_read_adapter_pilot": true,
                "rust_collector_authority_sources": ["pppoe"],
                "collector_authority_mode": "rust_collector_authority_pilot",
                "collector_authority_manifest_pilot": true,
                "allow_collector_authority_manifest": true,
                "collector_authority_dry_run_selection_pilot": true,
                "allow_collector_authority_dry_run_selection": true,
                "collector_authority_dry_run_bundle_pilot": true,
                "allow_collector_authority_dry_run_bundle": true,
                "run_cycle_rust_shadow_report_enabled": true,
                "run_cycle_rust_shadow_report_pilot": true,
                "collector_authority_activation_pilot": true,
                "allow_collector_authority_activation": true,
                "collector_authority_activation_mode": "rust_collector_authority_pilot",
                "collector_authority_require_python_fallback": true,
                "collector_authority_require_run_cycle_shadow": true,
                "collector_authority_min_shadow_cycles": 3,
                "collector_authority_runtime_pilot": true,
                "allow_collector_authority_runtime_contract": true,
                "collector_authority_runtime_mode": "rust_collector_authority_runtime_contract",
                "collector_authority_runtime_require_activation_plan": true,
                "collector_authority_runtime_require_python_fallback": true,
                "collector_authority_runtime_max_shadow_age_seconds": 900,
                "collector_authority_switch_rehearsal_pilot": true,
                "allow_collector_authority_switch_rehearsal": true,
                "collector_authority_switch_mode": "rust_collector_authority_switch_rehearsal",
                "collector_authority_switch_require_runtime_contract": true,
                "collector_authority_switch_require_python_fallback": true,
                "collector_authority_switch_require_manual_confirmation": true,
                "collector_authority_pilot_execution_pilot": true,
                "allow_collector_authority_pilot_execution_contract": true,
                "collector_authority_pilot_execution_mode": "rust_collector_authority_pilot_execution_contract",
                "collector_authority_pilot_execution_require_switch_rehearsal": true,
                "collector_authority_pilot_execution_require_python_fallback": true,
                "collector_authority_pilot_execution_require_manual_confirmation": true,
                "collector_authority_pilot_execution_require_diagnostic_selection": true,
                "collector_authority_pilot_execution_max_shadow_age_seconds": 900
            }));
        }
    }

    #[test]
    fn defaults_to_shadow_only_pilot_execution_contract() {
        let payload = base_payload();
        let (result, errors, _warnings) = build_collector_authority_pilot_execution_contract_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_pilot_execution_shadow_only"));
        assert_eq!(result.get("collector_authority").and_then(Value::as_str), Some("python_authoritative"));
        assert_eq!(result.get("production_collector_authority_switched").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn builds_ready_pilot_execution_contract_when_switch_and_gates_are_ready() {
        let mut payload = base_payload();
        enable_all_gates(&mut payload);
        let (result, errors, _warnings) = build_collector_authority_pilot_execution_contract_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_pilot_execution_contract_ready"));
        assert_eq!(result.get("pilot_execution_contract_only").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("collector_authority_pilot_execution_executed").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("diagnostic_row_authority").and_then(Value::as_str), Some("rust_shadow_diagnostics"));
        assert_eq!(result.get("rust_rows_may_feed_pilot_observation").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("rust_can_drive_cleanup").and_then(Value::as_bool), Some(false));
        let text = serde_json::to_string(&result).unwrap();
        assert!(!text.contains("pilot-execution-password"));
        assert!(!text.contains("\"password\":"));
    }

    #[test]
    fn requires_diagnostics_only_selection_before_observation_ready() {
        let mut payload = json!({
            "shadow_age_seconds": 30,
            "confirmation": "CONFIRM_COLLECTOR_AUTHORITY_PILOT_EXECUTION",
            "collector_authority_switch_rehearsal": {
                "status": "collector_authority_switch_rehearsal_ready",
                "production_collector_authority_switched": false,
                "collector_authority_switch_executed": false,
                "python_collector_fallback_required": true,
                "diagnostic_row_authority": "python_authoritative",
                "production_row_authority": "python_collector",
                "cleanup_row_authority": "python_collector",
                "diagnostic_selection_only": true,
                "rust_diagnostic_selection_ready": false,
                "rust_rows_selected_for_diagnostics": false,
                "rust_rows_safe_for_observation": false,
                "rust_row_count": 1,
                "python_row_count": 1
            }
        });
        enable_all_gates(&mut payload);

        let (result, errors, warnings) = build_collector_authority_pilot_execution_contract_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_pilot_execution_waiting_for_gates"));
        assert_eq!(result.get("diagnostic_selection_ready").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("rust_rows_may_feed_pilot_observation").and_then(Value::as_bool), Some(false));
        assert!(warnings.iter().any(|w| w.code == "collector_authority_pilot_execution_diagnostic_selection_not_ready"));
    }

    #[test]
    fn blocks_execute_attempts() {
        let mut payload = base_payload();
        enable_all_gates(&mut payload);
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("execute".to_string(), json!(true));
        }
        let (result, errors, _warnings) = build_collector_authority_pilot_execution_contract_payload(&payload);
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
    }
}
