use crate::collector_authority_pilot_result::evaluate_collector_authority_pilot_result_payload;
use crate::protocol::Diagnostic;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const CONFIRM_PROMOTION_READINESS: &str = "CONFIRM_COLLECTOR_AUTHORITY_PROMOTION_READINESS";

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

fn promotion_id(seed: &Value) -> String {
    let text = serde_json::to_string(seed).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("caprdy-{}", &digest[..16])
}

/// Build a non-mutating readiness report for a future collector-authority promotion.
///
/// v4.5 is still a bridge: it reviews the pilot result and promotion gates, but it
/// does not promote Rust collectors to production authority, does not drive cleanup,
/// and does not write generated LibreQoS files.
pub fn build_collector_authority_promotion_readiness_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(str_value(payload.get("mode"), "readiness"), "execute" | "switch" | "promote" | "authority" | "apply" | "production");
    if requested_execute {
        errors.push(Diagnostic::error(
            "collector_authority_promotion_execute_not_implemented",
            Some("collector_authority_promotion_readiness".to_string()),
            "This release only builds a collector authority promotion readiness report. It does not promote Rust collectors, drive cleanup, write files, or apply LibreQoS.",
        ));
    }

    let allow_readiness = bool_value(config_value(payload, "allow_collector_authority_promotion_readiness"), false);
    let readiness_pilot = bool_value(config_value(payload, "collector_authority_promotion_readiness_pilot"), false);
    let readiness_mode = str_value(config_value(payload, "collector_authority_promotion_readiness_mode"), "readiness_only");
    let require_pilot_result = bool_value(config_value(payload, "collector_authority_promotion_require_pilot_result"), true);
    let require_python_fallback = bool_value(config_value(payload, "collector_authority_promotion_require_python_fallback"), true);
    let require_manual_confirmation = bool_value(config_value(payload, "collector_authority_promotion_require_manual_confirmation"), true);
    let require_no_side_effects = bool_value(config_value(payload, "collector_authority_promotion_require_no_cleanup_apply"), true);
    let require_diagnostic_observation = bool_value(config_value(payload, "collector_authority_promotion_require_diagnostic_observation"), true);
    let max_shadow_age = number_value(config_value(payload, "collector_authority_promotion_max_shadow_age_seconds"), 900);
    let shadow_age = number_value(payload.get("shadow_age_seconds"), 0);

    let confirmation = str_value(payload.get("confirmation"), "");
    let confirmation_ok = !require_manual_confirmation || confirmation == CONFIRM_PROMOTION_READINESS;
    if require_manual_confirmation && !confirmation_ok {
        warnings.push(Diagnostic::warning(
            "collector_authority_promotion_confirmation_required",
            Some("confirmation".to_string()),
            "Collector authority promotion readiness requires CONFIRM_COLLECTOR_AUTHORITY_PROMOTION_READINESS before it can report ready.",
        ));
    }

    if !require_python_fallback {
        errors.push(Diagnostic::error(
            "collector_authority_promotion_requires_python_fallback",
            Some("rust_core.collector_authority_promotion_require_python_fallback".to_string()),
            "Collector authority promotion readiness requires Python collector fallback in this release.",
        ));
    }

    if shadow_age > max_shadow_age {
        warnings.push(Diagnostic::warning(
            "collector_authority_promotion_shadow_stale",
            Some("shadow_age_seconds".to_string()),
            "Rust-shadow collector data is older than the configured maximum age; promotion readiness remains under review.",
        ).with_value(json!({"shadow_age_seconds": shadow_age, "max_shadow_age_seconds": max_shadow_age})));
    }

    let pilot_result_value = first_object(payload, &[
        "collector_authority_pilot_result_evaluation",
        "pilot_result_evaluation",
        "collector_authority_pilot_result",
    ]).cloned();

    let (pilot_result, pilot_errors, mut pilot_warnings) = match pilot_result_value {
        Some(v) => (v, Vec::new(), Vec::new()),
        None => evaluate_collector_authority_pilot_result_payload(payload),
    };
    warnings.append(&mut pilot_warnings);

    if !pilot_errors.is_empty() {
        warnings.push(Diagnostic::warning(
            "collector_authority_promotion_pilot_result_not_clean",
            Some("collector_authority_pilot_result".to_string()),
            "Pilot result evaluator returned errors; promotion readiness remains shadow-only.",
        ));
    }

    let pilot_status = pilot_result.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let pilot_result_pass = pilot_errors.is_empty()
        && pilot_status == "collector_authority_pilot_result_pass"
        && pilot_result.get("production_collector_authority_switched").and_then(Value::as_bool) == Some(false)
        && pilot_result.get("collector_authority_pilot_result_evaluated").and_then(Value::as_bool).unwrap_or(false)
        && pilot_result.get("python_collector_fallback_required").and_then(Value::as_bool).unwrap_or(true);

    let diagnostic_observation_ready = bool_value(pilot_result.get("diagnostic_observation_ready"), false);
    let production_row_authority = str_value(pilot_result.get("production_row_authority"), "python_collector");
    let cleanup_row_authority = str_value(pilot_result.get("cleanup_row_authority"), "python_collector");
    let diagnostic_row_authority = str_value(pilot_result.get("diagnostic_row_authority"), "python_authoritative");
    let observed_rust_row_count = number_value(pilot_result.get("observed_rust_row_count"), 0);
    let observed_python_row_count = number_value(pilot_result.get("observed_python_row_count"), 0);
    let runtime_evidence_source = str_value(pilot_result.get("runtime_evidence_source"), "not_ready");
    let diagnostic_observation_ok = pilot_result_pass
        && diagnostic_observation_ready
        && observed_rust_row_count > 0
        && observed_python_row_count > 0
        && diagnostic_row_authority == "rust_shadow_diagnostics"
        && production_row_authority == "python_collector"
        && cleanup_row_authority == "python_collector";

    if require_pilot_result && !pilot_result_pass {
        warnings.push(Diagnostic::warning(
            "collector_authority_promotion_pilot_result_not_passed",
            Some("collector_authority_pilot_result".to_string()),
            "Collector authority pilot result has not passed; promotion readiness remains shadow-only or under review.",
        ));
    }
    if require_diagnostic_observation && !diagnostic_observation_ok {
        warnings.push(Diagnostic::warning(
            "collector_authority_promotion_diagnostic_observation_not_ready",
            Some("collector_authority_pilot_result".to_string()),
            "Promotion readiness requires a pilot result backed by diagnostics-only Rust observation evidence.",
        ).with_value(json!({
            "diagnostic_observation_ready": diagnostic_observation_ready,
            "observed_rust_row_count": observed_rust_row_count,
            "observed_python_row_count": observed_python_row_count,
            "diagnostic_row_authority": diagnostic_row_authority
        })));
    }

    let cleanup_attempted = bool_value(payload.get("cleanup_attempted"), false)
        || bool_value(pilot_result.get("cleanup_attempted"), false);
    let apply_attempted = bool_value(payload.get("apply_attempted"), false)
        || bool_value(pilot_result.get("apply_attempted"), false);
    let write_attempted = bool_value(payload.get("write_attempted"), false)
        || bool_value(pilot_result.get("write_attempted"), false);
    let authority_switched = bool_value(payload.get("production_collector_authority_switched"), false)
        || bool_value(pilot_result.get("production_collector_authority_switched"), false);
    let side_effect_free = !cleanup_attempted && !apply_attempted && !write_attempted && !authority_switched;

    if require_no_side_effects && !side_effect_free {
        errors.push(Diagnostic::error(
            "collector_authority_promotion_side_effect_detected",
            Some("collector_authority_promotion_readiness".to_string()),
            "Promotion readiness detected cleanup/apply/write/authority side effects, which are forbidden in this release.",
        ));
    }

    let gates_ready = allow_readiness && readiness_pilot && readiness_mode == "rust_collector_authority_promotion_readiness";
    if !gates_ready {
        warnings.push(Diagnostic::warning(
            "collector_authority_promotion_gates_not_enabled",
            Some("rust_core".to_string()),
            "Collector authority promotion readiness gates are not fully enabled; report remains shadow-only.",
        ));
    }

    let ready = errors.is_empty()
        && gates_ready
        && confirmation_ok
        && (!require_pilot_result || pilot_result_pass)
        && (!require_diagnostic_observation || diagnostic_observation_ok)
        && shadow_age <= max_shadow_age
        && side_effect_free
        && require_python_fallback;

    let review = errors.is_empty() && pilot_result_pass && side_effect_free;
    let status = if !errors.is_empty() {
        "blocked"
    } else if ready {
        "collector_authority_promotion_readiness_ready"
    } else if review {
        "collector_authority_promotion_readiness_review"
    } else {
        "collector_authority_promotion_readiness_shadow_only"
    };

    let mut seed = Map::new();
    seed.insert("status".to_string(), json!(status));
    seed.insert("pilot_status".to_string(), json!(pilot_status));
    seed.insert("diagnostic_observation_ok".to_string(), json!(diagnostic_observation_ok));
    seed.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    seed.insert("confirmation_ok".to_string(), json!(confirmation_ok));

    let mut map = Map::new();
    map.insert("mode".to_string(), json!("collector_authority_promotion_readiness"));
    map.insert("status".to_string(), json!(status));
    map.insert("promotion_readiness_id".to_string(), json!(promotion_id(&Value::Object(seed))));
    map.insert("collector_authority".to_string(), json!("python_authoritative"));
    map.insert("full_rust_backend".to_string(), json!(false));
    map.insert("promotion_readiness_only".to_string(), json!(true));
    map.insert("promotion_ready".to_string(), json!(ready));
    map.insert("collector_authority_promotion_supported".to_string(), json!(false));
    map.insert("collector_authority_promotion_executed".to_string(), json!(false));
    map.insert("production_collector_authority_switched".to_string(), json!(false));
    map.insert("rust_can_drive_cleanup".to_string(), json!(false));
    map.insert("rust_can_drive_apply".to_string(), json!(false));
    map.insert("rust_can_write_generated_files".to_string(), json!(false));
    map.insert("safe_for_cleanup".to_string(), json!(false));
    map.insert("write_allowed".to_string(), json!(false));
    map.insert("apply_allowed".to_string(), json!(false));
    map.insert("python_collector_fallback_required".to_string(), json!(true));
    map.insert("manual_confirmation_required".to_string(), json!(require_manual_confirmation));
    map.insert("manual_confirmation_accepted".to_string(), json!(confirmation_ok));
    map.insert("gates_ready".to_string(), json!(gates_ready));
    map.insert("pilot_result_status".to_string(), json!(pilot_status));
    map.insert("pilot_result_pass".to_string(), json!(pilot_result_pass));
    map.insert("require_diagnostic_observation".to_string(), json!(require_diagnostic_observation));
    map.insert("diagnostic_observation_ready".to_string(), json!(diagnostic_observation_ready));
    map.insert("diagnostic_observation_ok".to_string(), json!(diagnostic_observation_ok));
    map.insert("production_row_authority".to_string(), json!(production_row_authority));
    map.insert("cleanup_row_authority".to_string(), json!(cleanup_row_authority));
    map.insert("diagnostic_row_authority".to_string(), json!(diagnostic_row_authority));
    map.insert("observed_rust_row_count".to_string(), json!(observed_rust_row_count));
    map.insert("observed_python_row_count".to_string(), json!(observed_python_row_count));
    map.insert("runtime_evidence_source".to_string(), json!(runtime_evidence_source));
    map.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    map.insert("max_shadow_age_seconds".to_string(), json!(max_shadow_age));
    map.insert("side_effect_free".to_string(), json!(side_effect_free));
    map.insert("connection_attempt_count".to_string(), json!(0));
    map.insert("authentication_attempt_count".to_string(), json!(0));
    map.insert("api_sentence_write_count".to_string(), json!(0));
    map.insert("api_reply_read_count".to_string(), json!(0));
    map.insert("note".to_string(), json!("v4.5 builds a non-mutating promotion readiness report after the pilot result evaluator and requires diagnostics-only Rust observation evidence before readiness."));

    (Value::Object(map), errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_payload() -> Value {
        let mut root = Map::new();
        root.insert("confirmation".to_string(), json!(CONFIRM_PROMOTION_READINESS));
        root.insert("shadow_age_seconds".to_string(), json!(10));

        let mut rust_core = Map::new();
        rust_core.insert("allow_collector_authority_promotion_readiness".to_string(), json!(true));
        rust_core.insert("collector_authority_promotion_readiness_pilot".to_string(), json!(true));
        rust_core.insert("collector_authority_promotion_readiness_mode".to_string(), json!("rust_collector_authority_promotion_readiness"));
        rust_core.insert("collector_authority_promotion_require_pilot_result".to_string(), json!(true));
        rust_core.insert("collector_authority_promotion_require_python_fallback".to_string(), json!(true));
        rust_core.insert("collector_authority_promotion_require_manual_confirmation".to_string(), json!(true));
        rust_core.insert("collector_authority_promotion_require_no_cleanup_apply".to_string(), json!(true));
        rust_core.insert("collector_authority_promotion_require_diagnostic_observation".to_string(), json!(true));
        rust_core.insert("collector_authority_promotion_max_shadow_age_seconds".to_string(), json!(900));
        root.insert("rust_core".to_string(), Value::Object(rust_core));

        let mut pilot = Map::new();
        pilot.insert("status".to_string(), json!("collector_authority_pilot_result_pass"));
        pilot.insert("collector_authority_pilot_result_evaluated".to_string(), json!(true));
        pilot.insert("python_collector_fallback_required".to_string(), json!(true));
        pilot.insert("production_collector_authority_switched".to_string(), json!(false));
        pilot.insert("cleanup_attempted".to_string(), json!(false));
        pilot.insert("apply_attempted".to_string(), json!(false));
        pilot.insert("write_attempted".to_string(), json!(false));
        pilot.insert("diagnostic_observation_ready".to_string(), json!(true));
        pilot.insert("production_row_authority".to_string(), json!("python_collector"));
        pilot.insert("cleanup_row_authority".to_string(), json!("python_collector"));
        pilot.insert("diagnostic_row_authority".to_string(), json!("rust_shadow_diagnostics"));
        pilot.insert("observed_rust_row_count".to_string(), json!(1));
        pilot.insert("observed_python_row_count".to_string(), json!(1));
        pilot.insert("runtime_evidence_source".to_string(), json!("live_read_shadow_history"));
        root.insert("collector_authority_pilot_result_evaluation".to_string(), Value::Object(pilot));

        Value::Object(root)
    }

    #[test]
    fn defaults_to_shadow_only_promotion_readiness() {
        let (result, errors, _warnings) = build_collector_authority_promotion_readiness_payload(&json!({}));
        assert!(errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_promotion_readiness_shadow_only"));
        assert_eq!(result.get("production_collector_authority_switched").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn blocks_execute_attempts() {
        let (result, errors, _warnings) = build_collector_authority_promotion_readiness_payload(&json!({"execute": true}));
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
    }

    #[test]
    fn builds_ready_promotion_readiness_when_pilot_result_and_gates_are_ready() {
        let payload = ready_payload();
        let (result, errors, _warnings) = build_collector_authority_promotion_readiness_payload(&payload);
        assert!(errors.is_empty(), "errors: {errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_promotion_readiness_ready"));
        assert_eq!(result.get("promotion_ready").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("diagnostic_observation_ok").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("diagnostic_row_authority").and_then(Value::as_str), Some("rust_shadow_diagnostics"));
        assert_eq!(result.get("collector_authority_promotion_executed").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("rust_can_drive_cleanup").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn keeps_promotion_under_review_without_diagnostic_observation() {
        let mut payload = ready_payload();
        if let Some(pilot) = payload
            .get_mut("collector_authority_pilot_result_evaluation")
            .and_then(Value::as_object_mut)
        {
            pilot.insert("diagnostic_observation_ready".to_string(), json!(false));
            pilot.insert("observed_rust_row_count".to_string(), json!(0));
        }

        let (result, errors, warnings) = build_collector_authority_promotion_readiness_payload(&payload);
        assert!(errors.is_empty(), "errors: {errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_promotion_readiness_review"));
        assert_eq!(result.get("promotion_ready").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("diagnostic_observation_ok").and_then(Value::as_bool), Some(false));
        assert!(warnings.iter().any(|w| w.code == "collector_authority_promotion_diagnostic_observation_not_ready"));
    }
}
