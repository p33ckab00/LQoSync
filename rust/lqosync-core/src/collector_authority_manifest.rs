use crate::collector_authority_pilot::evaluate_rust_collector_authority_pilot_payload;
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

fn source_path(source: &str) -> &'static str {
    match source {
        "pppoe" => "/ppp/active",
        "dhcp" => "/ip/dhcp-server/lease",
        "hotspot" => "/ip/hotspot/active",
        _ => "/ppp/active",
    }
}

fn sources_from_payload(payload: &Value) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(values) = payload.get("sources").and_then(Value::as_array) {
        for value in values {
            if let Some(s) = value.as_str() {
                if !s.trim().is_empty() && !out.iter().any(|x| x == s.trim()) {
                    out.push(s.trim().to_string());
                }
            } else if let Some(obj) = value.as_object() {
                if let Some(s) = obj.get("source").and_then(Value::as_str) {
                    if !s.trim().is_empty() && !out.iter().any(|x| x == s.trim()) {
                        out.push(s.trim().to_string());
                    }
                }
            }
        }
    }
    if out.is_empty() {
        if let Some(s) = payload.get("source").and_then(Value::as_str) {
            if !s.trim().is_empty() {
                out.push(s.trim().to_string());
            }
        }
    }
    if out.is_empty() {
        out.push("pppoe".to_string());
    }
    out
}

