use crate::protocol::Diagnostic;
use crate::rust_run_cycle_orchestrator_handoff::build_rust_run_cycle_orchestrator_handoff_contract_payload;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const CONFIRM_CONFIG_STATE_AUTHORITY_HANDOFF: &str = "CONFIRM_RUST_CONFIG_STATE_AUTHORITY_HANDOFF_CONTRACT";
const CONFIRM_RUN_CYCLE_ORCHESTRATOR_HANDOFF: &str = "CONFIRM_RUST_RUN_CYCLE_ORCHESTRATOR_HANDOFF_CONTRACT";

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

fn handoff_id(seed: &Value) -> String {
    let text = serde_json::to_string(seed).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("cshandoff-{}", &digest[..16])
}

/// Build a Rust config/state authority handoff contract while keeping Python authoritative.
///
/// v5.4 continues the full-Rust-backend track after the run_cycle orchestrator
/// handoff. It verifies that config/state shadow writes, atomic writer paths,
/// transaction journal previews, audit previews, and rollback-manifest previews
/// are ready, but it does not switch config/state authority to Rust and does not
/// remove the Python backend.
pub fn build_rust_config_state_authority_handoff_contract_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(
            str_value(payload.get("mode"), "contract"),
            "execute" | "commit" | "switch" | "remove-python" | "replace-config-state" | "production" | "authoritative" | "write"
        );
    if requested_execute {
        errors.push(Diagnostic::error(
            "rust_config_state_authority_handoff_execute_not_implemented",
            Some("rust_config_state_authority_handoff_contract".to_string()),
            "This release only builds a Rust config/state authority handoff contract. It does not switch config/state authority or remove Python.",
        ));
    }

    let allow_contract = bool_value(config_value(payload, "allow_rust_config_state_authority_handoff_contract"), false);
    let contract_pilot = bool_value(config_value(payload, "rust_config_state_authority_handoff_contract_pilot"), false);
    let handoff_mode = str_value(config_value(payload, "rust_config_state_authority_handoff_mode"), "contract_only");
    let require_run_cycle_handoff = bool_value(config_value(payload, "rust_config_state_authority_handoff_require_run_cycle_orchestrator"), true);
    let require_python_fallback = bool_value(config_value(payload, "rust_config_state_authority_handoff_require_python_fallback"), true);
    let require_manual_confirmation = bool_value(config_value(payload, "rust_config_state_authority_handoff_require_manual_confirmation"), true);
    let require_config_state_shadow = bool_value(config_value(payload, "rust_config_state_authority_handoff_require_config_state_shadow"), true);
    let require_atomic_writer_shadow = bool_value(config_value(payload, "rust_config_state_authority_handoff_require_atomic_writer_shadow"), true);
    let require_transaction_journal_shadow = bool_value(config_value(payload, "rust_config_state_authority_handoff_require_transaction_journal_shadow"), true);
    let require_audit_shadow = bool_value(config_value(payload, "rust_config_state_authority_handoff_require_audit_shadow"), true);
    let require_rollback_shadow = bool_value(config_value(payload, "rust_config_state_authority_handoff_require_rollback_shadow"), true);
    let require_no_side_effects = bool_value(config_value(payload, "rust_config_state_authority_handoff_require_no_side_effects"), true);
    let max_shadow_age = number_value(config_value(payload, "rust_config_state_authority_handoff_max_shadow_age_seconds"), 900);
    let shadow_age = number_value(payload.get("shadow_age_seconds"), 0);

    let confirmation = str_value(payload.get("confirmation"), "");
    let confirmation_ok = !require_manual_confirmation || confirmation == CONFIRM_CONFIG_STATE_AUTHORITY_HANDOFF;
    if require_manual_confirmation && !confirmation_ok {
        warnings.push(Diagnostic::warning(
            "rust_config_state_authority_handoff_confirmation_required",
            Some("confirmation".to_string()),
            "Rust config/state authority handoff requires CONFIRM_RUST_CONFIG_STATE_AUTHORITY_HANDOFF_CONTRACT before it can report ready.",
        ));
    }

    if !require_python_fallback {
        errors.push(Diagnostic::error(
            "rust_config_state_authority_handoff_requires_python_fallback",
            Some("rust_core.rust_config_state_authority_handoff_require_python_fallback".to_string()),
            "v5.4 still requires Python config/state backend as fallback. Python removal belongs to a later authority execution phase.",
        ));
    }

    if shadow_age > max_shadow_age {
        warnings.push(Diagnostic::warning(
            "rust_config_state_authority_handoff_shadow_stale",
            Some("shadow_age_seconds".to_string()),
            "Rust-shadow data is older than the configured maximum age; config/state authority handoff remains under review.",
        ).with_value(json!({"shadow_age_seconds": shadow_age, "max_shadow_age_seconds": max_shadow_age})));
    }

    let run_cycle_handoff_value = first_object(payload, &[
        "rust_run_cycle_orchestrator_handoff_contract",
        "run_cycle_orchestrator_handoff_contract",
        "rust_run_cycle_orchestrator_handoff",
    ]).cloned();

    let (run_cycle_handoff, run_cycle_errors, mut run_cycle_warnings) = match run_cycle_handoff_value {
        Some(v) => (v, Vec::new(), Vec::new()),
        None => {
            let mut nested_payload = payload.clone();
            if let Some(obj) = nested_payload.as_object_mut() {
                let run_cycle_confirmation = str_value(
                    payload.get("rust_run_cycle_orchestrator_handoff_confirmation"),
                    CONFIRM_RUN_CYCLE_ORCHESTRATOR_HANDOFF,
                );
                obj.insert("confirmation".to_string(), json!(run_cycle_confirmation));
            }
            build_rust_run_cycle_orchestrator_handoff_contract_payload(&nested_payload)
        }
    };
    warnings.append(&mut run_cycle_warnings);

    if !run_cycle_errors.is_empty() {
        warnings.push(Diagnostic::warning(
            "rust_config_state_authority_handoff_run_cycle_not_clean",
            Some("rust_run_cycle_orchestrator_handoff_contract".to_string()),
            "Rust run_cycle orchestrator handoff returned errors; config/state authority handoff remains shadow-only.",
        ));
    }

    let run_cycle_status = run_cycle_handoff.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let run_cycle_ready = run_cycle_errors.is_empty()
        && run_cycle_status == "rust_run_cycle_orchestrator_handoff_contract_ready"
        && run_cycle_handoff.get("rust_run_cycle_orchestrator_handoff_ready").and_then(Value::as_bool).unwrap_or(false)
        && run_cycle_handoff.get("rust_run_cycle_authoritative").and_then(Value::as_bool).unwrap_or(false) == false
        && run_cycle_handoff.get("python_run_cycle_authoritative").and_then(Value::as_bool).unwrap_or(true);

    if require_run_cycle_handoff && !run_cycle_ready {
        warnings.push(Diagnostic::warning(
            "rust_config_state_authority_handoff_run_cycle_not_ready",
            Some("rust_run_cycle_orchestrator_handoff_contract".to_string()),
            "Rust run_cycle orchestrator handoff contract has not passed; config/state authority handoff remains shadow-only or under review.",
        ));
    }

    let config_state_shadow_ready = bool_value(payload.get("config_state_shadow_ready"), false);
    let config_state_shadow_count = number_value(payload.get("config_state_shadow_count"), 0);
    let config_state_ready = !require_config_state_shadow || (config_state_shadow_ready && config_state_shadow_count > 0);
    if require_config_state_shadow && !config_state_ready {
        warnings.push(Diagnostic::warning(
            "rust_config_state_authority_handoff_config_state_shadow_required",
            Some("config_state_shadow_ready".to_string()),
            "Rust config/state authority handoff requires config/state shadow verification before it can report ready.",
        ));
    }

    let atomic_writer_shadow_ready = bool_value(payload.get("atomic_writer_shadow_ready"), false);
    let atomic_writer_shadow_count = number_value(payload.get("atomic_writer_shadow_count"), 0);
    let atomic_writer_ready = !require_atomic_writer_shadow || (atomic_writer_shadow_ready && atomic_writer_shadow_count > 0);
    if require_atomic_writer_shadow && !atomic_writer_ready {
        warnings.push(Diagnostic::warning(
            "rust_config_state_authority_handoff_atomic_writer_shadow_required",
            Some("atomic_writer_shadow_ready".to_string()),
            "Rust config/state authority handoff requires atomic writer shadow verification before it can report ready.",
        ));
    }

    let transaction_journal_shadow_ready = bool_value(payload.get("transaction_journal_shadow_ready"), false);
    let transaction_journal_shadow_count = number_value(payload.get("transaction_journal_shadow_count"), 0);
    let transaction_journal_ready = !require_transaction_journal_shadow || (transaction_journal_shadow_ready && transaction_journal_shadow_count > 0);
    if require_transaction_journal_shadow && !transaction_journal_ready {
        warnings.push(Diagnostic::warning(
            "rust_config_state_authority_handoff_transaction_journal_shadow_required",
            Some("transaction_journal_shadow_ready".to_string()),
            "Rust config/state authority handoff requires transaction journal shadow verification before it can report ready.",
        ));
    }

    let audit_shadow_ready = bool_value(payload.get("audit_shadow_ready"), false);
    let audit_shadow_count = number_value(payload.get("audit_shadow_count"), 0);
    let audit_ready = !require_audit_shadow || (audit_shadow_ready && audit_shadow_count > 0);
    if require_audit_shadow && !audit_ready {
        warnings.push(Diagnostic::warning(
            "rust_config_state_authority_handoff_audit_shadow_required",
            Some("audit_shadow_ready".to_string()),
            "Rust config/state authority handoff requires audit shadow verification before it can report ready.",
        ));
    }

    let rollback_shadow_ready = bool_value(payload.get("rollback_manifest_shadow_ready"), false);
    let rollback_shadow_count = number_value(payload.get("rollback_manifest_shadow_count"), 0);
    let rollback_ready = !require_rollback_shadow || (rollback_shadow_ready && rollback_shadow_count > 0);
    if require_rollback_shadow && !rollback_ready {
        warnings.push(Diagnostic::warning(
            "rust_config_state_authority_handoff_rollback_shadow_required",
            Some("rollback_manifest_shadow_ready".to_string()),
            "Rust config/state authority handoff requires rollback-manifest shadow verification before it can report ready.",
        ));
    }

    let config_write_attempted = bool_value(payload.get("config_write_attempted"), false);
    let state_write_attempted = bool_value(payload.get("state_write_attempted"), false);
    let audit_write_attempted = bool_value(payload.get("audit_write_attempted"), false);
    let journal_write_attempted = bool_value(payload.get("journal_write_attempted"), false);
    let python_removed = bool_value(payload.get("python_backend_removed"), false);
    let authority_switched = bool_value(payload.get("config_state_authority_switched_to_rust"), false);
    let side_effect_free = !config_write_attempted && !state_write_attempted && !audit_write_attempted && !journal_write_attempted && !python_removed && !authority_switched;

    if require_no_side_effects && !side_effect_free {
        errors.push(Diagnostic::error(
            "rust_config_state_authority_handoff_side_effect_detected",
            Some("rust_config_state_authority_handoff_contract".to_string()),
            "Config/state handoff detected config/state/audit/journal writes, Python removal, or authority switch side effects, which are forbidden in this release.",
        ));
    }

    let gates_ready = allow_contract && contract_pilot && handoff_mode == "contract_only";
    if !gates_ready {
        warnings.push(Diagnostic::warning(
            "rust_config_state_authority_handoff_gates_not_enabled",
            Some("rust_core".to_string()),
            "Rust config/state authority handoff gates are not fully enabled; report remains shadow-only.",
        ));
    }

    let ready = errors.is_empty()
        && gates_ready
        && confirmation_ok
        && (!require_run_cycle_handoff || run_cycle_ready)
        && shadow_age <= max_shadow_age
        && require_python_fallback
        && config_state_ready
        && atomic_writer_ready
        && transaction_journal_ready
        && audit_ready
        && rollback_ready
        && side_effect_free;

    let review = errors.is_empty() && run_cycle_ready && config_state_ready && atomic_writer_ready && transaction_journal_ready && audit_ready && rollback_ready && side_effect_free;
    let status = if !errors.is_empty() {
        "blocked"
    } else if ready {
        "rust_config_state_authority_handoff_contract_ready"
    } else if review {
        "rust_config_state_authority_handoff_contract_review"
    } else {
        "rust_config_state_authority_handoff_contract_shadow_only"
    };

    let mut seed = Map::new();
    seed.insert("status".to_string(), json!(status));
    seed.insert("run_cycle_status".to_string(), json!(run_cycle_status));
    seed.insert("config_state_shadow_count".to_string(), json!(config_state_shadow_count));
    seed.insert("transaction_journal_shadow_count".to_string(), json!(transaction_journal_shadow_count));

    let mut handoff_steps = Vec::new();
    for (idx, name) in [
        "mirror_python_config_state_inputs",
        "shadow_validate_atomic_writes",
        "shadow_validate_transaction_journal",
        "shadow_validate_audit_and_rollback",
        "keep_python_config_state_authoritative",
        "defer_config_state_switch_to_future_release",
    ].iter().enumerate() {
        let mut step = Map::new();
        step.insert("step".to_string(), json!(idx + 1));
        step.insert("name".to_string(), json!(name));
        step.insert("mutating".to_string(), json!(false));
        handoff_steps.push(Value::Object(step));
    }

    let mut map = Map::new();
    map.insert("mode".to_string(), json!("rust_config_state_authority_handoff_contract"));
    map.insert("status".to_string(), json!(status));
    map.insert("config_state_authority_handoff_contract_id".to_string(), json!(handoff_id(&Value::Object(seed))));
    map.insert("rust_config_state_authority_handoff_ready".to_string(), json!(ready));
    map.insert("run_cycle_orchestrator_handoff_status".to_string(), json!(run_cycle_status));
    map.insert("run_cycle_orchestrator_handoff_ready".to_string(), json!(run_cycle_ready));
    map.insert("config_state_shadow_ready".to_string(), json!(config_state_ready));
    map.insert("config_state_shadow_count".to_string(), json!(config_state_shadow_count));
    map.insert("atomic_writer_shadow_ready".to_string(), json!(atomic_writer_ready));
    map.insert("atomic_writer_shadow_count".to_string(), json!(atomic_writer_shadow_count));
    map.insert("transaction_journal_shadow_ready".to_string(), json!(transaction_journal_ready));
    map.insert("transaction_journal_shadow_count".to_string(), json!(transaction_journal_shadow_count));
    map.insert("audit_shadow_ready".to_string(), json!(audit_ready));
    map.insert("audit_shadow_count".to_string(), json!(audit_shadow_count));
    map.insert("rollback_manifest_shadow_ready".to_string(), json!(rollback_ready));
    map.insert("rollback_manifest_shadow_count".to_string(), json!(rollback_shadow_count));
    map.insert("config_state_handoff_steps".to_string(), Value::Array(handoff_steps));
    map.insert("webui_ux_unchanged".to_string(), json!(true));
    map.insert("full_rust_backend".to_string(), json!(false));
    map.insert("python_backend_removable".to_string(), json!(false));
    map.insert("python_backend_removed".to_string(), json!(false));
    map.insert("python_backend_required".to_string(), json!(true));
    map.insert("python_backend_fallback_required".to_string(), json!(true));
    map.insert("python_config_state_authoritative".to_string(), json!(true));
    map.insert("rust_config_state_authoritative".to_string(), json!(false));
    map.insert("rust_run_cycle_authoritative".to_string(), json!(false));
    map.insert("rust_api_service_authoritative".to_string(), json!(false));
    map.insert("rust_apply_authoritative".to_string(), json!(false));
    map.insert("rust_can_drive_cleanup".to_string(), json!(false));
    map.insert("rust_can_drive_apply".to_string(), json!(false));
    map.insert("rust_can_write_generated_files".to_string(), json!(false));
    map.insert("safe_for_cleanup".to_string(), json!(false));
    map.insert("write_allowed".to_string(), json!(false));
    map.insert("apply_allowed".to_string(), json!(false));
    map.insert("manual_confirmation_required".to_string(), json!(require_manual_confirmation));
    map.insert("manual_confirmation_accepted".to_string(), json!(confirmation_ok));
    map.insert("gates_ready".to_string(), json!(gates_ready));
    map.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    map.insert("max_shadow_age_seconds".to_string(), json!(max_shadow_age));
    map.insert("side_effect_free".to_string(), json!(side_effect_free));
    map.insert("next_stage".to_string(), json!("rust_live_collector_execution_authority_handoff_contract"));
    map.insert("note".to_string(), json!("v5.4 builds the Rust config/state authority handoff contract while keeping Python config/state authoritative and preserving the existing WebUI/UX."));

    (Value::Object(map), errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_payload() -> Value {
        let mut root = Map::new();
        root.insert("confirmation".to_string(), json!(CONFIRM_CONFIG_STATE_AUTHORITY_HANDOFF));
        root.insert("shadow_age_seconds".to_string(), json!(10));
        root.insert("config_state_shadow_ready".to_string(), json!(true));
        root.insert("config_state_shadow_count".to_string(), json!(3));
        root.insert("atomic_writer_shadow_ready".to_string(), json!(true));
        root.insert("atomic_writer_shadow_count".to_string(), json!(3));
        root.insert("transaction_journal_shadow_ready".to_string(), json!(true));
        root.insert("transaction_journal_shadow_count".to_string(), json!(3));
        root.insert("audit_shadow_ready".to_string(), json!(true));
        root.insert("audit_shadow_count".to_string(), json!(3));
        root.insert("rollback_manifest_shadow_ready".to_string(), json!(true));
        root.insert("rollback_manifest_shadow_count".to_string(), json!(3));

        let mut rust_core = Map::new();
        rust_core.insert("allow_rust_config_state_authority_handoff_contract".to_string(), json!(true));
        rust_core.insert("rust_config_state_authority_handoff_contract_pilot".to_string(), json!(true));
        rust_core.insert("rust_config_state_authority_handoff_mode".to_string(), json!("contract_only"));
        rust_core.insert("rust_config_state_authority_handoff_require_run_cycle_orchestrator".to_string(), json!(true));
        rust_core.insert("rust_config_state_authority_handoff_require_python_fallback".to_string(), json!(true));
        rust_core.insert("rust_config_state_authority_handoff_require_manual_confirmation".to_string(), json!(true));
        rust_core.insert("rust_config_state_authority_handoff_require_config_state_shadow".to_string(), json!(true));
        rust_core.insert("rust_config_state_authority_handoff_require_atomic_writer_shadow".to_string(), json!(true));
        rust_core.insert("rust_config_state_authority_handoff_require_transaction_journal_shadow".to_string(), json!(true));
        rust_core.insert("rust_config_state_authority_handoff_require_audit_shadow".to_string(), json!(true));
        rust_core.insert("rust_config_state_authority_handoff_require_rollback_shadow".to_string(), json!(true));
        rust_core.insert("rust_config_state_authority_handoff_require_no_side_effects".to_string(), json!(true));
        rust_core.insert("rust_config_state_authority_handoff_max_shadow_age_seconds".to_string(), json!(900));
        root.insert("rust_core".to_string(), Value::Object(rust_core));

        let mut run_cycle = Map::new();
        run_cycle.insert("status".to_string(), json!("rust_run_cycle_orchestrator_handoff_contract_ready"));
        run_cycle.insert("rust_run_cycle_orchestrator_handoff_ready".to_string(), json!(true));
        run_cycle.insert("rust_run_cycle_authoritative".to_string(), json!(false));
        run_cycle.insert("python_run_cycle_authoritative".to_string(), json!(true));
        run_cycle.insert("python_backend_required".to_string(), json!(true));
        root.insert("rust_run_cycle_orchestrator_handoff_contract".to_string(), Value::Object(run_cycle));

        Value::Object(root)
    }

    #[test]
    fn defaults_to_shadow_only_config_state_authority_handoff_contract() {
        let (result, errors, _warnings) = build_rust_config_state_authority_handoff_contract_payload(&json!({}));
        assert!(errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("rust_config_state_authority_handoff_contract_shadow_only"));
        assert_eq!(result.get("rust_config_state_authoritative").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn blocks_execute_and_config_state_switch_attempts() {
        let (result, errors, _warnings) = build_rust_config_state_authority_handoff_contract_payload(&json!({"execute": true}));
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
    }

    #[test]
    fn builds_ready_config_state_authority_handoff_contract_without_switching_python_authority() {
        let payload = ready_payload();
        let (result, errors, _warnings) = build_rust_config_state_authority_handoff_contract_payload(&payload);
        assert!(errors.is_empty(), "errors: {errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("rust_config_state_authority_handoff_contract_ready"));
        assert_eq!(result.get("rust_config_state_authority_handoff_ready").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("rust_config_state_authoritative").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("python_config_state_authoritative").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("python_backend_required").and_then(Value::as_bool), Some(true));
    }
}
