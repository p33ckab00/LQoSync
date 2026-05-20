use crate::protocol::Diagnostic;
use crate::rust_full_backend_post_retirement_verifier::build_full_rust_backend_post_retirement_verifier_payload;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const CONFIRM_STEADY_STATE_GUARD: &str = "CONFIRM_FULL_RUST_BACKEND_STEADY_STATE_GUARD";
const CONFIRM_POST_RETIREMENT_VERIFIER: &str = "CONFIRM_FULL_RUST_BACKEND_POST_RETIREMENT_VERIFIER";

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

fn steady_state_guard_id(seed: &Value) -> String {
    let text = serde_json::to_string(seed).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("fullrust-steady-{}", &digest[..16])
}

/// Build the v7.3 steady-state guard for a fully migrated Rust backend.
///
/// This is a post-retirement production guard. It verifies that the Rust backend
/// remains authoritative after Python retirement, that Python has not drifted
/// back into service, that WebUI/static assets remain unchanged, and that rollback
/// remains available. It does not mutate services, delete files, write config, or
/// apply LibreQoS changes.
pub fn build_full_rust_backend_steady_state_guard_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(str_value(payload.get("mode"), "guard"), "execute" | "repair" | "delete" | "disable" | "restart" | "switch");
    if requested_execute {
        errors.push(Diagnostic::error(
            "full_rust_backend_steady_state_guard_execute_not_implemented",
            Some("full_rust_backend_steady_state_guard".to_string()),
            "The steady-state guard is verification-only. It does not mutate services, delete Python files, or switch traffic.",
        ));
    }

    let allow_guard = bool_value(config_value(payload, "allow_full_rust_backend_steady_state_guard"), false);
    let guard_pilot = bool_value(config_value(payload, "full_rust_backend_steady_state_guard_pilot"), false);
    let guard_mode = str_value(config_value(payload, "full_rust_backend_steady_state_guard_mode"), "guard_only");
    let require_post_retirement = bool_value(config_value(payload, "full_rust_backend_steady_state_require_post_retirement_verifier"), true);
    let require_runtime_health = bool_value(config_value(payload, "full_rust_backend_steady_state_require_runtime_health"), true);
    let require_no_python_drift = bool_value(config_value(payload, "full_rust_backend_steady_state_require_no_python_drift"), true);
    let require_webui_unchanged = bool_value(config_value(payload, "full_rust_backend_steady_state_require_webui_unchanged"), true);
    let require_rollback_package = bool_value(config_value(payload, "full_rust_backend_steady_state_require_rollback_package"), true);
    let require_server_tests = bool_value(config_value(payload, "full_rust_backend_steady_state_require_server_tests"), true);
    let require_manual_confirmation = bool_value(config_value(payload, "full_rust_backend_steady_state_require_manual_confirmation"), true);
    let require_operator_ack = bool_value(config_value(payload, "full_rust_backend_steady_state_require_operator_ack"), true);
    let max_shadow_age = number_value(config_value(payload, "full_rust_backend_steady_state_max_shadow_age_seconds"), 900);
    let shadow_age = number_value(payload.get("shadow_age_seconds"), 0);

    let confirmation = str_value(payload.get("confirmation"), "");
    let confirmation_ok = !require_manual_confirmation || confirmation == CONFIRM_STEADY_STATE_GUARD;
    if require_manual_confirmation && !confirmation_ok {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_steady_state_confirmation_required",
            Some("confirmation".to_string()),
            "Steady-state guard requires CONFIRM_FULL_RUST_BACKEND_STEADY_STATE_GUARD before it can report verified.",
        ));
    }

    if shadow_age > max_shadow_age {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_steady_state_guard_stale",
            Some("shadow_age_seconds".to_string()),
            "Production steady-state observations are older than the configured maximum age.",
        ).with_value(json!({"shadow_age_seconds": shadow_age, "max_shadow_age_seconds": max_shadow_age})));
    }

    let post_retirement_value = first_object(payload, &[
        "full_rust_backend_post_retirement_verifier",
        "full_rust_backend_post_retirement_verifier_contract",
        "rust_full_backend_post_retirement_verifier",
    ]).cloned();

    let (post_retirement, post_retirement_errors, mut post_retirement_warnings) = match post_retirement_value {
        Some(v) => (v, Vec::new(), Vec::new()),
        None => {
            let mut nested_payload = payload.clone();
            if let Some(obj) = nested_payload.as_object_mut() {
                let nested_confirmation = str_value(
                    payload.get("full_rust_backend_post_retirement_verifier_confirmation"),
                    CONFIRM_POST_RETIREMENT_VERIFIER,
                );
                obj.insert("confirmation".to_string(), json!(nested_confirmation));
            }
            build_full_rust_backend_post_retirement_verifier_payload(&nested_payload)
        }
    };
    warnings.append(&mut post_retirement_warnings);

    let post_retirement_status = post_retirement.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let post_retirement_ready = post_retirement_errors.is_empty()
        && post_retirement_status == "full_rust_backend_post_retirement_verified"
        && post_retirement.get("full_rust_backend").and_then(Value::as_bool).unwrap_or(false)
        && post_retirement.get("python_backend_removed").and_then(Value::as_bool).unwrap_or(false)
        && post_retirement.get("webui_ux_unchanged").and_then(Value::as_bool).unwrap_or(true);
    if require_post_retirement && !post_retirement_ready {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_steady_state_post_retirement_not_ready",
            Some("full_rust_backend_post_retirement_verifier".to_string()),
            "Post-retirement verifier has not passed; steady-state production verification remains blocked or review-only.",
        ));
    }

    let rust_runtime_ready = bool_value(payload.get("rust_service_active"), false)
        && bool_value(payload.get("rust_api_healthcheck_passed"), false)
        && (bool_value(payload.get("rust_unix_socket_active"), false) || bool_value(payload.get("rust_http_api_active"), false))
        && bool_value(payload.get("rust_service_runtime_authoritative"), false)
        && bool_value(payload.get("api_traffic_switched_to_rust"), false);
    if require_runtime_health && !rust_runtime_ready {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_steady_state_runtime_health_required",
            Some("rust_service_active".to_string()),
            "Rust service, healthcheck, socket/API, API traffic switch, and runtime authority must remain healthy.",
        ));
    }

    let python_drift_absent = bool_value(payload.get("flask_routes_disabled"), false)
        && bool_value(payload.get("python_backend_stopped_or_disabled"), false)
        && (bool_value(payload.get("python_backend_service_masked_or_disabled"), false) || bool_value(payload.get("python_backend_service_removed"), false))
        && !bool_value(payload.get("python_backend_unexpectedly_running"), false)
        && !bool_value(payload.get("flask_routes_reappeared"), false)
        && !bool_value(payload.get("api_traffic_routed_to_python"), false);
    if require_no_python_drift && !python_drift_absent {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_steady_state_python_drift_detected",
            Some("python_backend_unexpectedly_running".to_string()),
            "Python/Flask backend must remain retired and API traffic must not drift back to Python.",
        ));
    }

    let webui_unchanged = bool_value(payload.get("webui_ux_unchanged"), false)
        && bool_value(payload.get("webui_static_asset_paths_unchanged"), false)
        && bool_value(payload.get("webui_static_assets_preserved"), false);
    if require_webui_unchanged && !webui_unchanged {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_steady_state_webui_unchanged_required",
            Some("webui_ux_unchanged".to_string()),
            "WebUI/UX and static assets must remain unchanged after Rust backend production migration.",
        ));
    }

    let rollback_ready = bool_value(payload.get("python_backend_rollback_package_ready"), false)
        && bool_value(payload.get("rollback_test_passed"), false)
        && !str_value(payload.get("rollback_path"), "").trim().is_empty();
    if require_rollback_package && !rollback_ready {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_steady_state_rollback_required",
            Some("python_backend_rollback_package_ready".to_string()),
            "Rollback package, rollback path, and rollback test must remain available during steady-state operations.",
        ));
    }

    let tests_passed = bool_value(payload.get("server_cargo_tests_passed"), false)
        && bool_value(payload.get("self_test_passed"), false)
        && bool_value(payload.get("production_healthcheck_passed"), false)
        && bool_value(payload.get("post_retirement_healthcheck_passed"), false)
        && bool_value(payload.get("steady_state_healthcheck_passed"), false);
    if require_server_tests && !tests_passed {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_steady_state_tests_required",
            Some("server_cargo_tests_passed".to_string()),
            "Server cargo, self-test, production, post-retirement, and steady-state healthchecks must pass.",
        ));
    }

    let operator_ack = bool_value(payload.get("operator_full_rust_backend_steady_state_ack"), false) || bool_value(payload.get("operator_acknowledged"), false);
    if require_operator_ack && !operator_ack {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_steady_state_operator_ack_required",
            Some("operator_full_rust_backend_steady_state_ack".to_string()),
            "Operator acknowledgement is required for steady-state production verification.",
        ));
    }

    let gates_ready = allow_guard && guard_pilot && guard_mode == "guard_only";
    if !gates_ready {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_steady_state_gates_not_enabled",
            Some("rust_core".to_string()),
            "Steady-state guard gates are not fully enabled.",
        ));
    }

    let ready = errors.is_empty()
        && gates_ready
        && confirmation_ok
        && (!require_post_retirement || post_retirement_ready)
        && (!require_runtime_health || rust_runtime_ready)
        && (!require_no_python_drift || python_drift_absent)
        && (!require_webui_unchanged || webui_unchanged)
        && (!require_rollback_package || rollback_ready)
        && (!require_server_tests || tests_passed)
        && (!require_operator_ack || operator_ack)
        && shadow_age <= max_shadow_age;

    let status = if !errors.is_empty() {
        "blocked"
    } else if ready {
        "full_rust_backend_steady_state_verified"
    } else if post_retirement_ready && rust_runtime_ready {
        "full_rust_backend_steady_state_review"
    } else {
        "full_rust_backend_steady_state_blocked"
    };

    let mut seed = Map::new();
    seed.insert("status".to_string(), json!(status));
    seed.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    seed.insert("post_retirement_status".to_string(), json!(post_retirement_status));

    let mut map = Map::new();
    map.insert("mode".to_string(), json!("full_rust_backend_steady_state_guard"));
    map.insert("status".to_string(), json!(status));
    map.insert("steady_state_guard_id".to_string(), json!(steady_state_guard_id(&Value::Object(seed))));
    map.insert("full_rust_backend".to_string(), json!(ready));
    map.insert("full_rust_backend_production_enabled".to_string(), json!(ready));
    map.insert("rust_service_runtime_authoritative".to_string(), json!(ready && rust_runtime_ready));
    map.insert("api_traffic_switched_to_rust".to_string(), json!(bool_value(payload.get("api_traffic_switched_to_rust"), false)));
    map.insert("python_backend_removed".to_string(), json!(ready && python_drift_absent));
    map.insert("python_backend_retired".to_string(), json!(ready && python_drift_absent));
    map.insert("python_drift_absent".to_string(), json!(python_drift_absent));
    map.insert("webui_ux_unchanged".to_string(), json!(webui_unchanged));
    map.insert("rollback_ready".to_string(), json!(rollback_ready));
    map.insert("server_tests_passed".to_string(), json!(tests_passed));
    map.insert("runtime_health_ready".to_string(), json!(rust_runtime_ready));
    map.insert("steady_state_healthcheck_passed".to_string(), json!(bool_value(payload.get("steady_state_healthcheck_passed"), false)));
    map.insert("post_retirement_status".to_string(), json!(post_retirement_status));
    map.insert("post_retirement_ready".to_string(), json!(post_retirement_ready));
    map.insert("gates_ready".to_string(), json!(gates_ready));
    map.insert("operator_acknowledged".to_string(), json!(operator_ack));
    map.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    map.insert("max_shadow_age_seconds".to_string(), json!(max_shadow_age));
    map.insert("note".to_string(), json!("v7.3 verifies steady-state full Rust backend production after Python retirement. It is verification-only and preserves WebUI/UX/static assets and rollback safety."));

    (Value::Object(map), errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_payload() -> Value {
        let mut rc = Map::new();
        rc.insert("full_rust_backend_steady_state_guard_pilot".to_string(), json!(true));
        rc.insert("allow_full_rust_backend_steady_state_guard".to_string(), json!(true));
        rc.insert("full_rust_backend_steady_state_guard_mode".to_string(), json!("guard_only"));
        rc.insert("full_rust_backend_steady_state_require_post_retirement_verifier".to_string(), json!(true));
        rc.insert("full_rust_backend_steady_state_require_runtime_health".to_string(), json!(true));
        rc.insert("full_rust_backend_steady_state_require_no_python_drift".to_string(), json!(true));
        rc.insert("full_rust_backend_steady_state_require_webui_unchanged".to_string(), json!(true));
        rc.insert("full_rust_backend_steady_state_require_rollback_package".to_string(), json!(true));
        rc.insert("full_rust_backend_steady_state_require_server_tests".to_string(), json!(true));
        rc.insert("full_rust_backend_steady_state_require_manual_confirmation".to_string(), json!(true));
        rc.insert("full_rust_backend_steady_state_require_operator_ack".to_string(), json!(true));
        rc.insert("full_rust_backend_steady_state_max_shadow_age_seconds".to_string(), json!(900));

        let mut post = Map::new();
        post.insert("status".to_string(), json!("full_rust_backend_post_retirement_verified"));
        post.insert("full_rust_backend".to_string(), json!(true));
        post.insert("python_backend_removed".to_string(), json!(true));
        post.insert("webui_ux_unchanged".to_string(), json!(true));

        let mut payload = Map::new();
        payload.insert("rust_core".to_string(), Value::Object(rc));
        payload.insert("full_rust_backend_post_retirement_verifier".to_string(), Value::Object(post));
        payload.insert("confirmation".to_string(), json!(CONFIRM_STEADY_STATE_GUARD));
        payload.insert("shadow_age_seconds".to_string(), json!(30));
        payload.insert("rust_service_active".to_string(), json!(true));
        payload.insert("rust_api_healthcheck_passed".to_string(), json!(true));
        payload.insert("rust_unix_socket_active".to_string(), json!(true));
        payload.insert("api_traffic_switched_to_rust".to_string(), json!(true));
        payload.insert("rust_service_runtime_authoritative".to_string(), json!(true));
        payload.insert("flask_routes_disabled".to_string(), json!(true));
        payload.insert("python_backend_stopped_or_disabled".to_string(), json!(true));
        payload.insert("python_backend_service_masked_or_disabled".to_string(), json!(true));
        payload.insert("python_api_routes_unregistered".to_string(), json!(true));
        payload.insert("webui_ux_unchanged".to_string(), json!(true));
        payload.insert("webui_static_asset_paths_unchanged".to_string(), json!(true));
        payload.insert("webui_static_assets_preserved".to_string(), json!(true));
        payload.insert("python_backend_rollback_package_ready".to_string(), json!(true));
        payload.insert("rollback_path".to_string(), json!("restore_python_backend_and_flask_routes"));
        payload.insert("rollback_test_passed".to_string(), json!(true));
        payload.insert("server_cargo_tests_passed".to_string(), json!(true));
        payload.insert("self_test_passed".to_string(), json!(true));
        payload.insert("production_healthcheck_passed".to_string(), json!(true));
        payload.insert("post_retirement_healthcheck_passed".to_string(), json!(true));
        payload.insert("steady_state_healthcheck_passed".to_string(), json!(true));
        payload.insert("operator_full_rust_backend_steady_state_ack".to_string(), json!(true));
        Value::Object(payload)
    }

    #[test]
    fn defaults_to_blocked_when_gates_missing() {
        let (result, errors, _warnings) = build_full_rust_backend_steady_state_guard_payload(&json!({}));
        assert!(errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("full_rust_backend_steady_state_blocked"));
        assert_eq!(result.get("full_rust_backend").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn blocks_execute_attempts() {
        let (result, errors, _warnings) = build_full_rust_backend_steady_state_guard_payload(&json!({"execute": true}));
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
    }

    #[test]
    fn verifies_steady_state_full_rust_backend_operations() {
        let payload = ready_payload();
        let (result, errors, _warnings) = build_full_rust_backend_steady_state_guard_payload(&payload);
        assert!(errors.is_empty(), "errors: {errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("full_rust_backend_steady_state_verified"));
        assert_eq!(result.get("full_rust_backend").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("python_backend_removed").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("python_drift_absent").and_then(Value::as_bool), Some(true));
    }
}
