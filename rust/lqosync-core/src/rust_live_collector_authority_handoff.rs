use crate::protocol::Diagnostic;
use crate::rust_config_state_authority_handoff::build_rust_config_state_authority_handoff_contract_payload;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const CONFIRM_LIVE_COLLECTOR_AUTHORITY_HANDOFF: &str = "CONFIRM_RUST_LIVE_COLLECTOR_AUTHORITY_HANDOFF_CONTRACT";
const CONFIRM_CONFIG_STATE_AUTHORITY_HANDOFF: &str = "CONFIRM_RUST_CONFIG_STATE_AUTHORITY_HANDOFF_CONTRACT";

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
    format!("lchandoff-{}", &digest[..16])
}

/// Build a Rust live collector authority handoff contract while Python remains the fallback.
///
/// v5.5 moves the full-Rust-backend track from config/state authority planning toward
/// live RouterOS collector execution authority. It validates live collector shadow/parity
/// evidence and prerequisite config/state handoff, but it does not switch live collector
/// authority to Rust and does not remove the Python backend.
pub fn build_rust_live_collector_authority_handoff_contract_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(
            str_value(payload.get("mode"), "contract"),
            "execute" | "commit" | "switch" | "remove-python" | "replace-collector" | "production" | "authoritative" | "live"
        );
    if requested_execute {
        errors.push(Diagnostic::error(
            "rust_live_collector_authority_handoff_execute_not_implemented",
            Some("rust_live_collector_authority_handoff_contract".to_string()),
            "This release only builds a Rust live collector authority handoff contract. It does not switch live collector authority or remove Python.",
        ));
    }

    let allow_contract = bool_value(config_value(payload, "allow_rust_live_collector_authority_handoff_contract"), false);
    let contract_pilot = bool_value(config_value(payload, "rust_live_collector_authority_handoff_contract_pilot"), false);
    let handoff_mode = str_value(config_value(payload, "rust_live_collector_authority_handoff_mode"), "contract_only");
    let require_config_state = bool_value(config_value(payload, "rust_live_collector_authority_handoff_require_config_state_authority"), true);
    let require_python_fallback = bool_value(config_value(payload, "rust_live_collector_authority_handoff_require_python_fallback"), true);
    let require_manual_confirmation = bool_value(config_value(payload, "rust_live_collector_authority_handoff_require_manual_confirmation"), true);
    let require_live_collector_shadow = bool_value(config_value(payload, "rust_live_collector_authority_handoff_require_live_collector_shadow"), true);
    let require_routeros_adapter_shadow = bool_value(config_value(payload, "rust_live_collector_authority_handoff_require_routeros_adapter_shadow"), true);
    let require_collector_parity = bool_value(config_value(payload, "rust_live_collector_authority_handoff_require_collector_parity"), true);
    let require_no_side_effects = bool_value(config_value(payload, "rust_live_collector_authority_handoff_require_no_side_effects"), true);
    let max_shadow_age = number_value(config_value(payload, "rust_live_collector_authority_handoff_max_shadow_age_seconds"), 900);
    let shadow_age = number_value(payload.get("shadow_age_seconds"), 0);

    let confirmation = str_value(payload.get("confirmation"), "");
    let confirmation_ok = !require_manual_confirmation || confirmation == CONFIRM_LIVE_COLLECTOR_AUTHORITY_HANDOFF;
    if require_manual_confirmation && !confirmation_ok {
        warnings.push(Diagnostic::warning(
            "rust_live_collector_authority_handoff_confirmation_required",
            Some("confirmation".to_string()),
            "Rust live collector authority handoff requires CONFIRM_RUST_LIVE_COLLECTOR_AUTHORITY_HANDOFF_CONTRACT before it can report ready.",
        ));
    }

    if !require_python_fallback {
        errors.push(Diagnostic::error(
            "rust_live_collector_authority_handoff_requires_python_fallback",
            Some("rust_core.rust_live_collector_authority_handoff_require_python_fallback".to_string()),
            "v5.5 still requires Python live collector backend as fallback. Python removal belongs to a later full-backend execution phase.",
        ));
    }

    if shadow_age > max_shadow_age {
        warnings.push(Diagnostic::warning(
            "rust_live_collector_authority_handoff_shadow_stale",
            Some("shadow_age_seconds".to_string()),
            "Rust-shadow data is older than the configured maximum age; live collector authority handoff remains under review.",
        ).with_value(json!({"shadow_age_seconds": shadow_age, "max_shadow_age_seconds": max_shadow_age})));
    }

    let config_state_value = first_object(payload, &[
        "rust_config_state_authority_handoff_contract",
        "config_state_authority_handoff_contract",
        "rust_config_state_authority_handoff",
    ]).cloned();

    let (config_state_handoff, config_state_errors, mut config_state_warnings) = match config_state_value {
        Some(v) => (v, Vec::new(), Vec::new()),
        None => {
            let mut nested_payload = payload.clone();
            if let Some(obj) = nested_payload.as_object_mut() {
                let nested_confirmation = str_value(
                    payload.get("rust_config_state_authority_handoff_confirmation"),
                    CONFIRM_CONFIG_STATE_AUTHORITY_HANDOFF,
                );
                obj.insert("confirmation".to_string(), json!(nested_confirmation));
            }
            build_rust_config_state_authority_handoff_contract_payload(&nested_payload)
        }
    };
    warnings.append(&mut config_state_warnings);

    if !config_state_errors.is_empty() {
        warnings.push(Diagnostic::warning(
            "rust_live_collector_authority_handoff_config_state_not_clean",
            Some("rust_config_state_authority_handoff_contract".to_string()),
            "Rust config/state authority handoff returned errors; live collector authority handoff remains shadow-only.",
        ));
    }

    let config_state_status = config_state_handoff.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let config_state_ready = config_state_errors.is_empty()
        && config_state_status == "rust_config_state_authority_handoff_contract_ready"
        && config_state_handoff.get("rust_config_state_authority_handoff_ready").and_then(Value::as_bool).unwrap_or(false)
        && config_state_handoff.get("rust_config_state_authoritative").and_then(Value::as_bool).unwrap_or(false) == false
        && config_state_handoff.get("python_config_state_authoritative").and_then(Value::as_bool).unwrap_or(true);

    if require_config_state && !config_state_ready {
        warnings.push(Diagnostic::warning(
            "rust_live_collector_authority_handoff_config_state_not_ready",
            Some("rust_config_state_authority_handoff_contract".to_string()),
            "Rust config/state authority handoff contract has not passed; live collector authority handoff remains shadow-only or under review.",
        ));
    }

    let live_collector_shadow_ready = bool_value(payload.get("live_collector_shadow_ready"), false);
    let live_collector_shadow_count = number_value(payload.get("live_collector_shadow_count"), 0);
    let live_collector_ready = !require_live_collector_shadow || (live_collector_shadow_ready && live_collector_shadow_count > 0);
    if require_live_collector_shadow && !live_collector_ready {
        warnings.push(Diagnostic::warning(
            "rust_live_collector_authority_handoff_live_shadow_required",
            Some("live_collector_shadow_ready".to_string()),
            "Rust live collector authority handoff requires live collector shadow verification before it can report ready.",
        ));
    }

    let routeros_adapter_shadow_ready = bool_value(payload.get("routeros_live_adapter_shadow_ready"), false);
    let routeros_adapter_shadow_count = number_value(payload.get("routeros_live_adapter_shadow_count"), 0);
    let routeros_adapter_ready = !require_routeros_adapter_shadow || (routeros_adapter_shadow_ready && routeros_adapter_shadow_count > 0);
    if require_routeros_adapter_shadow && !routeros_adapter_ready {
        warnings.push(Diagnostic::warning(
            "rust_live_collector_authority_handoff_routeros_adapter_shadow_required",
            Some("routeros_live_adapter_shadow_ready".to_string()),
            "Rust live collector authority handoff requires RouterOS live adapter shadow verification before it can report ready.",
        ));
    }

    let parity_score = payload.get("collector_parity_score").and_then(Value::as_f64).unwrap_or(0.0);
    let parity_verdict = str_value(payload.get("collector_parity_verdict"), "not_available");
    let collector_parity_ready = !require_collector_parity || (parity_verdict == "parity_pass" && parity_score >= 99.0);
    if require_collector_parity && !collector_parity_ready {
        warnings.push(Diagnostic::warning(
            "rust_live_collector_authority_handoff_parity_required",
            Some("collector_parity_verdict".to_string()),
            "Rust live collector authority handoff requires collector parity pass before it can report ready.",
        ).with_value(json!({"collector_parity_verdict": parity_verdict, "collector_parity_score": parity_score})));
    }

    let side_effect_free = !bool_value(payload.get("live_collector_authority_switched_to_rust"), false)
        && !bool_value(payload.get("python_backend_removed"), false)
        && !bool_value(payload.get("routeros_live_write_attempted"), false)
        && !bool_value(payload.get("config_write_attempted"), false)
        && !bool_value(payload.get("state_write_attempted"), false);
    if require_no_side_effects && !side_effect_free {
        errors.push(Diagnostic::error(
            "rust_live_collector_authority_handoff_side_effect_detected",
            Some("rust_live_collector_authority_handoff_contract".to_string()),
            "Live collector handoff side effects are forbidden in v5.5.",
        ));
    }

    let gates_ready = allow_contract && contract_pilot && handoff_mode == "contract_only";
    if !gates_ready {
        warnings.push(Diagnostic::warning(
            "rust_live_collector_authority_handoff_gates_not_enabled",
            Some("rust_core".to_string()),
            "Rust live collector authority handoff gates are not enabled.",
        ));
    }

    let ready = errors.is_empty()
        && gates_ready
        && confirmation_ok
        && (!require_config_state || config_state_ready)
        && require_python_fallback
        && live_collector_ready
        && routeros_adapter_ready
        && collector_parity_ready
        && side_effect_free
        && shadow_age <= max_shadow_age;
    let review = errors.is_empty() && config_state_ready && live_collector_ready && routeros_adapter_ready && collector_parity_ready && side_effect_free;
    let status = if !errors.is_empty() {
        "blocked"
    } else if ready {
        "rust_live_collector_authority_handoff_contract_ready"
    } else if review {
        "rust_live_collector_authority_handoff_contract_review"
    } else {
        "rust_live_collector_authority_handoff_contract_shadow_only"
    };

    let mut seed = Map::new();
    seed.insert("status".to_string(), json!(status));
    seed.insert("config_state_status".to_string(), json!(config_state_status));
    seed.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    seed.insert("parity_verdict".to_string(), json!(parity_verdict));

    let mut map = Map::new();
    map.insert("mode".to_string(), json!("rust_live_collector_authority_handoff_contract"));
    map.insert("status".to_string(), json!(status));
    map.insert("handoff_contract_id".to_string(), json!(handoff_id(&Value::Object(seed))));
    map.insert("rust_live_collector_authority_handoff_ready".to_string(), json!(ready));
    map.insert("config_state_authority_handoff_ready".to_string(), json!(config_state_ready));
    map.insert("live_collector_shadow_ready".to_string(), json!(live_collector_ready));
    map.insert("routeros_live_adapter_shadow_ready".to_string(), json!(routeros_adapter_ready));
    map.insert("collector_parity_ready".to_string(), json!(collector_parity_ready));
    map.insert("collector_parity_verdict".to_string(), json!(parity_verdict));
    map.insert("collector_parity_score".to_string(), json!(parity_score));
    map.insert("webui_ux_unchanged".to_string(), json!(true));
    map.insert("full_rust_backend".to_string(), json!(false));
    map.insert("python_backend_removable".to_string(), json!(false));
    map.insert("python_backend_removed".to_string(), json!(false));
    map.insert("python_backend_required".to_string(), json!(true));
    map.insert("python_backend_fallback_required".to_string(), json!(true));
    map.insert("python_live_collector_authoritative".to_string(), json!(true));
    map.insert("rust_live_collector_authoritative".to_string(), json!(false));
    map.insert("rust_config_state_authoritative".to_string(), json!(false));
    map.insert("rust_run_cycle_authoritative".to_string(), json!(false));
    map.insert("rust_api_service_authoritative".to_string(), json!(false));
    map.insert("rust_apply_authoritative".to_string(), json!(false));
    map.insert("routeros_live_write_allowed".to_string(), json!(false));
    map.insert("safe_for_cleanup".to_string(), json!(false));
    map.insert("write_allowed".to_string(), json!(false));
    map.insert("apply_allowed".to_string(), json!(false));
    map.insert("next_stage".to_string(), json!("rust_circuit_builder_authority_handoff_contract"));

    (Value::Object(map), errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_payload() -> Value {
        let mut rc = Map::new();
        rc.insert("rust_live_collector_authority_handoff_contract_pilot".to_string(), json!(true));
        rc.insert("allow_rust_live_collector_authority_handoff_contract".to_string(), json!(true));
        rc.insert("rust_live_collector_authority_handoff_mode".to_string(), json!("contract_only"));
        rc.insert("rust_live_collector_authority_handoff_require_config_state_authority".to_string(), json!(true));
        rc.insert("rust_live_collector_authority_handoff_require_python_fallback".to_string(), json!(true));
        rc.insert("rust_live_collector_authority_handoff_require_manual_confirmation".to_string(), json!(true));
        rc.insert("rust_live_collector_authority_handoff_require_live_collector_shadow".to_string(), json!(true));
        rc.insert("rust_live_collector_authority_handoff_require_routeros_adapter_shadow".to_string(), json!(true));
        rc.insert("rust_live_collector_authority_handoff_require_collector_parity".to_string(), json!(true));
        rc.insert("rust_live_collector_authority_handoff_require_no_side_effects".to_string(), json!(true));
        rc.insert("rust_live_collector_authority_handoff_max_shadow_age_seconds".to_string(), json!(900));

        let mut config_state = Map::new();
        config_state.insert("status".to_string(), json!("rust_config_state_authority_handoff_contract_ready"));
        config_state.insert("rust_config_state_authority_handoff_ready".to_string(), json!(true));
        config_state.insert("python_config_state_authoritative".to_string(), json!(true));
        config_state.insert("rust_config_state_authoritative".to_string(), json!(false));

        let mut payload = Map::new();
        payload.insert("rust_core".to_string(), Value::Object(rc));
        payload.insert("confirmation".to_string(), json!(CONFIRM_LIVE_COLLECTOR_AUTHORITY_HANDOFF));
        payload.insert("shadow_age_seconds".to_string(), json!(30));
        payload.insert("rust_config_state_authority_handoff_contract".to_string(), Value::Object(config_state));
        payload.insert("live_collector_shadow_ready".to_string(), json!(true));
        payload.insert("live_collector_shadow_count".to_string(), json!(3));
        payload.insert("routeros_live_adapter_shadow_ready".to_string(), json!(true));
        payload.insert("routeros_live_adapter_shadow_count".to_string(), json!(3));
        payload.insert("collector_parity_verdict".to_string(), json!("parity_pass"));
        payload.insert("collector_parity_score".to_string(), json!(100.0));
        Value::Object(payload)
    }

    #[test]
    fn defaults_to_shadow_only_live_collector_handoff() {
        let (result, errors, _warnings) = build_rust_live_collector_authority_handoff_contract_payload(&json!({}));
        assert!(errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("rust_live_collector_authority_handoff_contract_shadow_only"));
        assert_eq!(result.get("rust_live_collector_authoritative").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn blocks_execute_attempts() {
        let (result, errors, _warnings) = build_rust_live_collector_authority_handoff_contract_payload(&json!({"execute": true}));
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
    }

    #[test]
    fn builds_ready_live_collector_handoff_without_switching_authority() {
        let payload = ready_payload();
        let (result, errors, _warnings) = build_rust_live_collector_authority_handoff_contract_payload(&payload);
        assert!(errors.is_empty(), "errors: {errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("rust_live_collector_authority_handoff_contract_ready"));
        assert_eq!(result.get("rust_live_collector_authority_handoff_ready").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("rust_live_collector_authoritative").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("python_live_collector_authoritative").and_then(Value::as_bool), Some(true));
    }
}
