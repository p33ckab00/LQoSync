use crate::protocol::Diagnostic;
use crate::rust_backend_api_handoff::build_rust_backend_api_handoff_plan_payload;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const CONFIRM_SCHEDULER_HANDOFF_PLAN: &str = "CONFIRM_RUST_BACKEND_SCHEDULER_RUN_CYCLE_HANDOFF_PLAN";
const CONFIRM_API_HANDOFF_PLAN: &str = "CONFIRM_RUST_BACKEND_API_HANDOFF_PLAN";

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

fn scheduler_handoff_id(seed: &Value) -> String {
    let text = serde_json::to_string(seed).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("schedhandoff-{}", &digest[..16])
}

/// Build the Rust scheduler/run-cycle handoff plan while keeping Python authoritative.
///
/// v5.2 continues the full-Rust-backend track after the API handoff plan. It does
/// not start a Rust scheduler, does not replace Python run_cycle, and does not
/// remove Python. It only verifies the handoff prerequisites and returns an
/// auditable, non-mutating plan.
pub fn build_rust_backend_scheduler_handoff_plan_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(
            str_value(payload.get("mode"), "plan"),
            "execute" | "commit" | "switch" | "remove-python" | "replace-scheduler" | "replace-run-cycle" | "production" | "cutover"
        );
    if requested_execute {
        errors.push(Diagnostic::error(
            "rust_backend_scheduler_handoff_execute_not_implemented",
            Some("rust_backend_scheduler_handoff_plan".to_string()),
            "This release only builds a Rust scheduler/run_cycle handoff plan. It does not start a Rust scheduler, replace Python run_cycle, or remove Python.",
        ));
    }

    let allow_plan = bool_value(config_value(payload, "allow_rust_backend_scheduler_handoff_plan"), false);
    let plan_pilot = bool_value(config_value(payload, "rust_backend_scheduler_handoff_plan_pilot"), false);
    let handoff_mode = str_value(config_value(payload, "rust_backend_scheduler_handoff_mode"), "plan_only");
    let require_api_handoff = bool_value(config_value(payload, "rust_backend_scheduler_handoff_require_api_handoff"), true);
    let require_python_fallback = bool_value(config_value(payload, "rust_backend_scheduler_handoff_require_python_fallback"), true);
    let require_manual_confirmation = bool_value(config_value(payload, "rust_backend_scheduler_handoff_require_manual_confirmation"), true);
    let require_run_cycle_shadow = bool_value(config_value(payload, "rust_backend_scheduler_handoff_require_run_cycle_shadow"), true);
    let require_scheduler_parity = bool_value(config_value(payload, "rust_backend_scheduler_handoff_require_scheduler_parity"), true);
    let require_no_side_effects = bool_value(config_value(payload, "rust_backend_scheduler_handoff_require_no_side_effects"), true);
    let max_shadow_age = number_value(config_value(payload, "rust_backend_scheduler_handoff_max_shadow_age_seconds"), 900);
    let shadow_age = number_value(payload.get("shadow_age_seconds"), 0);

    let confirmation = str_value(payload.get("confirmation"), "");
    let confirmation_ok = !require_manual_confirmation || confirmation == CONFIRM_SCHEDULER_HANDOFF_PLAN;
    if require_manual_confirmation && !confirmation_ok {
        warnings.push(Diagnostic::warning(
            "rust_backend_scheduler_handoff_confirmation_required",
            Some("confirmation".to_string()),
            "Rust scheduler/run_cycle handoff planning requires CONFIRM_RUST_BACKEND_SCHEDULER_RUN_CYCLE_HANDOFF_PLAN before it can report ready.",
        ));
    }

    if !require_python_fallback {
        errors.push(Diagnostic::error(
            "rust_backend_scheduler_handoff_requires_python_fallback",
            Some("rust_core.rust_backend_scheduler_handoff_require_python_fallback".to_string()),
            "v5.2 still requires the Python scheduler/run_cycle backend as fallback. Python removal belongs to later authoritative runtime phases.",
        ));
    }

    if shadow_age > max_shadow_age {
        warnings.push(Diagnostic::warning(
            "rust_backend_scheduler_handoff_shadow_stale",
            Some("shadow_age_seconds".to_string()),
            "Rust-shadow data is older than the configured maximum age; scheduler/run_cycle handoff plan remains under review.",
        ).with_value(json!({"shadow_age_seconds": shadow_age, "max_shadow_age_seconds": max_shadow_age})));
    }

    let api_handoff_value = first_object(payload, &[
        "rust_backend_api_handoff_plan",
        "api_handoff_plan",
        "rust_backend_api_handoff",
    ]).cloned();

    let (api_handoff, api_errors, mut api_warnings) = match api_handoff_value {
        Some(v) => (v, Vec::new(), Vec::new()),
        None => {
            let mut nested_payload = payload.clone();
            if let Some(obj) = nested_payload.as_object_mut() {
                let api_confirmation = str_value(
                    payload.get("rust_backend_api_handoff_confirmation"),
                    CONFIRM_API_HANDOFF_PLAN,
                );
                obj.insert("confirmation".to_string(), json!(api_confirmation));
            }
            build_rust_backend_api_handoff_plan_payload(&nested_payload)
        }
    };
    warnings.append(&mut api_warnings);

    if !api_errors.is_empty() {
        warnings.push(Diagnostic::warning(
            "rust_backend_scheduler_handoff_api_handoff_not_clean",
            Some("rust_backend_api_handoff_plan".to_string()),
            "Rust API handoff plan returned errors; scheduler/run_cycle handoff remains shadow-only.",
        ));
    }

    let api_status = api_handoff.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let api_ready = api_errors.is_empty()
        && api_status == "rust_backend_api_handoff_plan_ready"
        && api_handoff.get("rust_backend_api_handoff_ready").and_then(Value::as_bool).unwrap_or(false)
        && api_handoff.get("webui_ux_unchanged").and_then(Value::as_bool).unwrap_or(false)
        && api_handoff.get("python_backend_required").and_then(Value::as_bool).unwrap_or(true)
        && api_handoff.get("python_backend_removed").and_then(Value::as_bool).unwrap_or(false) == false;

    if require_api_handoff && !api_ready {
        warnings.push(Diagnostic::warning(
            "rust_backend_scheduler_handoff_api_handoff_not_ready",
            Some("rust_backend_api_handoff_plan".to_string()),
            "Rust API handoff plan has not passed; scheduler/run_cycle handoff remains shadow-only or under review.",
        ));
    }

    let scheduler_interval_seconds = number_value(payload.get("scheduler_interval_seconds"), 30);
    let scheduler_manifest_ready = bool_value(payload.get("scheduler_manifest_ready"), false) && scheduler_interval_seconds > 0;
    if require_scheduler_parity && !scheduler_manifest_ready {
        warnings.push(Diagnostic::warning(
            "rust_backend_scheduler_handoff_scheduler_manifest_required",
            Some("scheduler_manifest_ready".to_string()),
            "Rust scheduler/run_cycle handoff requires a scheduler manifest with a valid interval before it can report ready.",
        ));
    }

    let run_cycle_shadow_ready = bool_value(payload.get("run_cycle_shadow_ready"), false);
    let run_cycle_shadow_count = number_value(payload.get("run_cycle_shadow_count"), 0);
    let run_cycle_ready = !require_run_cycle_shadow || (run_cycle_shadow_ready && run_cycle_shadow_count > 0);
    if require_run_cycle_shadow && !run_cycle_ready {
        warnings.push(Diagnostic::warning(
            "rust_backend_scheduler_handoff_run_cycle_shadow_required",
            Some("run_cycle_shadow_ready".to_string()),
            "Rust scheduler/run_cycle handoff requires successful run_cycle shadow cycles before it can report ready.",
        ));
    }

    let cleanup_attempted = bool_value(payload.get("cleanup_attempted"), false);
    let apply_attempted = bool_value(payload.get("apply_attempted"), false);
    let write_attempted = bool_value(payload.get("write_attempted"), false);
    let python_removed = bool_value(payload.get("python_backend_removed"), false);
    let scheduler_switched = bool_value(payload.get("scheduler_switched_to_rust"), false);
    let run_cycle_switched = bool_value(payload.get("run_cycle_switched_to_rust"), false);
    let side_effect_free = !cleanup_attempted && !apply_attempted && !write_attempted && !python_removed && !scheduler_switched && !run_cycle_switched;

    if require_no_side_effects && !side_effect_free {
        errors.push(Diagnostic::error(
            "rust_backend_scheduler_handoff_side_effect_detected",
            Some("rust_backend_scheduler_handoff_plan".to_string()),
            "Scheduler/run_cycle handoff plan detected cleanup/apply/write/Python-removal/scheduler-switch side effects, which are forbidden in this release.",
        ));
    }

    let gates_ready = allow_plan && plan_pilot && handoff_mode == "plan_only";
    if !gates_ready {
        warnings.push(Diagnostic::warning(
            "rust_backend_scheduler_handoff_gates_not_enabled",
            Some("rust_core".to_string()),
            "Rust scheduler/run_cycle handoff plan gates are not fully enabled; report remains shadow-only.",
        ));
    }

    let ready = errors.is_empty()
        && gates_ready
        && confirmation_ok
        && (!require_api_handoff || api_ready)
        && shadow_age <= max_shadow_age
        && require_python_fallback
        && scheduler_manifest_ready
        && run_cycle_ready
        && side_effect_free;

    let review = errors.is_empty() && api_ready && scheduler_manifest_ready && run_cycle_ready && side_effect_free;
    let status = if !errors.is_empty() {
        "blocked"
    } else if ready {
        "rust_backend_scheduler_handoff_plan_ready"
    } else if review {
        "rust_backend_scheduler_handoff_plan_review"
    } else {
        "rust_backend_scheduler_handoff_plan_shadow_only"
    };

    let mut seed = Map::new();
    seed.insert("status".to_string(), json!(status));
    seed.insert("api_status".to_string(), json!(api_status));
    seed.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    seed.insert("scheduler_interval_seconds".to_string(), json!(scheduler_interval_seconds));

    let mut plan_steps = Vec::new();
    for (idx, name) in [
        "mirror_python_scheduler_config",
        "shadow_run_cycle_invocations",
        "compare_run_cycle_outputs",
        "keep_python_scheduler_authoritative",
        "defer_scheduler_switch_to_future_release",
    ].iter().enumerate() {
        let mut step = Map::new();
        step.insert("step".to_string(), json!(idx + 1));
        step.insert("name".to_string(), json!(name));
        step.insert("mutating".to_string(), json!(false));
        plan_steps.push(Value::Object(step));
    }

    let mut map = Map::new();
    map.insert("mode".to_string(), json!("rust_backend_scheduler_handoff_plan"));
    map.insert("status".to_string(), json!(status));
    map.insert("scheduler_handoff_plan_id".to_string(), json!(scheduler_handoff_id(&Value::Object(seed))));
    map.insert("rust_backend_scheduler_handoff_ready".to_string(), json!(ready));
    map.insert("api_handoff_status".to_string(), json!(api_status));
    map.insert("api_handoff_ready".to_string(), json!(api_ready));
    map.insert("scheduler_manifest_ready".to_string(), json!(scheduler_manifest_ready));
    map.insert("scheduler_interval_seconds".to_string(), json!(scheduler_interval_seconds));
    map.insert("run_cycle_shadow_ready".to_string(), json!(run_cycle_ready));
    map.insert("run_cycle_shadow_count".to_string(), json!(run_cycle_shadow_count));
    map.insert("scheduler_handoff_plan_steps".to_string(), Value::Array(plan_steps));
    map.insert("webui_ux_unchanged".to_string(), json!(api_handoff.get("webui_ux_unchanged").and_then(Value::as_bool).unwrap_or(false)));
    map.insert("full_rust_backend".to_string(), json!(false));
    map.insert("python_backend_removable".to_string(), json!(false));
    map.insert("python_backend_removed".to_string(), json!(false));
    map.insert("python_backend_required".to_string(), json!(true));
    map.insert("python_backend_fallback_required".to_string(), json!(true));
    map.insert("rust_api_service_authoritative".to_string(), json!(false));
    map.insert("rust_scheduler_authoritative".to_string(), json!(false));
    map.insert("rust_run_cycle_authoritative".to_string(), json!(false));
    map.insert("rust_apply_authoritative".to_string(), json!(false));
    map.insert("scheduler_switched_to_rust".to_string(), json!(false));
    map.insert("run_cycle_switched_to_rust".to_string(), json!(false));
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
    map.insert("next_stage".to_string(), json!("rust_run_cycle_orchestrator_handoff_contract"));
    map.insert("note".to_string(), json!("v5.2 plans the Rust scheduler/run_cycle handoff while keeping Python scheduler/run_cycle authoritative and preserving the existing WebUI/UX."));

    (Value::Object(map), errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_payload() -> Value {
        let mut root = Map::new();
        root.insert("confirmation".to_string(), json!(CONFIRM_SCHEDULER_HANDOFF_PLAN));
        root.insert("shadow_age_seconds".to_string(), json!(10));
        root.insert("scheduler_manifest_ready".to_string(), json!(true));
        root.insert("scheduler_interval_seconds".to_string(), json!(30));
        root.insert("run_cycle_shadow_ready".to_string(), json!(true));
        root.insert("run_cycle_shadow_count".to_string(), json!(3));

        let mut rust_core = Map::new();
        rust_core.insert("allow_rust_backend_scheduler_handoff_plan".to_string(), json!(true));
        rust_core.insert("rust_backend_scheduler_handoff_plan_pilot".to_string(), json!(true));
        rust_core.insert("rust_backend_scheduler_handoff_mode".to_string(), json!("plan_only"));
        rust_core.insert("rust_backend_scheduler_handoff_require_api_handoff".to_string(), json!(true));
        rust_core.insert("rust_backend_scheduler_handoff_require_python_fallback".to_string(), json!(true));
        rust_core.insert("rust_backend_scheduler_handoff_require_manual_confirmation".to_string(), json!(true));
        rust_core.insert("rust_backend_scheduler_handoff_require_run_cycle_shadow".to_string(), json!(true));
        rust_core.insert("rust_backend_scheduler_handoff_require_scheduler_parity".to_string(), json!(true));
        rust_core.insert("rust_backend_scheduler_handoff_require_no_side_effects".to_string(), json!(true));
        rust_core.insert("rust_backend_scheduler_handoff_max_shadow_age_seconds".to_string(), json!(900));
        root.insert("rust_core".to_string(), Value::Object(rust_core));

        let mut api_handoff = Map::new();
        api_handoff.insert("status".to_string(), json!("rust_backend_api_handoff_plan_ready"));
        api_handoff.insert("rust_backend_api_handoff_ready".to_string(), json!(true));
        api_handoff.insert("webui_ux_unchanged".to_string(), json!(true));
        api_handoff.insert("python_backend_required".to_string(), json!(true));
        api_handoff.insert("python_backend_removed".to_string(), json!(false));
        root.insert("rust_backend_api_handoff_plan".to_string(), Value::Object(api_handoff));

        Value::Object(root)
    }

    #[test]
    fn defaults_to_shadow_only_scheduler_handoff_plan() {
        let (result, errors, _warnings) = build_rust_backend_scheduler_handoff_plan_payload(&json!({}));
        assert!(errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("rust_backend_scheduler_handoff_plan_shadow_only"));
        assert_eq!(result.get("rust_scheduler_authoritative").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn blocks_execute_and_scheduler_switch_attempts() {
        let (result, errors, _warnings) = build_rust_backend_scheduler_handoff_plan_payload(&json!({"execute": true}));
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
    }

    #[test]
    fn builds_ready_scheduler_handoff_plan_without_switching_python_authority() {
        let payload = ready_payload();
        let (result, errors, _warnings) = build_rust_backend_scheduler_handoff_plan_payload(&payload);
        assert!(errors.is_empty(), "errors: {errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("rust_backend_scheduler_handoff_plan_ready"));
        assert_eq!(result.get("rust_backend_scheduler_handoff_ready").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("rust_scheduler_authoritative").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("rust_run_cycle_authoritative").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("python_backend_required").and_then(Value::as_bool), Some(true));
    }
}
