use crate::protocol::Diagnostic;
use crate::routeros_api_codec::build_routeros_api_sentence_payload;
use crate::routeros_auth_session::build_routeros_auth_session_contract_payload;
use crate::routeros_results::validate_routeros_read_results_payload;
use crate::routeros_tcp_probe::run_routeros_tcp_connectivity_pilot_payload;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

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

fn router_value(payload: &Value) -> Value {
    if let Some(router) = payload.get("router").filter(|v| v.is_object()) {
        return router.clone();
    }
    let requested = payload.get("router").and_then(Value::as_str).unwrap_or("");
    if let Some(routers) = payload.get("config").and_then(|c| c.get("routers")).and_then(Value::as_array) {
        for router in routers {
            if !router.get("enabled").and_then(Value::as_bool).unwrap_or(true) {
                continue;
            }
            if requested.is_empty() || router.get("name").and_then(Value::as_str).unwrap_or("") == requested {
                return router.clone();
            }
        }
    }
    json!({})
}

fn u16_value(v: Option<&Value>, default: u16) -> u16 {
    v.and_then(Value::as_u64).and_then(|n| u16::try_from(n).ok()).unwrap_or(default)
}

fn timeout_seconds(payload: &Value) -> u64 {
    payload
        .get("timeout_seconds")
        .and_then(Value::as_u64)
        .or_else(|| config_value(payload, "routeros_live_read_timeout_seconds").and_then(Value::as_u64))
        .unwrap_or(5)
        .clamp(1, 30)
}

fn sensitive_key(key: &str) -> bool {
    let lowered = key.to_ascii_lowercase();
    lowered.contains("password") || lowered.contains("secret") || lowered.contains("token") || lowered.contains("key")
}

fn merge_diags(target: &mut Vec<Diagnostic>, mut source: Vec<Diagnostic>) {
    target.append(&mut source);
}

fn encode_length(len: usize) -> Vec<u8> {
    if len < 0x80 {
        vec![len as u8]
    } else if len < 0x4000 {
        vec![((len >> 8) as u8) | 0x80, (len & 0xff) as u8]
    } else if len < 0x20_0000 {
        vec![((len >> 16) as u8) | 0xC0, ((len >> 8) & 0xff) as u8, (len & 0xff) as u8]
    } else if len < 0x1000_0000 {
        vec![((len >> 24) as u8) | 0xE0, ((len >> 16) & 0xff) as u8, ((len >> 8) & 0xff) as u8, (len & 0xff) as u8]
    } else {
        vec![0xF0, ((len >> 24) & 0xff) as u8, ((len >> 16) & 0xff) as u8, ((len >> 8) & 0xff) as u8, (len & 0xff) as u8]
    }
}

fn decode_length<R: Read>(reader: &mut R) -> std::io::Result<usize> {
    let mut first = [0u8; 1];
    reader.read_exact(&mut first)?;
    let b = first[0];
    if (b & 0x80) == 0 {
        Ok(b as usize)
    } else if (b & 0xC0) == 0x80 {
        let mut rest = [0u8; 1];
        reader.read_exact(&mut rest)?;
        Ok((((b & !0xC0) as usize) << 8) | rest[0] as usize)
    } else if (b & 0xE0) == 0xC0 {
        let mut rest = [0u8; 2];
        reader.read_exact(&mut rest)?;
        Ok((((b & !0xE0) as usize) << 16) | ((rest[0] as usize) << 8) | rest[1] as usize)
    } else if (b & 0xF0) == 0xE0 {
        let mut rest = [0u8; 3];
        reader.read_exact(&mut rest)?;
        Ok((((b & !0xF0) as usize) << 24) | ((rest[0] as usize) << 16) | ((rest[1] as usize) << 8) | rest[2] as usize)
    } else {
        let mut rest = [0u8; 4];
        reader.read_exact(&mut rest)?;
        Ok(((rest[0] as usize) << 24) | ((rest[1] as usize) << 16) | ((rest[2] as usize) << 8) | rest[3] as usize)
    }
}

fn write_sentence(stream: &mut TcpStream, words: &[String]) -> std::io::Result<()> {
    for word in words {
        let bytes = word.as_bytes();
        stream.write_all(&encode_length(bytes.len()))?;
        stream.write_all(bytes)?;
    }
    stream.write_all(&[0])?;
    stream.flush()
}

fn read_sentence(stream: &mut TcpStream) -> std::io::Result<Vec<String>> {
    let mut words = Vec::new();
    loop {
        let len = decode_length(stream)?;
        if len == 0 {
            break;
        }
        if len > 1024 * 1024 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "RouterOS API word is too large"));
        }
        let mut buf = vec![0u8; len];
        stream.read_exact(&mut buf)?;
        let word = String::from_utf8(buf)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "RouterOS API word was not UTF-8"))?;
        words.push(word);
    }
    Ok(words)
}

