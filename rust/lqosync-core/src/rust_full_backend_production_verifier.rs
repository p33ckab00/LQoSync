use crate::protocol::Diagnostic;
use crate::rust_full_backend_production_cutover::build_full_rust_backend_production_cutover_payload;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER: &str = "CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER";
const CONFIRM_FULL_RUST_BACKEND_PRODUCTION_CUTOVER: &str = "CONFIRM_FULL_RUST_BACKEND_PRODUCTION_CUTOVER";

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

fn verifier_id(seed: &Value) -> String {
    let text = serde_json::to_string(seed).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("fullrust-prodverify-{}", &digest[..16])
}

/// Build the v7.1 full Rust backend production verifier and Python retirement guard.
///
/// This is a post-cutover verification/guard operation. It can report that the
/// Rust backend is production-authoritative when the operator supplies runtime
/// health signals, but it still does not stop services or delete Python files
/// inside the Rust core. OS-level retirement remains delegated to the guarded
/// shell script with explicit confirmation and rollback prerequisites.
pub fn build_full_rust_backend_production_verifier_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(str_value(payload.get("mode"), "verify"), "execute" | "remove" | "delete" | "disable-python" | "retire-python" | "switch");
    if requested_execute {
        errors.push(Diagnostic::error(
            "full_rust_backend_production_verifier_execute_not_implemented",
            Some("full_rust_backend_production_verifier".to_string()),
            "The Rust core verifier is non-mutating. Use scripts/python-backend-retirement-executor-guard.sh with explicit confirmation for OS-level Python service retirement.",
        ));
    }

    let allow_verify = bool_value(config_value(payload, "allow_full_rust_backend_production_verifier"), false);
    let verifier_pilot = bool_value(config_value(payload, "full_rust_backend_production_verifier_pilot"), false);
    let verifier_mode = str_value(config_value(payload, "full_rust_backend_production_verifier_mode"), "verify_only");
    let require_cutover = bool_value(config_value(payload, "full_rust_backend_production_verifier_require_cutover"), true);
    let require_webui_unchanged = bool_value(config_value(payload, "full_rust_backend_production_verifier_require_webui_unchanged"), true);
    let require_runtime_health = bool_value(config_value(payload, "full_rust_backend_production_verifier_require_runtime_health"), true);
    let require_rollback_package = bool_value(config_value(payload, "full_rust_backend_production_verifier_require_rollback_package"), true);
    let require_manual_confirmation = bool_value(config_value(payload, "full_rust_backend_production_verifier_require_manual_confirmation"), true);
    let require_operator_ack = bool_value(config_value(payload, "full_rust_backend_production_verifier_require_operator_ack"), true);
    let require_server_tests = bool_value(config_value(payload, "full_rust_backend_production_verifier_require_server_tests"), true);
    let max_shadow_age = number_value(config_value(payload, "full_rust_backend_production_verifier_max_shadow_age_seconds"), 900);
    let shadow_age = number_value(payload.get("shadow_age_seconds"), 0);

    let confirmation = str_value(payload.get("confirmation"), "");
    let confirmation_ok = !require_manual_confirmation || confirmation == CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER;
    if require_manual_confirmation && !confirmation_ok {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_production_verifier_confirmation_required",
            Some("confirmation".to_string()),
            "Full Rust backend production verifier requires CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER before it can report production verified.",
        ));
    }

    if shadow_age > max_shadow_age {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_production_verifier_shadow_stale",
            Some("shadow_age_seconds".to_string()),
            "Rust-shadow data is older than the configured maximum age; production verification remains under review.",
        ).with_value(json!({"shadow_age_seconds": shadow_age, "max_shadow_age_seconds": max_shadow_age})));
    }

    let cutover_value = first_object(payload, &[
        "full_rust_backend_production_cutover",
        "full_rust_backend_production_cutover_contract",
        "rust_full_backend_production_cutover",
    ]).cloned();

    let (cutover, cutover_errors, mut cutover_warnings) = match cutover_value {
        Some(v) => (v, Vec::new(), Vec::new()),
        None => {
            let mut nested_payload = payload.clone();
            if let Some(obj) = nested_payload.as_object_mut() {
                let nested_confirmation = str_value(
                    payload.get("full_rust_backend_production_cutover_confirmation"),
                    CONFIRM_FULL_RUST_BACKEND_PRODUCTION_CUTOVER,
                );
                obj.insert("confirmation".to_string(), json!(nested_confirmation));
            }
            build_full_rust_backend_production_cutover_payload(&nested_payload)
        }
    };
    warnings.append(&mut cutover_warnings);

    let cutover_status = cutover.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let cutover_ready = cutover_errors.is_empty()
        && cutover_status == "full_rust_backend_production_cutover_ready"
        && cutover.get("cutover_allowed").and_then(Value::as_bool).unwrap_or(false)
        && cutover.get("webui_ux_unchanged").and_then(Value::as_bool).unwrap_or(true)
        && cutover.get("python_removal_allowed").and_then(Value::as_bool).unwrap_or(false)
        && cutover.get("python_backend_removable").and_then(Value::as_bool).unwrap_or(false);

    if require_cutover && !cutover_ready {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_production_verifier_cutover_not_ready",
            Some("full_rust_backend_production_cutover".to_string()),
            "Full Rust backend production cutover has not passed; production verification remains blocked or review-only.",
        ));
    }

    let webui_unchanged = bool_value(payload.get("webui_ux_unchanged"), true)
        && bool_value(payload.get("webui_static_asset_paths_unchanged"), true)
        && bool_value(payload.get("webui_static_assets_preserved"), true);
    if require_webui_unchanged && !webui_unchanged {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_production_verifier_webui_changed",
            Some("webui_ux_unchanged".to_string()),
            "WebUI/UX and static asset paths must remain unchanged after full Rust backend production cutover.",
        ));
    }

    let rust_service_active = bool_value(payload.get("rust_service_active"), false);
    let rust_api_healthcheck_passed = bool_value(payload.get("rust_api_healthcheck_passed"), false);
    let rust_unix_socket_active = bool_value(payload.get("rust_unix_socket_active"), false) || bool_value(payload.get("rust_http_api_active"), false);
    let api_traffic_switched_to_rust = bool_value(payload.get("api_traffic_switched_to_rust"), false);
    let flask_routes_disabled = bool_value(payload.get("flask_routes_disabled"), false);
    let python_backend_stopped_or_disabled = bool_value(payload.get("python_backend_stopped_or_disabled"), false);
    let runtime_health_ready = rust_service_active && rust_api_healthcheck_passed && rust_unix_socket_active && api_traffic_switched_to_rust;
    if require_runtime_health && !runtime_health_ready {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_production_verifier_runtime_health_required",
            Some("rust_service_active".to_string()),
            "Rust service, API healthcheck, runtime socket/API, and API traffic switch must be verified before production can be marked active.",
        ));
    }

    let rollback_path = str_value(payload.get("rollback_path"), "restore_python_backend_and_flask_routes");
    let rollback_package_ready = bool_value(payload.get("python_backend_rollback_package_ready"), false)
        || bool_value(payload.get("python_fallback_backup_ready"), false);
    let rollback_ready = !require_rollback_package || (!rollback_path.trim().is_empty() && rollback_package_ready);
    if require_rollback_package && !rollback_ready {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_production_verifier_rollback_package_required",
            Some("python_backend_rollback_package_ready".to_string()),
            "Python backend rollback package and rollback path must be ready before Python retirement can be allowed.",
        ));
    }

    let operator_ack = bool_value(payload.get("operator_full_rust_backend_production_verifier_ack"), false)
        || bool_value(payload.get("operator_acknowledged"), false);
    if require_operator_ack && !operator_ack {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_production_verifier_operator_ack_required",
            Some("operator_full_rust_backend_production_verifier_ack".to_string()),
            "Operator acknowledgment is required before full Rust backend production verification can pass.",
        ));
    }

    let server_tests_passed = bool_value(payload.get("server_cargo_tests_passed"), false)
        && bool_value(payload.get("self_test_passed"), false)
        && bool_value(payload.get("rollback_test_passed"), false)
        && bool_value(payload.get("production_healthcheck_passed"), false);
    if require_server_tests && !server_tests_passed {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_production_verifier_server_tests_required",
            Some("server_cargo_tests_passed".to_string()),
            "Server cargo tests, self-test, rollback test, and production healthcheck must pass before production verification can pass.",
        ));
    }

    let gates_ready = allow_verify && verifier_pilot && verifier_mode == "verify_only";
    if !gates_ready {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_production_verifier_gates_not_enabled",
            Some("rust_core".to_string()),
            "Full Rust backend production verifier gates are not fully enabled; report remains blocked or review-only.",
        ));
    }

    let production_verified = errors.is_empty()
        && gates_ready
        && confirmation_ok
        && (!require_cutover || cutover_ready)
        && webui_unchanged
        && runtime_health_ready
        && rollback_ready
        && operator_ack
        && server_tests_passed
        && shadow_age <= max_shadow_age;

    let retirement_executor_allowed = production_verified
        && python_backend_stopped_or_disabled
        && flask_routes_disabled
        && rollback_ready;

    let status = if !errors.is_empty() {
        "blocked"
    } else if production_verified {
        "full_rust_backend_production_verified"
    } else if cutover_ready && webui_unchanged && rollback_ready {
        "full_rust_backend_production_verifier_review"
    } else {
        "full_rust_backend_production_verifier_blocked"
    };

    let mut seed = Map::new();
    seed.insert("status".to_string(), json!(status));
    seed.insert("cutover_status".to_string(), json!(cutover_status));
    seed.insert("production_verified".to_string(), json!(production_verified));
    seed.insert("retirement_executor_allowed".to_string(), json!(retirement_executor_allowed));

    let mut guard_steps = Vec::new();
    for (idx, name) in [
        "verify_rust_backend_service_health",
        "verify_api_traffic_served_by_rust",
        "verify_webui_static_assets_unchanged",
        "verify_python_rollback_package",
        "guard_python_backend_retirement_executor",
    ].iter().enumerate() {
        let mut step = Map::new();
        step.insert("step".to_string(), json!(idx + 1));
        step.insert("name".to_string(), json!(name));
        step.insert("requires_operator_supervision".to_string(), json!(true));
        guard_steps.push(Value::Object(step));
    }

    let mut map = Map::new();
    map.insert("mode".to_string(), json!("full_rust_backend_production_verifier"));
    map.insert("status".to_string(), json!(status));
    map.insert("production_verifier_id".to_string(), json!(verifier_id(&Value::Object(seed))));
    map.insert("full_rust_backend".to_string(), json!(production_verified));
    map.insert("full_rust_backend_production_enabled".to_string(), json!(production_verified));
    map.insert("rust_service_runtime_authoritative".to_string(), json!(production_verified));
    map.insert("api_traffic_switched_to_rust".to_string(), json!(api_traffic_switched_to_rust));
    map.insert("flask_routes_disabled".to_string(), json!(flask_routes_disabled));
    map.insert("python_backend_removed".to_string(), json!(false));
    map.insert("python_backend_removable".to_string(), json!(retirement_executor_allowed));
    map.insert("python_removal_allowed".to_string(), json!(retirement_executor_allowed));
    map.insert("python_retirement_executor_allowed".to_string(), json!(retirement_executor_allowed));
    map.insert("python_backend_stopped_or_disabled".to_string(), json!(python_backend_stopped_or_disabled));
    map.insert("webui_ux_unchanged".to_string(), json!(webui_unchanged));
    map.insert("webui_static_assets_preserved".to_string(), json!(webui_unchanged));
    map.insert("rollback_path".to_string(), json!(rollback_path));
    map.insert("rollback_ready".to_string(), json!(rollback_ready));
    map.insert("rust_service_active".to_string(), json!(rust_service_active));
    map.insert("rust_api_healthcheck_passed".to_string(), json!(rust_api_healthcheck_passed));
    map.insert("rust_runtime_endpoint_active".to_string(), json!(rust_unix_socket_active));
    map.insert("server_tests_passed".to_string(), json!(server_tests_passed));
    map.insert("operator_acknowledged".to_string(), json!(operator_ack));
    map.insert("cutover_status".to_string(), json!(cutover_status));
    map.insert("cutover_ready".to_string(), json!(cutover_ready));
    map.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    map.insert("gates_ready".to_string(), json!(gates_ready));
    map.insert("guard_steps".to_string(), Value::Array(guard_steps));
    map.insert("note".to_string(), json!("v7.1 verifies full Rust backend production state and guards Python backend retirement. The Rust core still does not delete files or stop services directly."));

    (Value::Object(map), errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_payload() -> Value {
        let mut root = Map::new();
        root.insert("confirmation".to_string(), json!(CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER));
        root.insert("shadow_age_seconds".to_string(), json!(30));
        root.insert("webui_ux_unchanged".to_string(), json!(true));
        root.insert("webui_static_asset_paths_unchanged".to_string(), json!(true));
        root.insert("webui_static_assets_preserved".to_string(), json!(true));
        root.insert("rust_service_active".to_string(), json!(true));
        root.insert("rust_api_healthcheck_passed".to_string(), json!(true));
        root.insert("rust_unix_socket_active".to_string(), json!(true));
        root.insert("api_traffic_switched_to_rust".to_string(), json!(true));
        root.insert("flask_routes_disabled".to_string(), json!(true));
        root.insert("python_backend_stopped_or_disabled".to_string(), json!(true));
        root.insert("python_backend_rollback_package_ready".to_string(), json!(true));
        root.insert("server_cargo_tests_passed".to_string(), json!(true));
        root.insert("self_test_passed".to_string(), json!(true));
        root.insert("rollback_test_passed".to_string(), json!(true));
        root.insert("production_healthcheck_passed".to_string(), json!(true));
        root.insert("operator_full_rust_backend_production_verifier_ack".to_string(), json!(true));
        root.insert("rollback_path".to_string(), json!("restore_python_backend_and_flask_routes"));

        let mut rc = Map::new();
        rc.insert("allow_full_rust_backend_production_verifier".to_string(), json!(true));
        rc.insert("full_rust_backend_production_verifier_pilot".to_string(), json!(true));
        rc.insert("full_rust_backend_production_verifier_mode".to_string(), json!("verify_only"));
        rc.insert("full_rust_backend_production_verifier_require_cutover".to_string(), json!(true));
        rc.insert("full_rust_backend_production_verifier_require_webui_unchanged".to_string(), json!(true));
        rc.insert("full_rust_backend_production_verifier_require_runtime_health".to_string(), json!(true));
        rc.insert("full_rust_backend_production_verifier_require_rollback_package".to_string(), json!(true));
        rc.insert("full_rust_backend_production_verifier_require_manual_confirmation".to_string(), json!(true));
        rc.insert("full_rust_backend_production_verifier_require_operator_ack".to_string(), json!(true));
        rc.insert("full_rust_backend_production_verifier_require_server_tests".to_string(), json!(true));
        rc.insert("full_rust_backend_production_verifier_max_shadow_age_seconds".to_string(), json!(900));
        root.insert("rust_core".to_string(), Value::Object(rc));

        root.insert("full_rust_backend_production_cutover".to_string(), json!({
            "status": "full_rust_backend_production_cutover_ready",
            "cutover_allowed": true,
            "webui_ux_unchanged": true,
            "python_removal_allowed": true,
            "python_backend_removable": true,
            "python_backend_removed": false,
            "api_traffic_switched_to_rust": false
        }));
        Value::Object(root)
    }

    #[test]
    fn blocks_execute_attempts() {
        let (result, errors, _warnings) = build_full_rust_backend_production_verifier_payload(&json!({"execute": true}));
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
    }

    #[test]
    fn defaults_to_blocked_when_gates_missing() {
        let (result, errors, _warnings) = build_full_rust_backend_production_verifier_payload(&json!({}));
        assert!(errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("full_rust_backend_production_verifier_blocked"));
        assert_eq!(result.get("python_backend_removed").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn reports_production_verified_and_retirement_guard_allowed() {
        let (result, errors, _warnings) = build_full_rust_backend_production_verifier_payload(&ready_payload());
        assert!(errors.is_empty(), "errors: {errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("full_rust_backend_production_verified"));
        assert_eq!(result.get("full_rust_backend").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("python_retirement_executor_allowed").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("python_backend_removed").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("webui_ux_unchanged").and_then(Value::as_bool), Some(true));
    }
}
