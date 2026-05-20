use crate::protocol::Diagnostic;
use crate::rust_full_backend_production_verifier::build_full_rust_backend_production_verifier_payload;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const CONFIRM_FULL_RUST_BACKEND_POST_RETIREMENT_VERIFIER: &str = "CONFIRM_FULL_RUST_BACKEND_POST_RETIREMENT_VERIFIER";
const CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER: &str = "CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER";

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

fn post_retirement_id(seed: &Value) -> String {
    let text = serde_json::to_string(seed).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("fullrust-postretire-{}", &digest[..16])
}

/// Build the v7.2 post-retirement verifier for full Rust backend production.
///
/// This operation verifies that Python backend retirement has already been
/// performed by the guarded executor and that Rust backend authority remains
/// healthy afterwards. The Rust core still does not delete files or stop services
/// directly; it only verifies the resulting production state and rollback safety.
pub fn build_full_rust_backend_post_retirement_verifier_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(str_value(payload.get("mode"), "verify"), "execute" | "remove" | "delete" | "disable-python" | "retire-python" | "switch");
    if requested_execute {
        errors.push(Diagnostic::error(
            "full_rust_backend_post_retirement_verifier_execute_not_implemented",
            Some("full_rust_backend_post_retirement_verifier".to_string()),
            "The post-retirement verifier is non-mutating. Python retirement must be executed only by the guarded retirement script before this verifier runs.",
        ));
    }

    let allow_verify = bool_value(config_value(payload, "allow_full_rust_backend_post_retirement_verifier"), false);
    let verifier_pilot = bool_value(config_value(payload, "full_rust_backend_post_retirement_verifier_pilot"), false);
    let verifier_mode = str_value(config_value(payload, "full_rust_backend_post_retirement_verifier_mode"), "verify_only");
    let require_production_verifier = bool_value(config_value(payload, "full_rust_backend_post_retirement_require_production_verifier"), true);
    let require_runtime_health = bool_value(config_value(payload, "full_rust_backend_post_retirement_require_runtime_health"), true);
    let require_python_retired = bool_value(config_value(payload, "full_rust_backend_post_retirement_require_python_retired"), true);
    let require_webui_unchanged = bool_value(config_value(payload, "full_rust_backend_post_retirement_require_webui_unchanged"), true);
    let require_rollback_package = bool_value(config_value(payload, "full_rust_backend_post_retirement_require_rollback_package"), true);
    let require_server_tests = bool_value(config_value(payload, "full_rust_backend_post_retirement_require_server_tests"), true);
    let require_manual_confirmation = bool_value(config_value(payload, "full_rust_backend_post_retirement_require_manual_confirmation"), true);
    let require_operator_ack = bool_value(config_value(payload, "full_rust_backend_post_retirement_require_operator_ack"), true);
    let max_shadow_age = number_value(config_value(payload, "full_rust_backend_post_retirement_max_shadow_age_seconds"), 900);
    let shadow_age = number_value(payload.get("shadow_age_seconds"), 0);

    let confirmation = str_value(payload.get("confirmation"), "");
    let confirmation_ok = !require_manual_confirmation || confirmation == CONFIRM_FULL_RUST_BACKEND_POST_RETIREMENT_VERIFIER;
    if require_manual_confirmation && !confirmation_ok {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_post_retirement_verifier_confirmation_required",
            Some("confirmation".to_string()),
            "Post-retirement verification requires CONFIRM_FULL_RUST_BACKEND_POST_RETIREMENT_VERIFIER before it can report full production verified.",
        ));
    }

    if shadow_age > max_shadow_age {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_post_retirement_verifier_shadow_stale",
            Some("shadow_age_seconds".to_string()),
            "Rust production state data is older than the configured maximum age; post-retirement verification remains under review.",
        ).with_value(json!({"shadow_age_seconds": shadow_age, "max_shadow_age_seconds": max_shadow_age})));
    }

    let production_verifier_value = first_object(payload, &[
        "full_rust_backend_production_verifier",
        "full_rust_backend_production_verifier_contract",
        "rust_full_backend_production_verifier",
    ]).cloned();

    let (production_verifier, production_verifier_errors, mut production_verifier_warnings) = match production_verifier_value {
        Some(v) => (v, Vec::new(), Vec::new()),
        None => {
            let mut nested_payload = payload.clone();
            if let Some(obj) = nested_payload.as_object_mut() {
                let nested_confirmation = str_value(
                    payload.get("full_rust_backend_production_verifier_confirmation"),
                    CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER,
                );
                obj.insert("confirmation".to_string(), json!(nested_confirmation));
            }
            build_full_rust_backend_production_verifier_payload(&nested_payload)
        }
    };
    warnings.append(&mut production_verifier_warnings);

    let production_verifier_status = production_verifier.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let production_verifier_ready = production_verifier_errors.is_empty()
        && production_verifier_status == "full_rust_backend_production_verified"
        && production_verifier.get("full_rust_backend").and_then(Value::as_bool).unwrap_or(false)
        && production_verifier.get("rust_service_runtime_authoritative").and_then(Value::as_bool).unwrap_or(false)
        && production_verifier.get("python_retirement_executor_allowed").and_then(Value::as_bool).unwrap_or(false)
        && production_verifier.get("webui_ux_unchanged").and_then(Value::as_bool).unwrap_or(true);

    if require_production_verifier && !production_verifier_ready {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_post_retirement_production_verifier_not_ready",
            Some("full_rust_backend_production_verifier".to_string()),
            "Full Rust backend production verifier has not passed; post-retirement verification remains blocked or review-only.",
        ));
    }

    let rust_service_active = bool_value(payload.get("rust_service_active"), false);
    let rust_api_healthcheck_passed = bool_value(payload.get("rust_api_healthcheck_passed"), false);
    let rust_unix_socket_active = bool_value(payload.get("rust_unix_socket_active"), false) || bool_value(payload.get("rust_http_api_active"), false);
    let api_traffic_switched_to_rust = bool_value(payload.get("api_traffic_switched_to_rust"), false);
    let rust_service_runtime_authoritative = bool_value(payload.get("rust_service_runtime_authoritative"), false);
    let runtime_health_ready = rust_service_active
        && rust_api_healthcheck_passed
        && rust_unix_socket_active
        && api_traffic_switched_to_rust
        && rust_service_runtime_authoritative;
    if require_runtime_health && !runtime_health_ready {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_post_retirement_runtime_health_required",
            Some("rust_service_active".to_string()),
            "Rust service, API healthcheck, runtime socket/API, API traffic switch, and runtime authority must be verified after Python retirement.",
        ));
    }

    let flask_routes_disabled = bool_value(payload.get("flask_routes_disabled"), false);
    let python_backend_stopped_or_disabled = bool_value(payload.get("python_backend_stopped_or_disabled"), false);
    let python_backend_service_masked_or_disabled = bool_value(payload.get("python_backend_service_masked_or_disabled"), false) || bool_value(payload.get("python_backend_service_removed"), false);
    let python_backend_files_preserved_for_rollback = bool_value(payload.get("python_backend_files_preserved_for_rollback"), false) || bool_value(payload.get("python_backend_rollback_package_ready"), false);
    let python_api_routes_unregistered = bool_value(payload.get("python_api_routes_unregistered"), false) || flask_routes_disabled;
    let python_retired = flask_routes_disabled
        && python_backend_stopped_or_disabled
        && python_backend_service_masked_or_disabled
        && python_api_routes_unregistered;
    if require_python_retired && !python_retired {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_post_retirement_python_not_retired",
            Some("python_backend_stopped_or_disabled".to_string()),
            "Python backend service/routes must be stopped, disabled/masked, and unregistered before post-retirement verification can pass.",
        ));
    }

    let webui_unchanged = bool_value(payload.get("webui_ux_unchanged"), true)
        && bool_value(payload.get("webui_static_asset_paths_unchanged"), true)
        && bool_value(payload.get("webui_static_assets_preserved"), true);
    if require_webui_unchanged && !webui_unchanged {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_post_retirement_webui_changed",
            Some("webui_ux_unchanged".to_string()),
            "WebUI/UX and static asset paths must remain unchanged after Python backend retirement.",
        ));
    }

    let rollback_path = str_value(payload.get("rollback_path"), "restore_python_backend_and_flask_routes");
    let rollback_package_ready = bool_value(payload.get("python_backend_rollback_package_ready"), false)
        || bool_value(payload.get("python_fallback_backup_ready"), false);
    let rollback_test_passed = bool_value(payload.get("rollback_test_passed"), false);
    let rollback_ready = !require_rollback_package || (!rollback_path.trim().is_empty() && rollback_package_ready && rollback_test_passed && python_backend_files_preserved_for_rollback);
    if require_rollback_package && !rollback_ready {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_post_retirement_rollback_package_required",
            Some("python_backend_rollback_package_ready".to_string()),
            "Rollback package, rollback test, and preserved Python backend files must be available after retirement.",
        ));
    }

    let server_tests_passed = bool_value(payload.get("server_cargo_tests_passed"), false)
        && bool_value(payload.get("self_test_passed"), false)
        && bool_value(payload.get("production_healthcheck_passed"), false)
        && bool_value(payload.get("post_retirement_healthcheck_passed"), false);
    if require_server_tests && !server_tests_passed {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_post_retirement_server_tests_required",
            Some("server_cargo_tests_passed".to_string()),
            "Server cargo tests, self-test, production healthcheck, and post-retirement healthcheck must pass after Python retirement.",
        ));
    }

    let operator_ack = bool_value(payload.get("operator_full_rust_backend_post_retirement_ack"), false)
        || bool_value(payload.get("operator_acknowledged"), false);
    if require_operator_ack && !operator_ack {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_post_retirement_operator_ack_required",
            Some("operator_full_rust_backend_post_retirement_ack".to_string()),
            "Operator must acknowledge that Python backend retirement has completed and Rust backend remains production-authoritative.",
        ));
    }

    let gates_ready = allow_verify && verifier_pilot && verifier_mode == "verify_only";
    if !gates_ready {
        warnings.push(Diagnostic::warning(
            "full_rust_backend_post_retirement_gates_not_enabled",
            Some("rust_core".to_string()),
            "Post-retirement verifier gates are not fully enabled; report remains blocked or review-only.",
        ));
    }

    let ready = errors.is_empty()
        && gates_ready
        && confirmation_ok
        && (!require_production_verifier || production_verifier_ready)
        && (!require_runtime_health || runtime_health_ready)
        && (!require_python_retired || python_retired)
        && (!require_webui_unchanged || webui_unchanged)
        && (!require_rollback_package || rollback_ready)
        && (!require_server_tests || server_tests_passed)
        && (!require_operator_ack || operator_ack)
        && shadow_age <= max_shadow_age;

    let review = errors.is_empty() && production_verifier_ready && runtime_health_ready && webui_unchanged;
    let status = if !errors.is_empty() {
        "blocked"
    } else if ready {
        "full_rust_backend_post_retirement_verified"
    } else if review {
        "full_rust_backend_post_retirement_review"
    } else {
        "full_rust_backend_post_retirement_blocked"
    };

    let mut seed = Map::new();
    seed.insert("status".to_string(), json!(status));
    seed.insert("production_verifier_status".to_string(), json!(production_verifier_status));
    seed.insert("python_retired".to_string(), json!(python_retired));
    seed.insert("runtime_health_ready".to_string(), json!(runtime_health_ready));
    seed.insert("confirmation_ok".to_string(), json!(confirmation_ok));

    let mut map = Map::new();
    map.insert("mode".to_string(), json!("full_rust_backend_post_retirement_verifier"));
    map.insert("status".to_string(), json!(status));
    map.insert("post_retirement_verifier_id".to_string(), json!(post_retirement_id(&Value::Object(seed))));
    map.insert("full_rust_backend".to_string(), json!(ready));
    map.insert("full_rust_backend_production_enabled".to_string(), json!(ready));
    map.insert("full_rust_backend_post_retirement_verified".to_string(), json!(ready));
    map.insert("rust_service_runtime_authoritative".to_string(), json!(ready));
    map.insert("api_traffic_switched_to_rust".to_string(), json!(api_traffic_switched_to_rust));
    map.insert("flask_routes_disabled".to_string(), json!(flask_routes_disabled));
    map.insert("python_backend_removed".to_string(), json!(ready && python_retired));
    map.insert("python_backend_retired".to_string(), json!(ready && python_retired));
    map.insert("python_backend_removal_verified".to_string(), json!(ready && python_retired));
    map.insert("python_backend_stopped_or_disabled".to_string(), json!(python_backend_stopped_or_disabled));
    map.insert("python_backend_service_masked_or_disabled".to_string(), json!(python_backend_service_masked_or_disabled));
    map.insert("python_api_routes_unregistered".to_string(), json!(python_api_routes_unregistered));
    map.insert("python_backend_files_preserved_for_rollback".to_string(), json!(python_backend_files_preserved_for_rollback));
    map.insert("webui_ux_unchanged".to_string(), json!(webui_unchanged));
    map.insert("webui_static_assets_preserved".to_string(), json!(webui_unchanged));
    map.insert("rollback_path".to_string(), json!(rollback_path));
    map.insert("rollback_ready".to_string(), json!(rollback_ready));
    map.insert("rust_service_active".to_string(), json!(rust_service_active));
    map.insert("rust_api_healthcheck_passed".to_string(), json!(rust_api_healthcheck_passed));
    map.insert("rust_runtime_endpoint_active".to_string(), json!(rust_unix_socket_active));
    map.insert("runtime_health_ready".to_string(), json!(runtime_health_ready));
    map.insert("server_tests_passed".to_string(), json!(server_tests_passed));
    map.insert("operator_acknowledged".to_string(), json!(operator_ack));
    map.insert("production_verifier_status".to_string(), json!(production_verifier_status));
    map.insert("production_verifier_ready".to_string(), json!(production_verifier_ready));
    map.insert("gates_ready".to_string(), json!(gates_ready));
    map.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    map.insert("note".to_string(), json!("v7.2 verifies the full Rust backend after guarded Python backend retirement. WebUI/UX remains unchanged and rollback stays available."));

    (Value::Object(map), errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_payload() -> Value {
        let mut root = Map::new();
        root.insert("confirmation".to_string(), json!(CONFIRM_FULL_RUST_BACKEND_POST_RETIREMENT_VERIFIER));
        root.insert("shadow_age_seconds".to_string(), json!(30));
        root.insert("webui_ux_unchanged".to_string(), json!(true));
        root.insert("webui_static_asset_paths_unchanged".to_string(), json!(true));
        root.insert("webui_static_assets_preserved".to_string(), json!(true));
        root.insert("rust_service_active".to_string(), json!(true));
        root.insert("rust_api_healthcheck_passed".to_string(), json!(true));
        root.insert("rust_unix_socket_active".to_string(), json!(true));
        root.insert("api_traffic_switched_to_rust".to_string(), json!(true));
        root.insert("rust_service_runtime_authoritative".to_string(), json!(true));
        root.insert("flask_routes_disabled".to_string(), json!(true));
        root.insert("python_backend_stopped_or_disabled".to_string(), json!(true));
        root.insert("python_backend_service_masked_or_disabled".to_string(), json!(true));
        root.insert("python_api_routes_unregistered".to_string(), json!(true));
        root.insert("python_backend_files_preserved_for_rollback".to_string(), json!(true));
        root.insert("python_backend_rollback_package_ready".to_string(), json!(true));
        root.insert("rollback_test_passed".to_string(), json!(true));
        root.insert("server_cargo_tests_passed".to_string(), json!(true));
        root.insert("self_test_passed".to_string(), json!(true));
        root.insert("production_healthcheck_passed".to_string(), json!(true));
        root.insert("post_retirement_healthcheck_passed".to_string(), json!(true));
        root.insert("operator_full_rust_backend_post_retirement_ack".to_string(), json!(true));
        root.insert("rollback_path".to_string(), json!("restore_python_backend_and_flask_routes"));

        let mut verifier = Map::new();
        verifier.insert("status".to_string(), json!("full_rust_backend_production_verified"));
        verifier.insert("full_rust_backend".to_string(), json!(true));
        verifier.insert("rust_service_runtime_authoritative".to_string(), json!(true));
        verifier.insert("python_retirement_executor_allowed".to_string(), json!(true));
        verifier.insert("webui_ux_unchanged".to_string(), json!(true));
        root.insert("full_rust_backend_production_verifier".to_string(), Value::Object(verifier));

        let mut rc = Map::new();
        rc.insert("allow_full_rust_backend_post_retirement_verifier".to_string(), json!(true));
        rc.insert("full_rust_backend_post_retirement_verifier_pilot".to_string(), json!(true));
        rc.insert("full_rust_backend_post_retirement_verifier_mode".to_string(), json!("verify_only"));
        rc.insert("full_rust_backend_post_retirement_require_production_verifier".to_string(), json!(true));
        rc.insert("full_rust_backend_post_retirement_require_runtime_health".to_string(), json!(true));
        rc.insert("full_rust_backend_post_retirement_require_python_retired".to_string(), json!(true));
        rc.insert("full_rust_backend_post_retirement_require_webui_unchanged".to_string(), json!(true));
        rc.insert("full_rust_backend_post_retirement_require_rollback_package".to_string(), json!(true));
        rc.insert("full_rust_backend_post_retirement_require_server_tests".to_string(), json!(true));
        rc.insert("full_rust_backend_post_retirement_require_manual_confirmation".to_string(), json!(true));
        rc.insert("full_rust_backend_post_retirement_require_operator_ack".to_string(), json!(true));
        rc.insert("full_rust_backend_post_retirement_max_shadow_age_seconds".to_string(), json!(900));
        root.insert("rust_core".to_string(), Value::Object(rc));

        Value::Object(root)
    }

    #[test]
    fn defaults_to_blocked_when_gates_missing() {
        let (result, errors, _warnings) = build_full_rust_backend_post_retirement_verifier_payload(&json!({}));
        assert!(errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("full_rust_backend_post_retirement_blocked"));
        assert_eq!(result.get("full_rust_backend").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("python_backend_removed").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn blocks_execute_attempts() {
        let mut payload = ready_payload();
        payload.as_object_mut().unwrap().insert("execute".to_string(), json!(true));
        let (result, errors, _warnings) = build_full_rust_backend_post_retirement_verifier_payload(&payload);
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
    }

    #[test]
    fn verifies_post_retirement_full_rust_backend_state() {
        let payload = ready_payload();
        let (result, errors, _warnings) = build_full_rust_backend_post_retirement_verifier_payload(&payload);
        assert!(errors.is_empty(), "errors: {errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("full_rust_backend_post_retirement_verified"));
        assert_eq!(result.get("full_rust_backend").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("python_backend_removed").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("webui_ux_unchanged").and_then(Value::as_bool), Some(true));
    }
}
