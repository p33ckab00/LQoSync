use crate::protocol::Diagnostic;
use crate::routeros_api_codec::build_routeros_api_sentence_payload;
use crate::routeros_auth_session::build_routeros_auth_session_contract_payload;
use crate::routeros_tcp_probe::run_routeros_tcp_connectivity_pilot_payload;
use serde_json::{json, Value};

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

fn merge_diags(target: &mut Vec<Diagnostic>, mut source: Vec<Diagnostic>) {
    target.append(&mut source);
}

fn live_requested(payload: &Value) -> bool {
    matches!(str_value(payload.get("adapter"), "contract"), "live" | "tcp" | "routeros")
        || matches!(str_value(payload.get("mode"), "contract"), "live" | "live_read" | "execute_live" | "authenticated_live_read")
}

fn router_name_from_payload(payload: &Value) -> String {
    payload
        .get("router")
        .and_then(|v| v.get("name"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("router").and_then(Value::as_str))
        .unwrap_or("unknown")
        .to_string()
}

fn source_from_path(path: &str) -> &'static str {
    if path.starts_with("/ppp/") {
        "pppoe"
    } else if path.starts_with("/ip/dhcp-server") {
        "dhcp"
    } else if path.starts_with("/ip/hotspot") {
        "hotspot"
    } else {
        "unknown"
    }
}

