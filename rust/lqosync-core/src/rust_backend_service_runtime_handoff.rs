use crate::protocol::Diagnostic;
use crate::rust_apply_journal_rollback_authority_handoff::build_rust_apply_journal_rollback_authority_handoff_contract_payload;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const CONFIRM_SERVICE_RUNTIME_HANDOFF: &str = "CONFIRM_RUST_BACKEND_SERVICE_RUNTIME_HANDOFF_CONTRACT";
const CONFIRM_APPLY_JOURNAL_HANDOFF: &str = "CONFIRM_RUST_APPLY_JOURNAL_ROLLBACK_AUTHORITY_HANDOFF_CONTRACT";

fn bool_value(v: Option<&Value>, default: bool) -> bool {
    v.and_then(Value::as_bool).unwrap_or(default)
}

fn str_value<'a>(v: Option<&'a Value>, default: &'a str) -> &'a str {
    v.and_then(Value::as_str).unwrap_or(default)
}

fn number_value(v: Option<&Value>, default: u64) -> u64 {
    v.and_then(Value::as_u64).unwrap_or(default)
}

fn float_value(v: Option<&Value>, default: f64) -> f64 {
    v.and_then(Value::as_f64).unwrap_or(default)
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
    format!("servicehandoff-{}", &digest[..16])
}