fn parse_attr(word: &str) -> Option<(String, String)> {
    if !word.starts_with('=') {
        return None;
    }
    let rest = &word[1..];
    let idx = rest.find('=')?;
    let key = rest[..idx].trim();
    if key.is_empty() {
        return None;
    }
    Some((key.to_string(), rest[idx + 1..].to_string()))
}

fn read_until_done(stream: &mut TcpStream) -> std::io::Result<(Vec<Value>, Vec<Value>, usize)> {
    let mut rows = Vec::new();
    let mut traps = Vec::new();
    let mut sentence_count = 0usize;
    loop {
        let words = read_sentence(stream)?;
        if words.is_empty() {
            continue;
        }
        sentence_count += 1;
        let marker = words[0].as_str();
        let mut fields = serde_json::Map::new();
        for word in words.iter().skip(1) {
            if let Some((key, value)) = parse_attr(word) {
                if !sensitive_key(&key) {
                    fields.insert(key, Value::String(value));
                }
            }
        }
        match marker {
            "!re" => rows.push(Value::Object(fields)),
            "!trap" | "!fatal" => traps.push(Value::Object(fields)),
            "!done" => break,
            _ => {}
        }
    }
    Ok((rows, traps, sentence_count))
}

fn live_read(payload: &Value, sentence_words: &[String], router_name: &str, source: &str, path: &str) -> Result<Value, String> {
    let router = router_value(payload);
    let address = payload
        .get("address")
        .and_then(Value::as_str)
        .or_else(|| router.get("address").and_then(Value::as_str))
        .unwrap_or("")
        .trim()
        .to_string();
    let username = router.get("username").and_then(Value::as_str).unwrap_or("").to_string();
    let password = router.get("password").and_then(Value::as_str).unwrap_or("").to_string();
    let port = u16_value(payload.get("port").or_else(|| router.get("port")), 8728);
    if address.is_empty() {
        return Err("Router address is required for Rust RouterOS live read adapter.".to_string());
    }
    if username.is_empty() || password.is_empty() {
        return Err("Router username and password are required for Rust RouterOS live read adapter.".to_string());
    }
    if sentence_words.is_empty() {
        return Err("RouterOS live read adapter requires encoded API sentence words.".to_string());
    }
    let timeout = Duration::from_secs(timeout_seconds(payload));
    let target = format!("{address}:{port}");
    let mut addrs = target.to_socket_addrs().map_err(|e| format!("Router address resolution failed: {e}"))?;
    let sock_addr = addrs.next().ok_or_else(|| "Router address did not resolve to a socket address.".to_string())?;
    let start = Instant::now();
    let mut stream = TcpStream::connect_timeout(&sock_addr, timeout).map_err(|e| format!("RouterOS TCP connect failed: {e}"))?;
    stream.set_read_timeout(Some(timeout)).map_err(|e| format!("Failed setting RouterOS read timeout: {e}"))?;
    stream.set_write_timeout(Some(timeout)).map_err(|e| format!("Failed setting RouterOS write timeout: {e}"))?;

    let login_words = vec!["/login".to_string(), format!("=name={username}"), format!("=password={password}")];
    write_sentence(&mut stream, &login_words).map_err(|e| format!("RouterOS login write failed: {e}"))?;
    let (_login_rows, login_traps, login_sentence_count) = read_until_done(&mut stream).map_err(|e| format!("RouterOS login read failed: {e}"))?;
    if !login_traps.is_empty() {
        let trap_count = login_traps.len();
        return Ok(json!({
            "status": "auth_trap",
            "rows": [],
            "traps": login_traps,
            "row_count": 0,
            "trap_count": trap_count,
            "login_sentence_count": login_sentence_count,
            "read_sentence_count": 0,
            "elapsed_ms": start.elapsed().as_secs_f64() * 1000.0
        }));
    }

    write_sentence(&mut stream, sentence_words).map_err(|e| format!("RouterOS read sentence write failed: {e}"))?;
    let (rows, traps, read_sentence_count) = read_until_done(&mut stream).map_err(|e| format!("RouterOS read reply failed: {e}"))?;
    let status = if traps.is_empty() { "ok" } else { "trap" };
    let row_count = rows.len();
    let trap_count = traps.len();
    Ok(json!({
        "router": router_name,
        "source": source,
        "path": path,
        "status": status,
        "rows": rows,
        "traps": traps,
        "row_count": row_count,
        "trap_count": trap_count,
        "login_sentence_count": login_sentence_count,
        "read_sentence_count": read_sentence_count,
        "elapsed_ms": start.elapsed().as_secs_f64() * 1000.0,
        "adapter": "live_read_only",
        "connection_attempted": true,
        "credential_material": "redacted"
    }))
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

/// Build or execute the guarded live RouterOS read adapter pilot.
///
/// Contract mode performs no network I/O. Live mode is allowed to open one
/// RouterOS API connection and run one read-only `print` command only when every
/// live-read gate is enabled. Python collectors remain authoritative.
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
    let allow_live_adapter = bool_value(config_value(payload, "allow_rust_routeros_live_read_adapter"), false);
    let live_read_pilot = bool_value(config_value(payload, "routeros_live_read_pilot"), false);
    let live_adapter_pilot = bool_value(config_value(payload, "routeros_live_read_adapter_pilot"), false);
    let authority = config_value(payload, "routeros_transport_authority")
        .and_then(Value::as_str)
        .unwrap_or("plan_only");

    let mut tcp_payload = payload.clone();
    if let Value::Object(ref mut map) = tcp_payload {
        map.insert("execute".to_string(), json!(false));
        map.insert("mode".to_string(), json!("rehearsal"));
    }
    let (tcp_contract, tcp_errors, tcp_warnings) = run_routeros_tcp_connectivity_pilot_payload(&tcp_payload);
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

    let gates_ready = allow_live_reads
        && allow_credentials
        && allow_tcp
        && allow_live_adapter
        && live_read_pilot
        && live_adapter_pilot
        && authority == "live_read_adapter_pilot";

    let mut live_result = Value::Null;
    let mut live_read_executed = false;
    let mut read_validation = Value::Null;
    let mut connection_attempt_count = 0u64;
    let mut authentication_attempt_count = 0u64;
    let mut api_sentence_write_count = 0u64;
    let mut api_reply_read_count = 0u64;
    let mut live_adapter_implemented = true;
    let mut live_transport_supported = true;

    let sentence_words: Vec<String> = api_sentence
        .get("sentence_words")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(|s| s.to_string()).collect())
        .unwrap_or_default();

    if execute || live_requested(payload) {
        if !gates_ready {
            let mut missing = Vec::new();
            if !allow_live_reads {
                missing.push("allow_rust_routeros_live_reads");
            }
            if !allow_credentials {
                missing.push("allow_rust_routeros_credentials");
            }
            if !allow_tcp {
                missing.push("allow_rust_routeros_tcp_connect");
            }
            if !allow_live_adapter {
                missing.push("allow_rust_routeros_live_read_adapter");
            }
            if !live_read_pilot {
                missing.push("routeros_live_read_pilot");
            }
            if !live_adapter_pilot {
                missing.push("routeros_live_read_adapter_pilot");
            }
            if authority != "live_read_adapter_pilot" {
                missing.push("routeros_transport_authority=live_read_adapter_pilot");
            }
            errors.push(Diagnostic::error(
                "routeros_live_read_adapter_gates_not_ready",
                Some("rust_core".to_string()),
                format!("Rust RouterOS live read adapter requested, but required gates are not enabled: {}.", missing.join(", ")),
            ));
            live_adapter_implemented = false;
            live_transport_supported = false;
        } else {
            connection_attempt_count = 1;
            authentication_attempt_count = 1;
            api_sentence_write_count = 2;
            match live_read(payload, &sentence_words, &router_name, source, path) {
                Ok(read_result) => {
                    live_read_executed = read_result.get("status").and_then(Value::as_str) == Some("ok");
                    api_reply_read_count = read_result.get("login_sentence_count").and_then(Value::as_u64).unwrap_or(0)
                        + read_result.get("read_sentence_count").and_then(Value::as_u64).unwrap_or(0);
                    let validation_payload = json!({
                        "plan": {"commands": [{"router": router_name, "source": source, "path": path, "required": true}]},
                        "results": [read_result.clone()],
                        "strict": true
                    });
                    let (validation, validation_errors, validation_warnings) = validate_routeros_read_results_payload(&validation_payload);
                    merge_diags(&mut errors, validation_errors);
                    merge_diags(&mut warnings, validation_warnings);
                    read_validation = validation;
                    live_result = read_result;
                }
                Err(message) => {
                    errors.push(Diagnostic::error(
                        "routeros_live_read_adapter_failed",
                        Some("router.address".to_string()),
                        message,
                    ));
                }
            }
        }
    }

    let status = if !errors.is_empty() {
        "blocked"
    } else if live_read_executed {
        "live_read_adapter_read_complete"
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
        "live_transport_supported": live_transport_supported,
        "live_adapter_implemented": live_adapter_implemented,
        "execute_requested": execute,
        "gates_ready_for_live_read": gates_ready,
        "router": router_name,
        "source": source,
        "path": path,
        "tcp_contract": tcp_contract,
        "auth_session": auth_session,
        "api_sentence": api_sentence,
        "read_result": live_result,
        "read_validation": read_validation,
        "authenticated": authenticated,
        "credential_material": "redacted",
        "username_emitted": false,
        "password_emitted": false,
        "credentials_used_for_routeros_auth": gates_ready && connection_attempt_count > 0,
        "session_token_emitted": false,
        "connection_attempt_count": connection_attempt_count,
        "authentication_attempt_count": authentication_attempt_count,
        "api_sentence_write_count": api_sentence_write_count,
        "api_reply_read_count": api_reply_read_count,
        "safe_for_cleanup": false,
        "collector_authority": "python_authoritative",
        "next_stage": "rust_routeros_live_read_shadow_parity",
        "note": "This guarded adapter can execute one read-only RouterOS API print when all live-read gates are enabled. It never writes RouterOS config, never emits credentials in output, and does not replace Python collectors."
    });

    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::net::TcpListener;
    use std::thread;

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
    fn blocks_live_read_adapter_execution_without_gates() {
        let payload = json!({
            "router": {"name":"R1", "address":"10.0.0.1", "port":8728, "username":"admin", "password":"super-secret"},
            "adapter": "live",
            "mode": "live_read",
            "execute": true,
            "path": "/ppp/active",
            "fixture_reply_words": ["!done"]
        });
        let (result, errors, _warnings) = run_routeros_live_read_adapter_pilot_payload(&payload);
        assert!(!errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("blocked"));
        assert!(errors.iter().any(|e| e.code == "routeros_live_read_adapter_gates_not_ready"));
        assert_eq!(result.get("connection_attempt_count").and_then(Value::as_u64), Some(0));
    }

    #[test]
    fn executes_read_only_live_adapter_against_local_routeros_fixture() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local RouterOS fixture");
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut stream, _addr) = listener.accept().expect("accept fixture connection");
            let login = read_sentence(&mut stream).expect("read login sentence");
            assert_eq!(login[0], "/login");
            assert!(login.iter().any(|w| w == "=name=admin"));
            assert!(login.iter().any(|w| w == "=password=fixture-secret"));
            write_sentence(&mut stream, &["!done".to_string()]).expect("write login done");

            let read = read_sentence(&mut stream).expect("read print sentence");
            assert_eq!(read[0], "/ppp/active/print");
            assert!(read.iter().any(|w| w == "=.proplist=name,address"));
            write_sentence(&mut stream, &[
                "!re".to_string(),
                "=name=juan".to_string(),
                "=address=10.0.0.2".to_string(),
            ]).expect("write row");
            write_sentence(&mut stream, &["!done".to_string()]).expect("write read done");
        });

        let payload = json!({
            "router": {"name":"R1", "address":"127.0.0.1", "port":port, "username":"admin", "password":"fixture-secret"},
            "adapter": "live",
            "mode": "live_read",
            "execute": true,
            "path": "/ppp/active",
            "fields": ["name", "address"],
            "fixture_reply_words": ["!done"],
            "rust_core": {
                "allow_rust_routeros_live_reads": true,
                "allow_rust_routeros_credentials": true,
                "allow_rust_routeros_tcp_connect": true,
                "allow_rust_routeros_live_read_adapter": true,
                "routeros_live_read_pilot": true,
                "routeros_live_read_adapter_pilot": true,
                "routeros_transport_authority": "live_read_adapter_pilot",
                "routeros_live_read_timeout_seconds": 2
            }
        });
        let (result, errors, _warnings) = run_routeros_live_read_adapter_pilot_payload(&payload);
        handle.join().expect("fixture server joined");
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("live_read_adapter_read_complete"));
        assert_eq!(result.get("connection_attempt_count").and_then(Value::as_u64), Some(1));
        assert_eq!(result.get("authentication_attempt_count").and_then(Value::as_u64), Some(1));
        assert_eq!(result.get("api_sentence_write_count").and_then(Value::as_u64), Some(2));
        assert_eq!(result.get("collector_authority").and_then(Value::as_str), Some("python_authoritative"));
        assert_eq!(result.get("safe_for_cleanup").and_then(Value::as_bool), Some(false));
        assert_eq!(result["read_result"]["row_count"], 1);
        assert_eq!(result["read_result"]["rows"][0]["name"], "juan");
        let text = serde_json::to_string(&result).unwrap();
        assert!(!text.contains("fixture-secret"));
    }
}