/// Build the guarded live RouterOS read adapter contract.
///
/// v3.4 intentionally does not perform live RouterOS API reads. It composes the
/// TCP pilot contract, auth-session contract, and API sentence encoder to prove
/// the state machine that a future live adapter must satisfy. Any request that
/// attempts to use the live adapter is blocked with a deterministic diagnostic.
pub fn run_routeros_live_read_adapter_pilot_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let execute = bool_value(payload.get("execute"), false);
    let adapter = str_value(payload.get("adapter"), "contract");
    let mode = str_value(payload.get("mode"), "contract");
    let path = str_value(payload.get("path"), "/ppp/active");
    let router_name = router_name_from_payload(payload);
    let source = str_value(payload.get("source"), source_from_path(path));

    let allow_live_reads = bool_value(config_value(payload, "allow_rust_routeros_live_reads"), false);
    let allow_credentials = bool_value(config_value(payload, "allow_rust_routeros_credentials"), false);
    let allow_tcp = bool_value(config_value(payload, "allow_rust_routeros_tcp_connect"), false);
    let live_read_pilot = bool_value(config_value(payload, "routeros_live_read_pilot"), false);
    let live_adapter_pilot = bool_value(config_value(payload, "routeros_live_read_adapter_pilot"), false);
    let authority = config_value(payload, "routeros_transport_authority")
        .and_then(Value::as_str)
        .unwrap_or("plan_only");

    let (tcp_contract, tcp_errors, tcp_warnings) = run_routeros_tcp_connectivity_pilot_payload(payload);
    merge_diags(&mut errors, tcp_errors);
    merge_diags(&mut warnings, tcp_warnings);

    let mut auth_payload = payload.clone();
    if let Value::Object(ref mut map) = auth_payload {
        map.insert("adapter".to_string(), json!("fixture"));
        map.insert("mode".to_string(), json!("contract"));
        map.entry("fixture_reply_words".to_string()).or_insert_with(|| json!(["!done"]));
    }
    let (auth_session, auth_errors, auth_warnings) = build_routeros_auth_session_contract_payload(&auth_payload);
    merge_diags(&mut errors, auth_errors);
    merge_diags(&mut warnings, auth_warnings);

    let sentence_payload = json!({
        "path": path,
        "fields": payload.get("fields").cloned().unwrap_or_else(|| json!(["name", "address"])),
        "execute": false,
        "mode": "encode"
    });
    let (api_sentence, sentence_errors, sentence_warnings) = build_routeros_api_sentence_payload(&sentence_payload);
    merge_diags(&mut errors, sentence_errors);
    merge_diags(&mut warnings, sentence_warnings);

    let authenticated = auth_session
        .get("authenticated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && auth_session.get("status").and_then(Value::as_str) == Some("auth_session_contract_ready");

    if execute || live_requested(payload) {
        errors.push(Diagnostic::error(
            "routeros_live_read_adapter_not_implemented",
            Some("adapter".to_string()),
            "The Rust RouterOS live read adapter is not implemented yet; this phase only builds the guarded adapter contract.",
        ));
    }

    let gates_ready = allow_live_reads
        && allow_credentials
        && allow_tcp
        && live_read_pilot
        && live_adapter_pilot
        && authority == "live_read_adapter_pilot";

    let status = if !errors.is_empty() {
        "blocked"
    } else if authenticated {
        "live_read_adapter_contract_ready"
    } else {
        "live_read_adapter_contract_not_authenticated"
    };

    let result = json!({
        "mode": "routeros_live_read_adapter_pilot",
        "status": status,
        "adapter": adapter,
        "requested_mode": mode,
        "authority": authority,
        "authority_required": "live_read_adapter_pilot",
        "full_rust_backend": false,
        "live_transport_supported": false,
        "live_adapter_implemented": false,
        "execute_requested": execute,
        "gates_ready_for_future_live_read": gates_ready,
        "router": router_name,
        "source": source,
        "path": path,
        "tcp_contract": tcp_contract,
        "auth_session": auth_session,
        "api_sentence": api_sentence,
        "authenticated": authenticated,
        "credential_material": "redacted",
        "username_emitted": false,
        "password_emitted": false,
        "session_token_emitted": false,
        "connection_attempt_count": 0,
        "authentication_attempt_count": 0,
        "api_sentence_write_count": 0,
        "api_reply_read_count": 0,
        "safe_for_cleanup": false,
        "collector_authority": "python_authoritative",
        "next_stage": "rust_routeros_live_read_socket_adapter",
        "note": "v3.4 builds the guarded live-read adapter contract only. It does not open sockets, authenticate, read RouterOS data, or replace Python collectors."
    });

    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_live_read_adapter_contract_without_network() {
        let payload = json!({
            "router": {"name":"R1", "address":"10.0.0.1", "port":8728, "username":"admin", "password":"super-secret"},
            "adapter": "contract",
            "mode": "contract",
            "execute": false,
            "path": "/ppp/active",
            "fields": ["name", "address"],
            "fixture_reply_words": ["!done"]
        });
        let (result, errors, _warnings) = run_routeros_live_read_adapter_pilot_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("live_read_adapter_contract_ready"));
        assert_eq!(result.get("connection_attempt_count").and_then(Value::as_u64), Some(0));
        assert_eq!(result.get("authentication_attempt_count").and_then(Value::as_u64), Some(0));
        assert_eq!(result.get("api_sentence_write_count").and_then(Value::as_u64), Some(0));
        let text = serde_json::to_string(&result).unwrap();
        assert!(!text.contains("super-secret"));
    }

    #[test]
    fn blocks_live_read_adapter_execution_even_when_gates_ready() {
        let payload = json!({
            "router": {"name":"R1", "address":"10.0.0.1", "port":8728, "username":"admin", "password":"super-secret"},
            "adapter": "live",
            "mode": "live_read",
            "execute": true,
            "path": "/ppp/active",
            "fixture_reply_words": ["!done"],
            "rust_core": {
                "allow_rust_routeros_live_reads": true,
                "allow_rust_routeros_credentials": true,
                "allow_rust_routeros_tcp_connect": true,
                "routeros_live_read_pilot": true,
                "routeros_live_read_adapter_pilot": true,
                "routeros_transport_authority": "live_read_adapter_pilot"
            }
        });
        let (result, errors, _warnings) = run_routeros_live_read_adapter_pilot_payload(&payload);
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
        assert!(errors.iter().any(|e| e.code == "routeros_live_read_adapter_not_implemented"));
        assert_eq!(result.get("connection_attempt_count").and_then(Value::as_u64), Some(0));
    }
}
