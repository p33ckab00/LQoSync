use crate::collector_authority_manifest::build_collector_authority_manifest_payload;
use crate::protocol::Diagnostic;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn bool_value(v: Option<&Value>, default: bool) -> bool {
    v.and_then(Value::as_bool).unwrap_or(default)
}

fn str_value<'a>(v: Option<&'a Value>, default: &'a str) -> &'a str {
    v.and_then(Value::as_str).unwrap_or(default)
}

fn config_value<'a>(payload: &'a Value, key: &str) -> Option<&'a Value> {
    payload
        .get("rust_core")
        .and_then(|v| v.get(key))
        .or_else(|| payload.get("config").and_then(|c| c.get("rust_core")).and_then(|v| v.get(key)))
}

fn selection_id(value: &Value) -> String {
    let text = serde_json::to_string(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("cas-{}", &digest[..16])
}

/// Build a dry-run collector authority selection from the v3.6 decision manifest.
///
/// v3.7 still does not switch production authority. It only tells Python which
/// sources would be eligible for Rust-shadow collector use during dry-run/pilot
/// views while production collection, cleanup eligibility, and apply decisions
/// remain Python-authoritative.
pub fn build_collector_authority_selection_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(str_value(payload.get("mode"), "dry_run"), "execute" | "promote" | "switch" | "authority");
    if requested_execute {
        errors.push(Diagnostic::error(
            "collector_authority_selection_execute_not_implemented",
            Some("collector_authority_selection".to_string()),
            "This release only builds a dry-run collector authority selection. It does not switch production authority away from Python.",
        ));
    }

    let allow_dry_run_selection = bool_value(config_value(payload, "allow_collector_authority_dry_run_selection"), false);
    let dry_run_selection_pilot = bool_value(config_value(payload, "collector_authority_dry_run_selection_pilot"), false);
    let authority_mode = config_value(payload, "collector_authority_mode")
        .and_then(Value::as_str)
        .unwrap_or("python_authoritative");
    let requested_mode = str_value(payload.get("mode"), "dry_run");

    let manifest = payload
        .get("collector_authority_manifest")
        .and_then(|v| v.get("result"))
        .or_else(|| payload.get("collector_authority_manifest"))
        .cloned();

    let (manifest_result, manifest_errors, mut manifest_warnings) = match manifest {
        Some(value) if value.is_object() => (value, Vec::new(), Vec::new()),
        _ => build_collector_authority_manifest_payload(payload),
    };
    warnings.append(&mut manifest_warnings);

    if !manifest_errors.is_empty() {
        warnings.push(Diagnostic::warning(
            "collector_authority_manifest_not_clean",
            Some("collector_authority_manifest".to_string()),
            "The collector authority manifest returned errors, so dry-run selection remains Python-only.",
        ));
    }

    let manifest_ready = manifest_errors.is_empty()
        && matches!(
            manifest_result.get("status").and_then(Value::as_str).unwrap_or(""),
            "collector_authority_manifest_ready" | "collector_authority_manifest_partial"
        );

    let decisions = manifest_result
        .get("decisions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut selections: Vec<Value> = Vec::new();
    let mut rust_shadow_count = 0usize;
    let mut python_count = 0usize;
    let mut blocked_count = 0usize;

    for decision in decisions.iter() {
        let source = decision.get("source").and_then(Value::as_str).unwrap_or("unknown");
        let path = decision.get("path").and_then(Value::as_str).unwrap_or("");
        let decision_value = decision.get("decision").and_then(Value::as_str).unwrap_or("python_authoritative_shadow");
        let gates_ready = decision.get("gates_ready").and_then(Value::as_bool).unwrap_or(false);
        let eligible = manifest_ready
            && gates_ready
            && decision_value == "rust_pilot_ready"
            && allow_dry_run_selection
            && dry_run_selection_pilot
            && authority_mode == "rust_collector_authority_pilot";

        let selected_for_dry_run = if eligible { "rust_shadow_collector" } else { "python_collector" };
        if eligible {
            rust_shadow_count += 1;
        } else if decision_value == "blocked" {
            blocked_count += 1;
            python_count += 1;
        } else {
            python_count += 1;
        }

        selections.push(json!({
            "source": source,
            "path": path,
            "manifest_decision": decision_value,
            "selected_for_dry_run": selected_for_dry_run,
            "production_authority": "python_collector",
            "cleanup_authority": "python_policy",
            "apply_authority": "python_orchestrator",
            "rust_shadow_selected": eligible,
            "gates_ready": gates_ready,
            "collector_output_can_drive_cleanup": false,
            "collector_output_can_drive_apply": false,
            "requires_python_fallback": true,
            "fallback": "python_collector",
            "reason": if eligible {
                "source is eligible for Rust-shadow collector dry-run comparison only"
            } else {
                "Python remains selected because Rust dry-run selection gates are not fully enabled or source is not manifest-ready"
            }
        }));
    }

    if selections.is_empty() {
        warnings.push(Diagnostic::warning(
            "collector_authority_selection_empty",
            Some("collector_authority_selection".to_string()),
            "No collector authority decisions were available; Python collectors remain selected.",
        ));
    }

    let status = if !errors.is_empty() {
        "blocked"
    } else if rust_shadow_count > 0 && blocked_count == 0 {
        "collector_authority_dry_run_selection_ready"
    } else if rust_shadow_count > 0 {
        "collector_authority_dry_run_selection_partial"
    } else {
        "collector_authority_dry_run_selection_python_only"
    };

    let seed = json!({
        "status": status,
        "authority_mode": authority_mode,
        "requested_mode": requested_mode,
        "rust_shadow_count": rust_shadow_count,
        "python_count": python_count,
        "blocked_count": blocked_count,
        "manifest_id": manifest_result.get("manifest_id"),
        "selections": selections,
    });

    let result = json!({
        "mode": "collector_authority_dry_run_selection",
        "status": status,
        "selection_id": selection_id(&seed),
        "manifest_id": manifest_result.get("manifest_id").cloned().unwrap_or_else(|| json!(null)),
        "manifest_status": manifest_result.get("status").cloned().unwrap_or_else(|| json!("unknown")),
        "authority_mode": authority_mode,
        "requested_mode": requested_mode,
        "collector_authority": "python_authoritative",
        "production_authority": "python_collector",
        "dry_run_authority": if rust_shadow_count > 0 { "rust_shadow_candidate" } else { "python_collector" },
        "selection_count": selections.len(),
        "rust_shadow_count": rust_shadow_count,
        "python_count": python_count,
        "blocked_count": blocked_count,
        "allow_dry_run_selection": allow_dry_run_selection,
        "dry_run_selection_pilot": dry_run_selection_pilot,
        "selections": selections,
        "full_rust_backend": false,
        "collector_authority_switch_supported": false,
        "collector_output_can_drive_cleanup": false,
        "collector_output_can_drive_apply": false,
        "python_collector_fallback_required": true,
        "connection_attempt_count": 0,
        "authentication_attempt_count": 0,
        "api_sentence_write_count": 0,
        "api_reply_read_count": 0,
        "safe_for_cleanup": false,
        "write_allowed": false,
        "apply_allowed": false,
        "next_stage": "rust_collector_authority_dry_run_in_run_cycle",
        "note": "v3.7 selects Rust-shadow collectors only for dry-run comparison. It does not switch production collector authority."
    });

    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn defaults_to_python_selection() {
        let leaked_password = "dry-run-selection-password-value";
        let payload = json!({
            "router": {"name":"R1", "address":"10.0.0.1", "username":"admin", "password": leaked_password},
            "sources": ["pppoe"],
            "collector_parity": {"parity_score": 100.0, "verdict":"parity_pass"}
        });
        let (result, errors, _warnings) = build_collector_authority_selection_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_dry_run_selection_python_only"));
        assert_eq!(result.get("collector_authority").and_then(Value::as_str), Some("python_authoritative"));
        assert_eq!(result.get("rust_shadow_count").and_then(Value::as_u64), Some(0));
        let text = serde_json::to_string(&result).unwrap();
        assert!(!text.contains(leaked_password));
        assert!(!text.contains("\"password\":"));
    }

    #[test]
    fn selects_rust_shadow_when_manifest_and_dry_run_gates_are_ready() {
        let payload = json!({
            "router": {"name":"R1", "address":"10.0.0.1", "username":"admin", "password":"redacted-by-contract"},
            "sources": ["pppoe"],
            "collector_parity": {"parity_score": 100.0, "verdict":"parity_pass"},
            "rust_core": {
                "allow_rust_collector_authority": true,
                "rust_collector_authority_pilot": true,
                "allow_rust_routeros_live_read_adapter": true,
                "routeros_live_read_adapter_pilot": true,
                "rust_collector_authority_sources": ["pppoe"],
                "collector_authority_mode": "rust_collector_authority_pilot",
                "collector_authority_manifest_pilot": true,
                "allow_collector_authority_manifest": true,
                "collector_authority_dry_run_selection_pilot": true,
                "allow_collector_authority_dry_run_selection": true
            }
        });
        let (result, errors, _warnings) = build_collector_authority_selection_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_dry_run_selection_ready"));
        assert_eq!(result.get("rust_shadow_count").and_then(Value::as_u64), Some(1));
        assert_eq!(result.get("production_authority").and_then(Value::as_str), Some("python_collector"));
        assert_eq!(result.get("safe_for_cleanup").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn blocks_execute_attempts() {
        let payload = json!({"execute": true, "sources": ["pppoe"]});
        let (result, errors, _warnings) = build_collector_authority_selection_payload(&payload);
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
        assert_eq!(result.get("collector_authority_switch_supported").and_then(Value::as_bool), Some(false));
    }
}
