use crate::collector_authority_promotion::build_collector_authority_promotion_readiness_payload;
use crate::protocol::Diagnostic;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const CONFIRM_PROMOTION_EXECUTION_REHEARSAL: &str = "CONFIRM_COLLECTOR_AUTHORITY_PROMOTION_EXECUTION_REHEARSAL";
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

fn promotion_execution_id(seed: &Value) -> String {
    let text = serde_json::to_string(seed).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("capexec-{}", &digest[..16])
}

/// Build a non-mutating rehearsal contract for future collector-authority promotion execution.
///
/// v4.6 is still a safety bridge: it can say whether the operator has supplied all gates
/// for a promotion execution rehearsal, but it does not promote Rust collectors, does not
/// transfer cleanup/apply authority, and does not write generated LibreQoS files.
pub fn build_collector_authority_promotion_execution_rehearsal_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(str_value(payload.get("mode"), "rehearsal"), "execute" | "switch" | "promote" | "authority" | "apply" | "production");
    if requested_execute {
        errors.push(Diagnostic::error(
            "collector_authority_promotion_execution_not_implemented",
            Some("collector_authority_promotion_execution_rehearsal".to_string()),
            "This release only builds a collector authority promotion execution rehearsal. It does not promote Rust collectors, drive cleanup, write files, or apply LibreQoS.",
        ));
    }

    let allow_rehearsal = bool_value(config_value(payload, "allow_collector_authority_promotion_execution_rehearsal"), false);
    let rehearsal_pilot = bool_value(config_value(payload, "collector_authority_promotion_execution_rehearsal_pilot"), false);
    let rehearsal_mode = str_value(config_value(payload, "collector_authority_promotion_execution_mode"), "rehearsal_only");
    let require_readiness = bool_value(config_value(payload, "collector_authority_promotion_execution_require_readiness"), true);
    let require_python_fallback = bool_value(config_value(payload, "collector_authority_promotion_execution_require_python_fallback"), true);
    let require_manual_confirmation = bool_value(config_value(payload, "collector_authority_promotion_execution_require_manual_confirmation"), true);
    let require_no_side_effects = bool_value(config_value(payload, "collector_authority_promotion_execution_require_no_cleanup_apply"), true);
    let max_shadow_age = number_value(config_value(payload, "collector_authority_promotion_execution_max_shadow_age_seconds"), 900);
    let shadow_age = number_value(payload.get("shadow_age_seconds"), 0);

    let confirmation = str_value(payload.get("confirmation"), "");
    let confirmation_ok = !require_manual_confirmation || confirmation == CONFIRM_PROMOTION_EXECUTION_REHEARSAL;
    if require_manual_confirmation && !confirmation_ok {
        warnings.push(Diagnostic::warning(
            "collector_authority_promotion_execution_confirmation_required",
            Some("confirmation".to_string()),
            "Collector authority promotion execution rehearsal requires CONFIRM_COLLECTOR_AUTHORITY_PROMOTION_EXECUTION_REHEARSAL before it can report ready.",
        ));
    }

    if !require_python_fallback {
        errors.push(Diagnostic::error(
            "collector_authority_promotion_execution_requires_python_fallback",
            Some("rust_core.collector_authority_promotion_execution_require_python_fallback".to_string()),
            "Collector authority promotion execution rehearsal requires Python collector fallback in this release.",
        ));
    }

    if shadow_age > max_shadow_age {
        warnings.push(Diagnostic::warning(
            "collector_authority_promotion_execution_shadow_stale",
            Some("shadow_age_seconds".to_string()),
            "Rust-shadow collector data is older than the configured maximum age; promotion execution rehearsal remains under review.",
        ).with_value(json!({"shadow_age_seconds": shadow_age, "max_shadow_age_seconds": max_shadow_age})));
    }

    let readiness_value = first_object(payload, &[
        "collector_authority_promotion_readiness",
        "promotion_readiness",
        "collector_authority_promotion_readiness_report",
    ]).cloned();

    let (promotion_readiness, readiness_errors, mut readiness_warnings) = match readiness_value {
        Some(v) => (v, Vec::new(), Vec::new()),
        None => {
            let mut nested_payload = payload.clone();
            if let Some(obj) = nested_payload.as_object_mut() {
                let readiness_confirmation = str_value(
                    payload.get("collector_authority_promotion_readiness_confirmation"),
                    CONFIRM_PROMOTION_READINESS,
                );
                obj.insert("confirmation".to_string(), json!(readiness_confirmation));
            }
            build_collector_authority_promotion_readiness_payload(&nested_payload)
        }
    };
    warnings.append(&mut readiness_warnings);

    if !readiness_errors.is_empty() {
        warnings.push(Diagnostic::warning(
            "collector_authority_promotion_execution_readiness_not_clean",
            Some("collector_authority_promotion_readiness".to_string()),
            "Promotion readiness returned errors; promotion execution rehearsal remains shadow-only.",
        ));
    }

    let readiness_status = promotion_readiness.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let readiness_ready = readiness_errors.is_empty()
        && readiness_status == "collector_authority_promotion_readiness_ready"
        && promotion_readiness.get("promotion_ready").and_then(Value::as_bool).unwrap_or(false)
        && promotion_readiness.get("production_collector_authority_switched").and_then(Value::as_bool) == Some(false)
        && promotion_readiness.get("python_collector_fallback_required").and_then(Value::as_bool).unwrap_or(true);

    if require_readiness && !readiness_ready {
        warnings.push(Diagnostic::warning(
            "collector_authority_promotion_execution_readiness_not_ready",
            Some("collector_authority_promotion_readiness".to_string()),
            "Collector authority promotion readiness has not passed; promotion execution rehearsal remains shadow-only or under review.",
        ));
    }

    let cleanup_attempted = bool_value(payload.get("cleanup_attempted"), false)
        || bool_value(promotion_readiness.get("cleanup_attempted"), false);
    let apply_attempted = bool_value(payload.get("apply_attempted"), false)
        || bool_value(promotion_readiness.get("apply_attempted"), false);
    let write_attempted = bool_value(payload.get("write_attempted"), false)
        || bool_value(promotion_readiness.get("write_attempted"), false);
    let authority_switched = bool_value(payload.get("production_collector_authority_switched"), false)
        || bool_value(promotion_readiness.get("production_collector_authority_switched"), false);
    let side_effect_free = !cleanup_attempted && !apply_attempted && !write_attempted && !authority_switched;

    if require_no_side_effects && !side_effect_free {
        errors.push(Diagnostic::error(
            "collector_authority_promotion_execution_side_effect_detected",
            Some("collector_authority_promotion_execution_rehearsal".to_string()),
            "Promotion execution rehearsal detected cleanup/apply/write/authority side effects, which are forbidden in this release.",
        ));
    }

    let gates_ready = allow_rehearsal && rehearsal_pilot && rehearsal_mode == "rehearsal_only";
    if !gates_ready {
        warnings.push(Diagnostic::warning(
            "collector_authority_promotion_execution_gates_not_enabled",
            Some("rust_core".to_string()),
            "Collector authority promotion execution rehearsal gates are not fully enabled; report remains shadow-only.",
        ));
    }

    let ready = errors.is_empty()
        && gates_ready
        && confirmation_ok
        && (!require_readiness || readiness_ready)
        && shadow_age <= max_shadow_age
        && side_effect_free
        && require_python_fallback;

    let review = errors.is_empty() && readiness_ready && side_effect_free;
    let status = if !errors.is_empty() {
        "blocked"
    } else if ready {
        "collector_authority_promotion_execution_rehearsal_ready"
    } else if review {
        "collector_authority_promotion_execution_rehearsal_review"
    } else {
        "collector_authority_promotion_execution_rehearsal_shadow_only"
    };

    let mut seed = Map::new();
    seed.insert("status".to_string(), json!(status));
    seed.insert("readiness_status".to_string(), json!(readiness_status));
    seed.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    seed.insert("confirmation_ok".to_string(), json!(confirmation_ok));

    let mut map = Map::new();
    map.insert("mode".to_string(), json!("collector_authority_promotion_execution_rehearsal"));
    map.insert("status".to_string(), json!(status));
    map.insert("promotion_execution_rehearsal_id".to_string(), json!(promotion_execution_id(&Value::Object(seed))));
    map.insert("collector_authority".to_string(), json!("python_authoritative"));
    map.insert("full_rust_backend".to_string(), json!(false));
    map.insert("promotion_execution_rehearsal_only".to_string(), json!(true));
    map.insert("promotion_execution_rehearsal_ready".to_string(), json!(ready));
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
    map.insert("promotion_readiness_status".to_string(), json!(readiness_status));
    map.insert("promotion_readiness_ready".to_string(), json!(readiness_ready));
    map.insert("shadow_age_seconds".to_string(), json!(shadow_age));
    map.insert("max_shadow_age_seconds".to_string(), json!(max_shadow_age));
    map.insert("side_effect_free".to_string(), json!(side_effect_free));
    map.insert("connection_attempt_count".to_string(), json!(0));
    map.insert("authentication_attempt_count".to_string(), json!(0));
    map.insert("api_sentence_write_count".to_string(), json!(0));
    map.insert("api_reply_read_count".to_string(), json!(0));
    map.insert("note".to_string(), json!("v4.6 builds a non-mutating promotion execution rehearsal after promotion readiness. It does not promote Rust collectors or transfer cleanup/apply authority."));

    (Value::Object(map), errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_payload() -> Value {
        let mut root = Map::new();
        root.insert("confirmation".to_string(), json!(CONFIRM_PROMOTION_EXECUTION_REHEARSAL));
        root.insert("shadow_age_seconds".to_string(), json!(10));

        let mut rust_core = Map::new();
        rust_core.insert("allow_collector_authority_promotion_execution_rehearsal".to_string(), json!(true));
        rust_core.insert("collector_authority_promotion_execution_rehearsal_pilot".to_string(), json!(true));
        rust_core.insert("collector_authority_promotion_execution_mode".to_string(), json!("rehearsal_only"));
        rust_core.insert("collector_authority_promotion_execution_require_readiness".to_string(), json!(true));
        rust_core.insert("collector_authority_promotion_execution_require_python_fallback".to_string(), json!(true));
        rust_core.insert("collector_authority_promotion_execution_require_manual_confirmation".to_string(), json!(true));
        rust_core.insert("collector_authority_promotion_execution_require_no_cleanup_apply".to_string(), json!(true));
        rust_core.insert("collector_authority_promotion_execution_max_shadow_age_seconds".to_string(), json!(900));
        root.insert("rust_core".to_string(), Value::Object(rust_core));

        let mut readiness = Map::new();
        readiness.insert("status".to_string(), json!("collector_authority_promotion_readiness_ready"));
        readiness.insert("promotion_ready".to_string(), json!(true));
        readiness.insert("python_collector_fallback_required".to_string(), json!(true));
        readiness.insert("production_collector_authority_switched".to_string(), json!(false));
        readiness.insert("cleanup_attempted".to_string(), json!(false));
        readiness.insert("apply_attempted".to_string(), json!(false));
        readiness.insert("write_attempted".to_string(), json!(false));
        root.insert("collector_authority_promotion_readiness".to_string(), Value::Object(readiness));

        Value::Object(root)
    }

    #[test]
    fn defaults_to_shadow_only_promotion_execution_rehearsal() {
        let (result, errors, _warnings) = build_collector_authority_promotion_execution_rehearsal_payload(&json!({}));
        assert!(errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_promotion_execution_rehearsal_shadow_only"));
        assert_eq!(result.get("production_collector_authority_switched").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn blocks_execute_attempts() {
        let (result, errors, _warnings) = build_collector_authority_promotion_execution_rehearsal_payload(&json!({"execute": true}));
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
    }

    #[test]
    fn builds_ready_promotion_execution_rehearsal_when_readiness_and_gates_are_ready() {
        let payload = ready_payload();
        let (result, errors, _warnings) = build_collector_authority_promotion_execution_rehearsal_payload(&payload);
        assert!(errors.is_empty(), "errors: {errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_promotion_execution_rehearsal_ready"));
        assert_eq!(result.get("promotion_execution_rehearsal_ready").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("collector_authority_promotion_executed").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("rust_can_drive_cleanup").and_then(Value::as_bool), Some(false));
    }
}
