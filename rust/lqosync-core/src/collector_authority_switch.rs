use crate::collector_authority_runtime::build_collector_authority_runtime_contract_payload;
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

fn switch_rehearsal_id(value: &Value) -> String {
    let text = serde_json::to_string(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("casr-{}", &digest[..16])
}

/// Build a non-mutating collector-authority switch rehearsal.
///
/// v4.2 is the bridge after the runtime contract. It creates an auditable
/// switch rehearsal that proves the runtime contract, manual confirmation, and
/// fallback guard shape before any future Rust collector authority handoff.
/// It never switches production authority, never drives cleanup, and never
/// allows Rust collector rows to write generated files in this release.
pub fn build_collector_authority_switch_rehearsal_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(str_value(payload.get("mode"), "rehearsal"), "execute" | "switch" | "promote" | "authority" | "apply" | "production");
    if requested_execute {
        errors.push(Diagnostic::error(
            "collector_authority_switch_execute_not_implemented",
            Some("collector_authority_switch".to_string()),
            "This release only builds a collector authority switch rehearsal. It does not switch production collector authority away from Python.",
        ));
    }

    let allow_rehearsal = bool_value(config_value(payload, "allow_collector_authority_switch_rehearsal"), false);
    let rehearsal_pilot = bool_value(config_value(payload, "collector_authority_switch_rehearsal_pilot"), false);
    let switch_mode = str_value(config_value(payload, "collector_authority_switch_mode"), "rehearsal_only");
    let require_runtime = bool_value(config_value(payload, "collector_authority_switch_require_runtime_contract"), true);
    let require_python_fallback = bool_value(config_value(payload, "collector_authority_switch_require_python_fallback"), true);
    let require_manual_confirmation = bool_value(config_value(payload, "collector_authority_switch_require_manual_confirmation"), true);
    let confirmation = str_value(payload.get("confirmation"), "");
    let confirmation_ok = !require_manual_confirmation || confirmation == "CONFIRM_COLLECTOR_AUTHORITY_REHEARSAL";

    let runtime_value = payload
        .get("collector_authority_runtime_contract")
        .and_then(|v| v.get("result"))
        .or_else(|| payload.get("collector_authority_runtime_contract"))
        .cloned();

    let (runtime_contract, runtime_errors, mut runtime_warnings) = match runtime_value {
        Some(v) if v.is_object() => (v, Vec::new(), Vec::new()),
        _ => build_collector_authority_runtime_contract_payload(payload),
    };
    warnings.append(&mut runtime_warnings);

    if !runtime_errors.is_empty() {
        warnings.push(Diagnostic::warning(
            "collector_authority_switch_runtime_not_clean",
            Some("collector_authority_runtime_contract".to_string()),
            "Collector authority runtime contract returned errors; switch rehearsal remains blocked.",
        ));
    }

    let runtime_status = runtime_contract.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let runtime_ready = runtime_errors.is_empty()
        && runtime_status == "collector_authority_runtime_contract_ready"
        && runtime_contract.get("production_collector_authority_switched").and_then(Value::as_bool) == Some(false)
        && runtime_contract.get("python_collector_fallback_required").and_then(Value::as_bool) == Some(true);

    if require_runtime && !runtime_ready {
        warnings.push(Diagnostic::warning(
            "collector_authority_switch_runtime_not_ready",
            Some("collector_authority_runtime_contract".to_string()),
            "Collector authority runtime contract is not ready; switch rehearsal remains shadow-only.",
        ));
    }
    if !require_python_fallback {
        errors.push(Diagnostic::error(
            "collector_authority_switch_requires_python_fallback",
            Some("rust_core.collector_authority_switch_require_python_fallback".to_string()),
            "Collector authority switch rehearsal requires Python collector fallback in this release.",
        ));
    }
    if !confirmation_ok {
        warnings.push(Diagnostic::warning(
            "collector_authority_switch_confirmation_missing",
            Some("collector_authority_switch.confirmation".to_string()),
            "Manual confirmation token is missing; switch rehearsal remains waiting for confirmation.",
        ));
    }

    let rehearsal_requested = allow_rehearsal && rehearsal_pilot && switch_mode == "rust_collector_authority_switch_rehearsal";
    let rehearsal_ready = errors.is_empty()
        && rehearsal_requested
        && (!require_runtime || runtime_ready)
        && require_python_fallback
        && confirmation_ok;

    let status = if !errors.is_empty() {
        "blocked"
    } else if rehearsal_ready {
        "collector_authority_switch_rehearsal_ready"
    } else if runtime_ready {
        "collector_authority_switch_rehearsal_waiting_for_gates"
    } else {
        "collector_authority_switch_rehearsal_shadow_only"
    };

    let rust_row_count = runtime_contract.get("rust_row_count").and_then(Value::as_u64).unwrap_or(0);
    let python_row_count = runtime_contract.get("python_row_count").and_then(Value::as_u64).unwrap_or(0);
    let runtime_contract_id = str_value(runtime_contract.get("runtime_contract_id"), "");
    let runtime_evidence_source = str_value(runtime_contract.get("runtime_evidence_source"), "not_ready");
    let rust_shadow_ready = bool_value(runtime_contract.get("rust_shadow_ready"), false);
    let dry_run_shadow_ready = bool_value(runtime_contract.get("dry_run_shadow_ready"), false);
    let live_read_shadow_ready = bool_value(runtime_contract.get("live_read_shadow_ready"), false);
    let shadow_history_successful_count = number_value(runtime_contract.get("shadow_history_successful_count"), 0);
    let live_read_shadow_row_count = number_value(runtime_contract.get("live_read_shadow_row_count"), 0);
    let parity_verdict = str_value(runtime_contract.get("parity_verdict"), "not_available");
    let live_read_shadow_parity_verdict = str_value(runtime_contract.get("live_read_shadow_parity_verdict"), "not_available");
    let runtime_may_select_diagnostics = bool_value(runtime_contract.get("rust_pilot_may_select_rows_for_diagnostics"), false);
    let rust_diagnostic_selection_ready = rehearsal_ready && runtime_may_select_diagnostics;
    let diagnostic_row_authority = if rust_diagnostic_selection_ready { "rust_shadow_diagnostics" } else { "python_authoritative" };
    let sources = payload.get("sources").cloned().unwrap_or_else(|| json!([]));

    let seed = json!({
        "status": status,
        "runtime_status": runtime_status,
        "switch_mode": switch_mode,
        "rust_row_count": rust_row_count,
        "runtime_evidence_source": runtime_evidence_source,
        "rust_diagnostic_selection_ready": rust_diagnostic_selection_ready,
        "confirmation_ok": confirmation_ok,
    });

    let result = json!({
        "mode": "collector_authority_switch_rehearsal",
        "status": status,
        "switch_rehearsal_id": switch_rehearsal_id(&seed),
        "collector_authority": "python_authoritative",
        "target_authority": if rehearsal_ready { "rust_collector_authority_rehearsal_candidate" } else { "python_authoritative" },
        "rehearsal_requested": rehearsal_requested,
        "allow_switch_rehearsal": allow_rehearsal,
        "switch_rehearsal_pilot": rehearsal_pilot,
        "switch_mode": switch_mode,
        "require_runtime_contract": require_runtime,
        "require_python_fallback": require_python_fallback,
        "require_manual_confirmation": require_manual_confirmation,
        "manual_confirmation_ok": confirmation_ok,
        "runtime_status": runtime_status,
        "runtime_ready": runtime_ready,
        "runtime_contract_id": runtime_contract_id,
        "runtime_evidence_source": runtime_evidence_source,
        "rust_shadow_ready": rust_shadow_ready,
        "dry_run_shadow_ready": dry_run_shadow_ready,
        "live_read_shadow_ready": live_read_shadow_ready,
        "shadow_history_successful_count": shadow_history_successful_count,
        "parity_verdict": parity_verdict,
        "live_read_shadow_parity_verdict": live_read_shadow_parity_verdict,
        "sources": sources,
        "python_row_count": python_row_count,
        "rust_row_count": rust_row_count,
        "live_read_shadow_row_count": live_read_shadow_row_count,
        "collector_authority_runtime_contract": runtime_contract,
        "production_row_authority": "python_collector",
        "cleanup_row_authority": "python_collector",
        "diagnostic_row_authority": diagnostic_row_authority,
        "diagnostic_selection_only": true,
        "rust_diagnostic_selection_ready": rust_diagnostic_selection_ready,
        "rust_rows_selected_for_diagnostics": rust_diagnostic_selection_ready,
        "rust_rows_safe_for_observation": rust_diagnostic_selection_ready,
        "runtime_may_select_rows_for_diagnostics": runtime_may_select_diagnostics,
        "full_rust_backend": false,
        "production_collector_authority_switched": false,
        "collector_authority_switch_supported": false,
        "collector_authority_switch_executed": false,
        "python_collector_fallback_required": true,
        "switch_rehearsal_only": true,
        "rust_pilot_may_be_compared_to_python": rehearsal_ready,
        "rust_can_drive_cleanup": false,
        "rust_can_drive_apply": false,
        "rust_can_write_generated_files": false,
        "safe_for_cleanup": false,
        "write_allowed": false,
        "apply_allowed": false,
        "connection_attempt_count": 0,
        "authentication_attempt_count": 0,
        "api_sentence_write_count": 0,
        "api_reply_read_count": 0,
        "next_stage": "rust_collector_authority_pilot_observation_window",
        "note": "v4.2 builds a non-mutating switch rehearsal after the runtime contract. It can select Rust shadow rows for diagnostics only; it does not switch production authority away from Python."
    });

    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn base_payload() -> Value {
        let row = json!({"Circuit ID":"selftest", "Circuit Name":"selftest", "Device ID":"selftest", "Device Name":"selftest", "Parent Node":"15M-RB5009", "MAC":"AA:BB:CC:DD:EE:FF", "IPv4":"10.0.0.2", "IPv6":"", "Download Min Mbps":"7.5", "Upload Min Mbps":"7.5", "Download Max Mbps":"15", "Upload Max Mbps":"15", "Comment":"PPP"});
        json!({
            "router": {"name":"RB5009", "address":"10.0.0.1", "port":8728, "username":"selftest", "password":"switch-rehearsal-password", "pppoe":{"per_plan_node":true, "plan_node_name":"{profile}-{router}"}},
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
            obj.insert("confirmation".to_string(), json!("CONFIRM_COLLECTOR_AUTHORITY_REHEARSAL"));
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
                "collector_authority_switch_require_manual_confirmation": true
            }));
        }
    }

    #[test]
    fn defaults_to_shadow_only_switch_rehearsal() {
        let payload = base_payload();
        let (result, errors, _warnings) = build_collector_authority_switch_rehearsal_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_switch_rehearsal_shadow_only"));
        assert_eq!(result.get("collector_authority").and_then(Value::as_str), Some("python_authoritative"));
        assert_eq!(result.get("production_collector_authority_switched").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn builds_ready_switch_rehearsal_when_runtime_and_gates_are_ready() {
        let mut payload = base_payload();
        enable_all_gates(&mut payload);
        let (result, errors, _warnings) = build_collector_authority_switch_rehearsal_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_switch_rehearsal_ready"));
        assert_eq!(result.get("switch_rehearsal_only").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("rust_can_drive_cleanup").and_then(Value::as_bool), Some(false));
        let text = serde_json::to_string(&result).unwrap();
        assert!(!text.contains("switch-rehearsal-password"));
        assert!(!text.contains("\"password\":"));
    }

    #[test]
    fn selects_live_read_shadow_rows_for_diagnostics_only() {
        let mut payload = json!({
            "sources": ["pppoe"],
            "collector_authority_runtime_contract": {
                "status": "collector_authority_runtime_contract_ready",
                "runtime_contract_id": "capr-live-shadow",
                "production_collector_authority_switched": false,
                "python_collector_fallback_required": true,
                "rust_pilot_may_select_rows_for_diagnostics": true,
                "runtime_evidence_source": "live_read_shadow_history",
                "rust_shadow_ready": true,
                "dry_run_shadow_ready": false,
                "live_read_shadow_ready": true,
                "shadow_history_successful_count": 3,
                "parity_verdict": "parity_pass",
                "live_read_shadow_parity_verdict": "parity_pass",
                "python_row_count": 1,
                "rust_row_count": 1,
                "live_read_shadow_row_count": 1
            }
        });
        enable_all_gates(&mut payload);

        let (result, errors, _warnings) = build_collector_authority_switch_rehearsal_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_switch_rehearsal_ready"));
        assert_eq!(result.get("runtime_evidence_source").and_then(Value::as_str), Some("live_read_shadow_history"));
        assert_eq!(result.get("diagnostic_row_authority").and_then(Value::as_str), Some("rust_shadow_diagnostics"));
        assert_eq!(result.get("production_row_authority").and_then(Value::as_str), Some("python_collector"));
        assert_eq!(result.get("rust_rows_selected_for_diagnostics").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("rust_rows_safe_for_observation").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("safe_for_cleanup").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("production_collector_authority_switched").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn blocks_execute_attempts() {
        let mut payload = base_payload();
        enable_all_gates(&mut payload);
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("execute".to_string(), json!(true));
        }
        let (result, errors, _warnings) = build_collector_authority_switch_rehearsal_payload(&payload);
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
    }
}
