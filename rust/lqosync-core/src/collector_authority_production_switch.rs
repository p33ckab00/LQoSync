use crate::collector_authority_production_freeze::build_collector_authority_production_freeze_gate_payload;
use crate::protocol::Diagnostic;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const CONFIRM_PRODUCTION_SWITCH_CONTRACT: &str = "CONFIRM_COLLECTOR_AUTHORITY_PRODUCTION_SWITCH_CONTRACT";
const CONFIRM_PRODUCTION_FREEZE_GATE: &str = "CONFIRM_COLLECTOR_AUTHORITY_PRODUCTION_FREEZE_GATE";

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

fn first_object<'a>(payload: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    for key in keys {
        if let Some(value) = payload.get(*key) {
            if let Some(result) = value.get("result") {
                if result.is_object() {
                    return Some(result);
                }
            }
            if value.is_object() {
                return Some(value);
            }
        }
    }
    None
}

fn switch_contract_id(seed: &Value) -> String {
    let text = serde_json::to_string(seed).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("capswitch-{}", &digest[..16])
}

/// Build a production collector-authority switch contract without executing it.
///
/// v5.0 is the first production-switch contract phase, but it remains
/// non-mutating. It can report that the switch contract is ready after the
/// v4.9 freeze gate, but it does not flip authority, remove Python, drive
/// cleanup/apply, or write generated LibreQoS files.
pub fn build_collector_authority_production_switch_contract_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(str_value(payload.get("mode"), "contract"), "execute" | "commit" | "switch" | "promote" | "authority" | "apply" | "production" | "cutover" | "remove-python");
    if requested_execute {
        errors.push(Diagnostic::error(
            "collector_authority_production_switch_execute_not_implemented",
            Some("collector_authority_production_switch_contract".to_string()),
            "This release only builds a production switch contract. It does not flip Rust collector authority, remove Python, drive cleanup, write files, or apply LibreQoS.",
        ));
    }

    let allow_contract = bool_value(config_value(payload, "allow_collector_authority_production_switch_contract"), false);
    let contract_pilot = bool_value(config_value(payload, "collector_authority_production_switch_contract_pilot"), false);
    let switch_mode = str_value(config_value(payload, "collector_authority_production_switch_mode"), "contract_only");
    let require_freeze_gate = bool_value(config_value(payload, "collector_authority_production_switch_require_freeze_gate"), true);
    let require_python_fallback = bool_value(config_value(payload, "collector_authority_production_switch_require_python_fallback"), true);
    let require_manual_confirmation = bool_value(config_value(payload, "collector_authority_production_switch_require_manual_confirmation"), true);
    let require_no_side_effects = bool_value(config_value(payload, "collector_authority_production_switch_require_no_cleanup_apply"), true);
    let require_rollback_path = bool_value(config_value(payload, "collector_authority_production_switch_require_rollback_path"), true);
    let require_maintenance_window = bool_value(config_value(payload, "collector_authority_production_switch_require_maintenance_window"), true);
    let require_operator_ack = bool_value(config_value(payload, "collector_authority_production_switch_require_operator_ack"), true);
    let max_shadow_age = number_value(config_value(payload, "collector_authority_production_switch_max_shadow_age_seconds"), 900);
    let shadow_age = number_value(payload.get("shadow_age_seconds"), 0);

    let confirmation = str_value(payload.get("confirmation"), "");
    let confirmation_ok = !require_manual_confirmation || confirmation == CONFIRM_PRODUCTION_SWITCH_CONTRACT;
    if require_manual_confirmation && !confirmation_ok {
        warnings.push(Diagnostic::warning(
            "collector_authority_production_switch_confirmation_required",
            Some("confirmation".to_string()),
            "Production switch contract requires CONFIRM_COLLECTOR_AUTHORITY_PRODUCTION_SWITCH_CONTRACT before it can report ready.",
        ));
    }

    if !require_python_fallback {
        errors.push(Diagnostic::error(
            "collector_authority_production_switch_requires_python_fallback",
            Some("rust_core.collector_authority_production_switch_require_python_fallback".to_string()),
            "v5.0 still requires Python collector fallback. Removing Python belongs to later full-Rust backend phases after scheduler, run_cycle, API, apply, journal, and rollback authority are Rust-owned.",
        ));
    }

    if shadow_age > max_shadow_age {
        warnings.push(Diagnostic::warning(
            "collector_authority_production_switch_shadow_stale",
            Some("shadow_age_seconds".to_string()),
            "Rust-shadow collector data is older than the configured maximum age; production switch contract remains under review.",
        ).with_value(json!({"shadow_age_seconds": shadow_age, "max_shadow_age_seconds": max_shadow_age})));
    }

    let freeze_value = first_object(payload, &[
        "collector_authority_production_freeze_gate",
        "production_freeze_gate",
        "collector_authority_freeze_gate",
    ]).cloned();

    let (freeze_gate, freeze_errors, mut freeze_warnings) = match freeze_value {
        Some(v) => (v, Vec::new(), Vec::new()),
        None => {
            let mut nested_payload = payload.clone();
            if let Some(obj) = nested_payload.as_object_mut() {
                let freeze_confirmation = str_value(
                    payload.get("collector_authority_production_freeze_confirmation"),
                    CONFIRM_PRODUCTION_FREEZE_GATE,
                );
                obj.insert("confirmation".to_string(), json!(freeze_confirmation));
            }
            build_collector_authority_production_freeze_gate_payload(&nested_payload)
        }
    };
    warnings.append(&mut freeze_warnings);

    if !freeze_errors.is_empty() {
        warnings.push(Diagnostic::warning(
            "collector_authority_production_switch_freeze_not_clean",
            Some("collector_authority_production_freeze_gate".to_string()),
            "Production freeze gate returned errors; production switch contract remains shadow-only.",
        ));
    }

    let freeze_status = freeze_gate.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let freeze_ready = freeze_errors.is_empty()
        && freeze_status == "collector_authority_production_freeze_gate_ready"
        && freeze_gate.get("production_freeze_ready").and_then(Value::as_bool).unwrap_or(false)
        && freeze_gate.get("production_collector_authority_switched").and_then(Value::as_bool) == Some(false)
        && freeze_gate.get("collector_authority_production_switch_executed").and_then(Value::as_bool).unwrap_or(false) == false
        && freeze_gate.get("python_backend_removable").and_then(Value::as_bool).unwrap_or(false) == false
        && freeze_gate.get("python_collector_fallback_required").and_then(Value::as_bool).unwrap_or(true);

    if require_freeze_gate && !freeze_ready {
        warnings.push(Diagnostic::warning(
            "collector_authority_production_switch_freeze_not_ready",
            Some("collector_authority_production_freeze_gate".to_string()),
            "Production freeze gate has not passed; production switch contract remains shadow-only or under review.",
        ));
    }

    let cleanup_attempted = bool_value(payload.get("cleanup_attempted"), false)
        || bool_value(freeze_gate.get("cleanup_attempted"), false);
    let apply_attempted = bool_value(payload.get("apply_attempted"), false)
        || bool_value(freeze_gate.get("apply_attempted"), false);
    let write_attempted = bool_value(payload.get("write_attempted"), false)
        || bool_value(freeze_gate.get("write_attempted"), false);
    let authority_switched = bool_value(payload.get("production_collector_authority_switched"), false)
        || bool_value(freeze_gate.get("production_collector_authority_switched"), false);
    let side_effect_free = !cleanup_attempted && !apply_attempted && !write_attempted && !authority_switched;

    if require_no_side_effects && !side_effect_free {
        errors.push(Diagnostic::error(
            "collector_authority_production_switch_side_effect_detected",
            Some("collector_authority_production_switch_contract".to_string()),
            "Production switch contract detected cleanup/apply/write/authority side effects, which are forbidden in this release.",
        ));
    }

    let rollback_path = payload
        .get("rollback_path")
        .and_then(Value::as_str)
        .or_else(|| freeze_gate.get("rollback_path").and_then(Value::as_str))
        .unwrap_or("python_fallback_revert");
    let rollback_ready = !require_rollback_path || !rollback_path.trim().is_empty();
    if require_rollback_path && !rollback_ready {
        warnings.push(Diagnostic::warning(
            "collector_authority_production_switch_rollback_path_required",
            Some("rollback_path".to_string()),
            "Production switch contract requires an explicit rollback path before it can report ready.",
        ));
    }

    let maintenance_window = payload
        .get("maintenance_window")
        .and_then(Value::as_str)
        .or_else(|| freeze_gate.get("maintenance_window").and_then(Value::as_str))
        .unwrap_or("");
    let maintenance_window_ready = !require_maintenance_window || !maintenance_window.trim().is_empty();
    if require_maintenance_window && !maintenance_window_ready {
        warnings.push(Diagnostic::warning(
            "collector_authority_production_switch_maintenance_window_required",
            Some("maintenance_window".to_string()),
            "Production switch contract requires a maintenance window before it can report ready.",
        ));
    }

    let operator_ack = bool_value(payload.get("operator_acknowledged"), false)
        || bool_value(freeze_gate.get("operator_acknowledged"), false);
    let operator_ack_ready = !require_operator_ack || operator_ack;
    if require_operator_ack && !operator_ack_ready {
        warnings.push(Diagnostic::warning(
            "collector_authority_production_switch_operator_ack_required",
            Some("operator_acknowledged".to_string()),
            "Production switch contract requires explicit operator acknowledgment before it can report ready.",
        ));
    }

    let gates_ready = allow_contract && contract_pilot && switch_mode == "contract_only";
    if !gates_ready {
        warnings.push(Diagnostic::warning(
            "collector_authority_production_switch_gates_not_enabled",
            Some("rust_core".to_string()),
            "Production switch contract gates are not fully enabled; report remains shadow-only.",
        ));
    }

    let ready = errors.is_empty()
        && gates_ready
        && confirmation_ok
        && (!require_freeze_gate || freeze_ready)
        && shadow_age <= max_shadow_age
        && side_effect_free
        && require_python_fallback
        && rollback_ready
        && maintenance_window_ready
        && operator_ack_ready;

    let review = errors.is_empty() && freeze_ready && side_effect_free && rollback_ready && maintenance_window_ready && operator_ack_ready;
    let status = if !errors.is_empty() {
        "blocked"
    } else if ready {
        "collector_authority_production_switch_contract_ready"
    } else if review {
        "collector_authority_production_switch_contract_review"
    } else {
        "collector_authority_production_switch_contract_shadow_only"
    };

    let mut seed = Map::new();
    seed.insert("status".to_string(), json!(status));
    seed.insert("freeze_status".to_string(), json!(freeze_status));
    seed.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    seed.insert("confirmation_ok".to_string(), json!(confirmation_ok));

    let mut contract_steps = Vec::new();
    let mut step1 = Map::new();
    step1.insert("step".to_string(), json!(1));
    step1.insert("name".to_string(), json!("verify_freeze_gate"));
    step1.insert("mutating".to_string(), json!(false));
    contract_steps.push(Value::Object(step1));
    let mut step2 = Map::new();
    step2.insert("step".to_string(), json!(2));
    step2.insert("name".to_string(), json!("prepare_collector_authority_switch_contract"));
    step2.insert("mutating".to_string(), json!(false));
    contract_steps.push(Value::Object(step2));
    let mut step3 = Map::new();
    step3.insert("step".to_string(), json!(3));
    step3.insert("name".to_string(), json!("keep_python_backend_and_fallback_enabled"));
    step3.insert("mutating".to_string(), json!(false));
    contract_steps.push(Value::Object(step3));

    let mut map = Map::new();
    map.insert("mode".to_string(), json!("collector_authority_production_switch_contract"));
    map.insert("status".to_string(), json!(status));
    map.insert("production_switch_contract_id".to_string(), json!(switch_contract_id(&Value::Object(seed))));
    map.insert("collector_authority".to_string(), json!("python_authoritative"));
    map.insert("target_collector_authority".to_string(), json!(if ready { "rust_collector_authority_contract_ready" } else { "python_authoritative" }));
    map.insert("full_rust_backend".to_string(), json!(false));
    map.insert("production_switch_contract_only".to_string(), json!(true));
    map.insert("production_switch_contract_ready".to_string(), json!(ready));
    map.insert("collector_authority_production_switch_supported".to_string(), json!(true));
    map.insert("collector_authority_production_switch_executed".to_string(), json!(false));
    map.insert("production_collector_authority_switched".to_string(), json!(false));
    map.insert("python_backend_removable".to_string(), json!(false));
    map.insert("python_backend_required".to_string(), json!(true));
    map.insert("python_collector_fallback_required".to_string(), json!(true));
    map.insert("rust_can_drive_cleanup".to_string(), json!(false));
    map.insert("rust_can_drive_apply".to_string(), json!(false));
    map.insert("rust_can_write_generated_files".to_string(), json!(false));
    map.insert("safe_for_cleanup".to_string(), json!(false));
    map.insert("write_allowed".to_string(), json!(false));
    map.insert("apply_allowed".to_string(), json!(false));
    map.insert("manual_confirmation_required".to_string(), json!(require_manual_confirmation));
    map.insert("manual_confirmation_accepted".to_string(), json!(confirmation_ok));
    map.insert("gates_ready".to_string(), json!(gates_ready));
    map.insert("production_freeze_status".to_string(), json!(freeze_status));
    map.insert("production_freeze_ready".to_string(), json!(freeze_ready));
    map.insert("rollback_path".to_string(), json!(rollback_path));
    map.insert("rollback_ready".to_string(), json!(rollback_ready));
    map.insert("maintenance_window".to_string(), json!(maintenance_window));
    map.insert("maintenance_window_ready".to_string(), json!(maintenance_window_ready));
    map.insert("operator_acknowledged".to_string(), json!(operator_ack));
    map.insert("operator_ack_ready".to_string(), json!(operator_ack_ready));
    map.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    map.insert("max_shadow_age_seconds".to_string(), json!(max_shadow_age));
    map.insert("side_effect_free".to_string(), json!(side_effect_free));
    map.insert("contract_steps".to_string(), Value::Array(contract_steps));
    map.insert("connection_attempt_count".to_string(), json!(0));
    map.insert("authentication_attempt_count".to_string(), json!(0));
    map.insert("api_sentence_write_count".to_string(), json!(0));
    map.insert("api_reply_read_count".to_string(), json!(0));
    map.insert("next_stage".to_string(), json!("rust_collector_authority_switch_executor"));
    map.insert("note".to_string(), json!("v5.0 builds the first production collector-authority switch contract after the freeze gate, but it does not execute the switch or remove Python."));

    (Value::Object(map), errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_payload() -> Value {
        let mut root = Map::new();
        root.insert("confirmation".to_string(), json!(CONFIRM_PRODUCTION_SWITCH_CONTRACT));
        root.insert("shadow_age_seconds".to_string(), json!(10));
        root.insert("maintenance_window".to_string(), json!("2026-05-20T23:00:00+08:00/PT30M"));
        root.insert("operator_acknowledged".to_string(), json!(true));
        root.insert("rollback_path".to_string(), json!("python_fallback_revert"));

        let mut rust_core = Map::new();
        rust_core.insert("allow_collector_authority_production_switch_contract".to_string(), json!(true));
        rust_core.insert("collector_authority_production_switch_contract_pilot".to_string(), json!(true));
        rust_core.insert("collector_authority_production_switch_mode".to_string(), json!("contract_only"));
        rust_core.insert("collector_authority_production_switch_require_freeze_gate".to_string(), json!(true));
        rust_core.insert("collector_authority_production_switch_require_python_fallback".to_string(), json!(true));
        rust_core.insert("collector_authority_production_switch_require_manual_confirmation".to_string(), json!(true));
        rust_core.insert("collector_authority_production_switch_require_no_cleanup_apply".to_string(), json!(true));
        rust_core.insert("collector_authority_production_switch_require_rollback_path".to_string(), json!(true));
        rust_core.insert("collector_authority_production_switch_require_maintenance_window".to_string(), json!(true));
        rust_core.insert("collector_authority_production_switch_require_operator_ack".to_string(), json!(true));
        rust_core.insert("collector_authority_production_switch_max_shadow_age_seconds".to_string(), json!(900));
        root.insert("rust_core".to_string(), Value::Object(rust_core));

        let mut freeze = Map::new();
        freeze.insert("status".to_string(), json!("collector_authority_production_freeze_gate_ready"));
        freeze.insert("production_freeze_ready".to_string(), json!(true));
        freeze.insert("python_collector_fallback_required".to_string(), json!(true));
        freeze.insert("python_backend_removable".to_string(), json!(false));
        freeze.insert("production_collector_authority_switched".to_string(), json!(false));
        freeze.insert("collector_authority_production_switch_executed".to_string(), json!(false));
        freeze.insert("rollback_path".to_string(), json!("python_fallback_revert"));
        freeze.insert("maintenance_window".to_string(), json!("2026-05-20T23:00:00+08:00/PT30M"));
        freeze.insert("operator_acknowledged".to_string(), json!(true));
        freeze.insert("cleanup_attempted".to_string(), json!(false));
        freeze.insert("apply_attempted".to_string(), json!(false));
        freeze.insert("write_attempted".to_string(), json!(false));
        root.insert("collector_authority_production_freeze_gate".to_string(), Value::Object(freeze));

        Value::Object(root)
    }

    #[test]
    fn defaults_to_shadow_only_production_switch_contract() {
        let (result, errors, _warnings) = build_collector_authority_production_switch_contract_payload(&json!({}));
        assert!(errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_production_switch_contract_shadow_only"));
        assert_eq!(result.get("production_collector_authority_switched").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("python_backend_removable").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn blocks_execute_attempts() {
        let (result, errors, _warnings) = build_collector_authority_production_switch_contract_payload(&json!({"execute": true}));
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
    }

    #[test]
    fn builds_ready_switch_contract_without_switching_authority_or_removing_python() {
        let payload = ready_payload();
        let (result, errors, _warnings) = build_collector_authority_production_switch_contract_payload(&payload);
        assert!(errors.is_empty(), "errors: {errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_production_switch_contract_ready"));
        assert_eq!(result.get("production_switch_contract_ready").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("production_collector_authority_switched").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("collector_authority_production_switch_executed").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("python_backend_removable").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("python_backend_required").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("rust_can_drive_cleanup").and_then(Value::as_bool), Some(false));
    }
}