/// Build a Rust backend service-runtime handoff contract while Python remains fallback.
///
/// v5.9 moves the full-Rust-backend track from apply/journal/rollback authority
/// toward the outer service/API runtime layer. It is still non-mutating: Rust does
/// not bind live API traffic, does not disable Flask/Python routes, and does not
/// remove Python backend service ownership.
pub fn build_rust_backend_service_runtime_handoff_contract_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(
            str_value(payload.get("mode"), "contract"),
            "execute" | "commit" | "switch" | "remove-python" | "replace-flask" | "bind-live-api" | "production" | "authoritative"
        );
    if requested_execute {
        errors.push(Diagnostic::error(
            "rust_backend_service_runtime_handoff_execute_not_implemented",
            Some("rust_backend_service_runtime_handoff_contract".to_string()),
            "This release only builds a Rust backend service-runtime handoff contract. It does not switch API traffic, remove Python, disable Flask routes, or claim full Rust backend production.",
        ));
    }

    let allow_contract = bool_value(config_value(payload, "allow_rust_backend_service_runtime_handoff_contract"), false);
    let contract_pilot = bool_value(config_value(payload, "rust_backend_service_runtime_handoff_contract_pilot"), false);
    let handoff_mode = str_value(config_value(payload, "rust_backend_service_runtime_handoff_mode"), "contract_only");
    let require_apply_journal = bool_value(config_value(payload, "rust_backend_service_runtime_handoff_require_apply_journal_rollback_authority"), true);
    let require_python_fallback = bool_value(config_value(payload, "rust_backend_service_runtime_handoff_require_python_fallback"), true);
    let require_manual_confirmation = bool_value(config_value(payload, "rust_backend_service_runtime_handoff_require_manual_confirmation"), true);
    let require_route_parity = bool_value(config_value(payload, "rust_backend_service_runtime_handoff_require_route_parity"), true);
    let require_static_assets = bool_value(config_value(payload, "rust_backend_service_runtime_handoff_require_static_assets"), true);
    let require_service_supervision = bool_value(config_value(payload, "rust_backend_service_runtime_handoff_require_service_supervision"), true);
    let require_api_shadow = bool_value(config_value(payload, "rust_backend_service_runtime_handoff_require_api_shadow"), true);
    let require_no_side_effects = bool_value(config_value(payload, "rust_backend_service_runtime_handoff_require_no_side_effects"), true);
    let max_shadow_age = number_value(config_value(payload, "rust_backend_service_runtime_handoff_max_shadow_age_seconds"), 900);
    let shadow_age = number_value(payload.get("shadow_age_seconds"), 0);

    let confirmation = str_value(payload.get("confirmation"), "");
    let confirmation_ok = !require_manual_confirmation || confirmation == CONFIRM_SERVICE_RUNTIME_HANDOFF;
    if require_manual_confirmation && !confirmation_ok {
        warnings.push(Diagnostic::warning(
            "rust_backend_service_runtime_handoff_confirmation_required",
            Some("confirmation".to_string()),
            "Rust backend service runtime handoff requires CONFIRM_RUST_BACKEND_SERVICE_RUNTIME_HANDOFF_CONTRACT before it can report ready.",
        ));
    }

    if !require_python_fallback {
        errors.push(Diagnostic::error(
            "rust_backend_service_runtime_handoff_requires_python_fallback",
            Some("rust_core.rust_backend_service_runtime_handoff_require_python_fallback".to_string()),
            "v5.9 still requires Python backend as fallback. Python removal belongs to a later full-backend production cutover phase.",
        ));
    }

    if shadow_age > max_shadow_age {
        warnings.push(Diagnostic::warning(
            "rust_backend_service_runtime_handoff_shadow_stale",
            Some("shadow_age_seconds".to_string()),
            "Rust-shadow data is older than the configured maximum age; backend service runtime handoff remains under review.",
        ).with_value(json!({"shadow_age_seconds": shadow_age, "max_shadow_age_seconds": max_shadow_age})));
    }

    let apply_value = first_object(payload, &[
        "rust_apply_journal_rollback_authority_handoff_contract",
        "apply_journal_rollback_authority_handoff_contract",
        "rust_apply_journal_rollback_authority_handoff",
    ]).cloned();

    let (apply_handoff, apply_errors, mut apply_warnings) = match apply_value {
        Some(v) => (v, Vec::new(), Vec::new()),
        None => {
            let mut nested_payload = payload.clone();
            if let Some(obj) = nested_payload.as_object_mut() {
                let nested_confirmation = str_value(
                    payload.get("rust_apply_journal_rollback_authority_handoff_confirmation"),
                    CONFIRM_APPLY_JOURNAL_HANDOFF,
                );
                obj.insert("confirmation".to_string(), json!(nested_confirmation));
            }
            build_rust_apply_journal_rollback_authority_handoff_contract_payload(&nested_payload)
        }
    };
    warnings.append(&mut apply_warnings);

    if !apply_errors.is_empty() {
        warnings.push(Diagnostic::warning(
            "rust_backend_service_runtime_handoff_apply_journal_not_clean",
            Some("rust_apply_journal_rollback_authority_handoff_contract".to_string()),
            "Rust apply/journal/rollback authority handoff returned errors; service runtime handoff remains shadow-only.",
        ));
    }

    let apply_status = apply_handoff.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let apply_ready = apply_errors.is_empty()
        && apply_status == "rust_apply_journal_rollback_authority_handoff_contract_ready"
        && apply_handoff.get("rust_apply_journal_rollback_authority_handoff_ready").and_then(Value::as_bool).unwrap_or(false)
        && apply_handoff.get("rust_apply_journal_rollback_authoritative").and_then(Value::as_bool).unwrap_or(false) == false
        && apply_handoff.get("python_apply_journal_rollback_authoritative").and_then(Value::as_bool).unwrap_or(true);

    if require_apply_journal && !apply_ready {
        warnings.push(Diagnostic::warning(
            "rust_backend_service_runtime_handoff_apply_journal_not_ready",
            Some("rust_apply_journal_rollback_authority_handoff_contract".to_string()),
            "Rust apply/journal/rollback authority handoff contract has not passed; backend service runtime handoff remains shadow-only or under review.",
        ));
    }

    let route_parity_score = float_value(payload.get("api_route_parity_score"), 0.0);
    let route_parity_ready = bool_value(payload.get("api_route_parity_ready"), false)
        && bool_value(payload.get("webui_ux_unchanged"), false)
        && route_parity_score >= 100.0;
    if require_route_parity && !route_parity_ready {
        warnings.push(Diagnostic::warning(
            "rust_backend_service_runtime_handoff_route_parity_required",
            Some("api_route_parity_ready".to_string()),
            "Rust backend service runtime handoff requires WebUI/API route parity verification before it can report ready.",
        ));
    }

    let static_assets_ready = bool_value(payload.get("static_assets_compat_ready"), false)
        && bool_value(payload.get("webui_static_asset_paths_unchanged"), false)
        && number_value(payload.get("static_asset_compat_error_count"), 0) == 0;
    if require_static_assets && !static_assets_ready {
        warnings.push(Diagnostic::warning(
            "rust_backend_service_runtime_handoff_static_assets_required",
            Some("static_assets_compat_ready".to_string()),
            "Rust backend service runtime handoff requires static WebUI asset compatibility verification before it can report ready.",
        ));
    }

    let service_supervision_ready = bool_value(payload.get("rust_service_supervision_shadow_ready"), false)
        && bool_value(payload.get("rust_daemon_socket_shadow_ready"), false)
        && bool_value(payload.get("rust_service_healthcheck_shadow_ready"), false)
        && number_value(payload.get("rust_service_supervision_error_count"), 0) == 0;
    if require_service_supervision && !service_supervision_ready {
        warnings.push(Diagnostic::warning(
            "rust_backend_service_runtime_handoff_service_supervision_required",
            Some("rust_service_supervision_shadow_ready".to_string()),
            "Rust backend service runtime handoff requires service supervision/socket/healthcheck shadow verification before it can report ready.",
        ));
    }

    let api_shadow_ready = bool_value(payload.get("rust_api_shadow_ready"), false)
        && bool_value(payload.get("rust_api_response_parity_ready"), false)
        && number_value(payload.get("rust_api_shadow_error_count"), 0) == 0;
    if require_api_shadow && !api_shadow_ready {
        warnings.push(Diagnostic::warning(
            "rust_backend_service_runtime_handoff_api_shadow_required",
            Some("rust_api_shadow_ready".to_string()),
            "Rust backend service runtime handoff requires Rust API shadow response parity before it can report ready.",
        ));
    }

    let side_effect_free = !any_side_effect(payload);
    if require_no_side_effects && !side_effect_free {
        errors.push(Diagnostic::error(
            "rust_backend_service_runtime_handoff_side_effect_detected",
            Some("rust_backend_service_runtime_handoff_contract".to_string()),
            "Rust backend service runtime handoff side effects are forbidden in this release.",
        ));
    }

    let gates_ready = allow_contract && contract_pilot && handoff_mode == "contract_only";
    if !gates_ready {
        warnings.push(Diagnostic::warning(
            "rust_backend_service_runtime_handoff_gates_not_enabled",
            Some("rust_core".to_string()),
            "Rust backend service runtime handoff gates are not enabled.",
        ));
    }

    let ready = errors.is_empty()
        && gates_ready
        && confirmation_ok
        && (!require_apply_journal || apply_ready)
        && require_python_fallback
        && (!require_route_parity || route_parity_ready)
        && (!require_static_assets || static_assets_ready)
        && (!require_service_supervision || service_supervision_ready)
        && (!require_api_shadow || api_shadow_ready)
        && side_effect_free
        && shadow_age <= max_shadow_age;
    let review = errors.is_empty()
        && apply_ready
        && route_parity_ready
        && static_assets_ready
        && service_supervision_ready
        && api_shadow_ready
        && side_effect_free;
    let status = if !errors.is_empty() {
        "blocked"
    } else if ready {
        "rust_backend_service_runtime_handoff_contract_ready"
    } else if review {
        "rust_backend_service_runtime_handoff_contract_review"
    } else {
        "rust_backend_service_runtime_handoff_contract_shadow_only"
    };

    let mut seed = Map::new();
    seed.insert("status".to_string(), json!(status));
    seed.insert("apply_status".to_string(), json!(apply_status));
    seed.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    seed.insert("confirmation_ok".to_string(), json!(confirmation_ok));

    let mut map = Map::new();
    map.insert("mode".to_string(), json!("rust_backend_service_runtime_handoff_contract"));
    map.insert("status".to_string(), json!(status));
    map.insert("handoff_contract_id".to_string(), json!(handoff_id(&Value::Object(seed))));
    map.insert("rust_backend_service_runtime_handoff_ready".to_string(), json!(ready));
    map.insert("apply_journal_rollback_authority_handoff_ready".to_string(), json!(apply_ready));
    map.insert("api_route_parity_ready".to_string(), json!(route_parity_ready));
    map.insert("api_route_parity_score".to_string(), json!(route_parity_score));
    map.insert("static_assets_compat_ready".to_string(), json!(static_assets_ready));
    map.insert("service_supervision_shadow_ready".to_string(), json!(service_supervision_ready));
    map.insert("rust_api_shadow_ready".to_string(), json!(api_shadow_ready));
    map.insert("webui_ux_unchanged".to_string(), json!(true));
    map.insert("full_rust_backend".to_string(), json!(false));
    map.insert("python_backend_removable".to_string(), json!(false));
    map.insert("python_backend_removed".to_string(), json!(false));
    map.insert("python_backend_required".to_string(), json!(true));
    map.insert("python_backend_fallback_required".to_string(), json!(true));
    map.insert("python_service_runtime_authoritative".to_string(), json!(true));
    map.insert("rust_service_runtime_authoritative".to_string(), json!(false));
    map.insert("python_api_routes_authoritative".to_string(), json!(true));
    map.insert("rust_api_routes_authoritative".to_string(), json!(false));
    map.insert("safe_for_cleanup".to_string(), json!(false));
    map.insert("write_allowed".to_string(), json!(false));
    map.insert("apply_allowed".to_string(), json!(false));
    map.insert("api_traffic_switch_allowed".to_string(), json!(false));
    map.insert("flask_disable_allowed".to_string(), json!(false));
    map.insert("next_stage".to_string(), json!("full_rust_backend_production_readiness_gate"));
    map.insert("note".to_string(), json!("v5.9 builds a non-mutating Rust backend service-runtime handoff contract while keeping Python service/API runtime authoritative and WebUI/UX unchanged."));

    (Value::Object(map), errors, warnings)
}