fn manifest_id(value: &Value) -> String {
    let text = serde_json::to_string(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("cam-{}", &digest[..16])
}

/// Build a non-mutating collector authority decision manifest.
///
/// v3.6 is still not a live authority switch. It only converts source-level
/// Rust collector authority pilot gate results into an explicit, auditable
/// decision manifest that Python can display before any future migration.
pub fn build_collector_authority_manifest_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || matches!(str_value(payload.get("mode"), "manifest"), "execute" | "promote" | "switch" | "authority");
    if requested_execute {
        errors.push(Diagnostic::error(
            "collector_authority_manifest_execute_not_implemented",
            Some("collector_authority_manifest".to_string()),
            "This release only builds the Rust collector authority decision manifest. It does not switch collector authority away from Python.",
        ));
    }

    let router = payload
        .get("router")
        .and_then(|v| v.get("name"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("router").and_then(Value::as_str))
        .unwrap_or("unknown");
    let authority_mode = config_value(payload, "collector_authority_mode")
        .and_then(Value::as_str)
        .unwrap_or("python_authoritative");
    let threshold = config_value(payload, "collector_authority_require_parity_score")
        .and_then(Value::as_f64)
        .unwrap_or(99.99);

    let sources = sources_from_payload(payload);
    let mut decisions: Vec<Value> = Vec::new();
    let mut ready_count = 0usize;
    let mut shadow_count = 0usize;
    let mut blocked_count = 0usize;

    for source in sources.iter() {
        let path = payload
            .get("paths")
            .and_then(|v| v.get(source))
            .and_then(Value::as_str)
            .unwrap_or_else(|| source_path(source));

        let mut gate_payload = payload.clone();
        if let Value::Object(ref mut map) = gate_payload {
            map.insert("source".to_string(), json!(source));
            map.insert("path".to_string(), json!(path));
            map.insert("execute".to_string(), json!(false));
        }
        let (gate_result, gate_errors, mut gate_warnings) = evaluate_rust_collector_authority_pilot_payload(&gate_payload);
        warnings.append(&mut gate_warnings);

        let gates_ready = gate_result.get("gates_ready").and_then(Value::as_bool).unwrap_or(false);
        let parity = gate_result.get("parity").cloned().unwrap_or_else(|| json!({"score":0.0,"verdict":"not_available","ok":false}));
        let decision = if !gate_errors.is_empty() {
            blocked_count += 1;
            "blocked"
        } else if gates_ready {
            ready_count += 1;
            "rust_pilot_ready"
        } else {
            shadow_count += 1;
            "python_authoritative_shadow"
        };

        decisions.push(json!({
            "source": source,
            "router": router,
            "path": path,
            "decision": decision,
            "current_authority": "python",
            "proposed_authority": if gates_ready { "rust_collector_pilot" } else { "python" },
            "gates_ready": gates_ready,
            "pilot_status": gate_result.get("status").cloned().unwrap_or_else(|| json!("unknown")),
            "parity": parity,
            "safe_for_cleanup": false,
            "write_allowed": false,
            "apply_allowed": false,
            "fallback": "python_collector",
            "live_adapter_contract_ready": gate_result.get("live_adapter_contract_ready").cloned().unwrap_or_else(|| json!(false)),
            "gate_errors": gate_errors.iter().map(|d| d.code.clone()).collect::<Vec<_>>()
        }));
    }

    let status = if !errors.is_empty() {
        "blocked"
    } else if ready_count > 0 && shadow_count == 0 && blocked_count == 0 {
        "collector_authority_manifest_ready"
    } else if ready_count > 0 {
        "collector_authority_manifest_partial"
    } else {
        "collector_authority_manifest_shadow_only"
    };

    let summary_seed = json!({
        "router": router,
        "authority_mode": authority_mode,
        "sources": sources,
        "ready_count": ready_count,
        "shadow_count": shadow_count,
        "blocked_count": blocked_count,
        "threshold": threshold,
        "status": status,
    });

    let result = json!({
        "mode": "collector_authority_decision_manifest",
        "status": status,
        "manifest_id": manifest_id(&summary_seed),
        "router": router,
        "authority_mode": authority_mode,
        "required_authority_mode": "rust_collector_authority_pilot",
        "collector_authority": "python_authoritative",
        "future_collector_authority": if ready_count > 0 { "rust_pilot_candidates" } else { "not_eligible" },
        "source_count": decisions.len(),
        "ready_count": ready_count,
        "shadow_count": shadow_count,
        "blocked_count": blocked_count,
        "parity_threshold": threshold,
        "decisions": decisions,
        "full_rust_backend": false,
        "collector_authority_switch_supported": false,
        "python_collector_fallback_required": true,
        "connection_attempt_count": 0,
        "authentication_attempt_count": 0,
        "api_sentence_write_count": 0,
        "api_reply_read_count": 0,
        "safe_for_cleanup": false,
        "write_allowed": false,
        "apply_allowed": false,
        "next_stage": "rust_collector_authority_dry_run_shadow_integration",
        "note": "v3.6 builds an auditable collector authority decision manifest only. It does not perform live reads, switch authority, or write LibreQoS files."
    });

    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn defaults_to_shadow_manifest() {
        let leaked_password = "shadow-manifest-password-value";
        let payload = json!({
            "router": {"name":"R1", "address":"10.0.0.1", "username":"admin", "password": leaked_password},
            "sources": ["pppoe"],
            "collector_parity": {"parity_score":100.0, "verdict":"parity_pass"}
        });
        let (result, errors, _warnings) = build_collector_authority_manifest_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_manifest_shadow_only"));
        assert_eq!(result.get("collector_authority").and_then(Value::as_str), Some("python_authoritative"));
        assert_eq!(result.get("ready_count").and_then(Value::as_u64), Some(0));
        let text = serde_json::to_string(&result).unwrap();
        assert!(!text.contains(leaked_password));
        assert!(!text.contains("\"password\":"));
    }

    #[test]
    fn builds_ready_manifest_when_gate_ready() {
        let payload = json!({
            "router": {"name":"R1", "address":"10.0.0.1", "username":"admin", "password":"redacted-by-contract"},
            "sources": ["pppoe"],
            "collector_parity": {"parity_score":100.0, "verdict":"parity_pass"},
            "rust_core": {
                "allow_rust_collector_authority": true,
                "rust_collector_authority_pilot": true,
                "allow_rust_routeros_live_read_adapter": true,
                "routeros_live_read_adapter_pilot": true,
                "rust_collector_authority_sources": ["pppoe"],
                "collector_authority_mode": "rust_collector_authority_pilot"
            }
        });
        let (result, errors, _warnings) = build_collector_authority_manifest_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("collector_authority_manifest_ready"));
        assert_eq!(result.get("ready_count").and_then(Value::as_u64), Some(1));
        assert_eq!(result.get("collector_authority_switch_supported").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn blocks_execute_attempts() {
        let payload = json!({"source":"pppoe", "execute": true});
        let (result, errors, _warnings) = build_collector_authority_manifest_payload(&payload);
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
        assert!(errors.iter().any(|e| e.code == "collector_authority_manifest_execute_not_implemented"));
    }
}