fn any_side_effect(payload: &Value) -> bool {
    bool_value(payload.get("python_backend_removed"), false)
        || bool_value(payload.get("flask_routes_disabled"), false)
        || bool_value(payload.get("api_traffic_switched_to_rust"), false)
        || bool_value(payload.get("rust_backend_live_bound"), false)
        || bool_value(payload.get("service_runtime_switched_to_rust"), false)
        || bool_value(payload.get("apply_attempted"), false)
        || bool_value(payload.get("cleanup_attempted"), false)
        || bool_value(payload.get("shaped_devices_write_attempted"), false)
        || bool_value(payload.get("journal_append_attempted"), false)
        || bool_value(payload.get("rollback_execute_attempted"), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_payload() -> Value {
        let mut payload = Map::new();
        payload.insert("confirmation".to_string(), json!(CONFIRM_SERVICE_RUNTIME_HANDOFF));
        payload.insert("shadow_age_seconds".to_string(), json!(30));
        payload.insert("api_route_parity_ready".to_string(), json!(true));
        payload.insert("api_route_parity_score".to_string(), json!(100.0));
        payload.insert("webui_ux_unchanged".to_string(), json!(true));
        payload.insert("static_assets_compat_ready".to_string(), json!(true));
        payload.insert("webui_static_asset_paths_unchanged".to_string(), json!(true));
        payload.insert("static_asset_compat_error_count".to_string(), json!(0));
        payload.insert("rust_service_supervision_shadow_ready".to_string(), json!(true));
        payload.insert("rust_daemon_socket_shadow_ready".to_string(), json!(true));
        payload.insert("rust_service_healthcheck_shadow_ready".to_string(), json!(true));
        payload.insert("rust_service_supervision_error_count".to_string(), json!(0));
        payload.insert("rust_api_shadow_ready".to_string(), json!(true));
        payload.insert("rust_api_response_parity_ready".to_string(), json!(true));
        payload.insert("rust_api_shadow_error_count".to_string(), json!(0));

        let mut rc = Map::new();
        rc.insert("allow_rust_backend_service_runtime_handoff_contract".to_string(), json!(true));
        rc.insert("rust_backend_service_runtime_handoff_contract_pilot".to_string(), json!(true));
        rc.insert("rust_backend_service_runtime_handoff_mode".to_string(), json!("contract_only"));
        rc.insert("rust_backend_service_runtime_handoff_require_apply_journal_rollback_authority".to_string(), json!(true));
        rc.insert("rust_backend_service_runtime_handoff_require_python_fallback".to_string(), json!(true));
        rc.insert("rust_backend_service_runtime_handoff_require_manual_confirmation".to_string(), json!(true));
        rc.insert("rust_backend_service_runtime_handoff_require_route_parity".to_string(), json!(true));
        rc.insert("rust_backend_service_runtime_handoff_require_static_assets".to_string(), json!(true));
        rc.insert("rust_backend_service_runtime_handoff_require_service_supervision".to_string(), json!(true));
        rc.insert("rust_backend_service_runtime_handoff_require_api_shadow".to_string(), json!(true));
        rc.insert("rust_backend_service_runtime_handoff_require_no_side_effects".to_string(), json!(true));
        rc.insert("rust_backend_service_runtime_handoff_max_shadow_age_seconds".to_string(), json!(900));
        payload.insert("rust_core".to_string(), Value::Object(rc));

        let mut apply = Map::new();
        apply.insert("status".to_string(), json!("rust_apply_journal_rollback_authority_handoff_contract_ready"));
        apply.insert("rust_apply_journal_rollback_authority_handoff_ready".to_string(), json!(true));
        apply.insert("rust_apply_journal_rollback_authoritative".to_string(), json!(false));
        apply.insert("python_apply_journal_rollback_authoritative".to_string(), json!(true));
        payload.insert("rust_apply_journal_rollback_authority_handoff_contract".to_string(), Value::Object(apply));

        Value::Object(payload)
    }

    #[test]
    fn defaults_to_shadow_only_service_runtime_handoff() {
        let (result, errors, _warnings) = build_rust_backend_service_runtime_handoff_contract_payload(&json!({}));
        assert!(errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("rust_backend_service_runtime_handoff_contract_shadow_only"));
        assert_eq!(result.get("rust_service_runtime_authoritative").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("python_service_runtime_authoritative").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn blocks_execute_attempts() {
        let mut payload = ready_payload();
        payload.as_object_mut().unwrap().insert("execute".to_string(), json!(true));
        let (result, errors, _warnings) = build_rust_backend_service_runtime_handoff_contract_payload(&payload);
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
    }

    #[test]
    fn builds_ready_service_runtime_handoff_without_removing_python() {
        let payload = ready_payload();
        let (result, errors, _warnings) = build_rust_backend_service_runtime_handoff_contract_payload(&payload);
        assert!(errors.is_empty(), "errors: {errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("rust_backend_service_runtime_handoff_contract_ready"));
        assert_eq!(result.get("rust_backend_service_runtime_handoff_ready").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("rust_service_runtime_authoritative").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("python_service_runtime_authoritative").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("api_traffic_switch_allowed").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("flask_disable_allowed").and_then(Value::as_bool), Some(false));
    }
}
