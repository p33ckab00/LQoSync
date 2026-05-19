use crate::apply_manifest::build_apply_manifest_payload;
use crate::atomic_state::atomic_write_text;
use crate::protocol::{Diagnostic, Severity};
use serde_json::{json, Value};
use std::path::Path;

fn str_path<'a>(value: &'a Value, path: &[&str], default: &'a str) -> &'a str {
    let mut current = value;
    for part in path {
        match current.get(*part) {
            Some(next) => current = next,
            None => return default,
        }
    }
    current.as_str().unwrap_or(default)
}

fn bool_path(value: &Value, path: &[&str], default: bool) -> bool {
    let mut current = value;
    for part in path {
        match current.get(*part) {
            Some(next) => current = next,
            None => return default,
        }
    }
    current.as_bool().unwrap_or(default)
}

fn warning(code: &str, path: Option<String>, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity: Severity::Warning,
        path,
        message: message.to_string(),
        value: None,
        safe_for_cleanup: None,
    }
}

/// Execute the safe file-write part of an apply manifest.
///
/// This operation is intentionally opt-in. By default it only returns a
/// transaction rehearsal. It never invokes LibreQoS.py in v1.0; Python remains
/// authoritative for external command execution.
pub fn execute_apply_transaction_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let (manifest, mut errors, mut warnings) = build_apply_manifest_payload(payload);
    let execute = payload.get("execute").and_then(Value::as_bool).unwrap_or(false);
    let allow_file_writes = payload.get("allow_file_writes").and_then(Value::as_bool).unwrap_or(false);
    let allow_libreqos_apply = payload.get("allow_libreqos_apply").and_then(Value::as_bool).unwrap_or(false);
    let dry_run = payload.get("mode").and_then(Value::as_str).unwrap_or("apply") == "dry_run";
    let status = manifest.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let paths = payload
        .get("paths")
        .cloned()
        .or_else(|| payload.get("config").and_then(|c| c.get("paths")).cloned())
        .unwrap_or_else(|| json!({}));
    let config = payload.get("config").cloned().unwrap_or_else(|| json!({}));
    let current_csv = payload.get("current_csv_text").and_then(Value::as_str).unwrap_or("");
    let proposed_csv = payload.get("proposed_csv_text").and_then(Value::as_str).unwrap_or("");
    let current_network = payload.get("current_network_text").and_then(Value::as_str).unwrap_or("{}");
    let proposed_network = payload.get("proposed_network_text").and_then(Value::as_str).unwrap_or("{}");
    let csv_path = str_path(&paths, &["shaped_devices_csv"], "");
    let network_path = str_path(&paths, &["network_json"], "");
    let backup_before_apply = bool_path(&config, &["app", "backup_before_apply"], false);
    let csv_changed = manifest.get("csv_changed").and_then(Value::as_bool).unwrap_or(current_csv != proposed_csv);
    let network_changed = manifest.get("network_changed").and_then(Value::as_bool).unwrap_or(current_network != proposed_network);
    let hashes = manifest.get("hashes").cloned().unwrap_or_else(|| json!({}));

    let mut write_results: Vec<Value> = Vec::new();
    let mut trace: Vec<Value> = Vec::new();

    if allow_libreqos_apply {
        warnings.push(warning(
            "transaction_libreqos_apply_not_implemented",
            Some("allow_libreqos_apply".to_string()),
            "Rust transaction executor v1.0 does not invoke LibreQoS.py; Python remains authoritative for external apply execution.",
        ));
        trace.push(json!({"step":"libreqos_apply","decision":"delegated_to_python"}));
    }

    if dry_run {
        trace.push(json!({"step":"execute","decision":"dry_run_preview_only"}));
    } else if status != "ready" {
        trace.push(json!({"step":"execute","decision":"not_ready","manifest_status":status}));
    } else if !execute || !allow_file_writes {
        trace.push(json!({"step":"execute","decision":"rehearsal_only","execute":execute,"allow_file_writes":allow_file_writes}));
    } else {
        if csv_changed {
            if csv_path.is_empty() {
                errors.push(Diagnostic::error(
                    "transaction_missing_csv_path",
                    Some("paths.shaped_devices_csv".to_string()),
                    "Cannot execute CSV write because shaped_devices_csv path is empty",
                ));
            } else {
                match atomic_write_text(
                    Path::new(csv_path),
                    proposed_csv,
                    backup_before_apply,
                    hashes.get("current_csv").and_then(Value::as_str),
                    "ShapedDevices.csv",
                ) {
                    Ok(result) => {
                        write_results.push(result);
                        trace.push(json!({"step":"write_csv","decision":"wrote","path":csv_path}));
                    }
                    Err(e) => errors.push(Diagnostic::error(
                        "transaction_csv_write_failed",
                        Some("paths.shaped_devices_csv".to_string()),
                        format!("CSV write failed: {e}"),
                    )),
                }
            }
        }
        if network_changed {
            if network_path.is_empty() {
                errors.push(Diagnostic::error(
                    "transaction_missing_network_path",
                    Some("paths.network_json".to_string()),
                    "Cannot execute network write because network_json path is empty",
                ));
            } else {
                match atomic_write_text(
                    Path::new(network_path),
                    proposed_network,
                    backup_before_apply,
                    hashes.get("current_network").and_then(Value::as_str),
                    "network.json",
                ) {
                    Ok(result) => {
                        write_results.push(result);
                        trace.push(json!({"step":"write_network","decision":"wrote","path":network_path}));
                    }
                    Err(e) => errors.push(Diagnostic::error(
                        "transaction_network_write_failed",
                        Some("paths.network_json".to_string()),
                        format!("network.json write failed: {e}"),
                    )),
                }
            }
        }
    }

    let executed = execute && allow_file_writes && !dry_run && status == "ready" && errors.is_empty();
    let final_status = if !errors.is_empty() {
        "failed"
    } else if executed {
        "executed_file_writes"
    } else if dry_run {
        "dry_run_preview_only"
    } else if status != "ready" {
        "not_ready"
    } else {
        "rehearsal_only"
    };

    let result = json!({
        "mode": "transaction_executor",
        "authoritative": executed,
        "executed": executed,
        "status": final_status,
        "manifest": manifest,
        "write_results": write_results,
        "write_count": write_results.len(),
        "execute_requested": execute,
        "allow_file_writes": allow_file_writes,
        "allow_libreqos_apply": allow_libreqos_apply,
        "libreqos_apply_executed": false,
        "trace": trace,
    });
    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> String {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("lqosync-{name}-{now}")).to_string_lossy().to_string()
    }

    #[test]
    fn rehearses_without_execute_flag() {
        let csv = temp_path("shaped.csv");
        fs::write(&csv, "Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,MAC,IPv4,IPv6,Download Min Mbps,Upload Min Mbps,Download Max Mbps,Upload Max Mbps,Comment\n").unwrap();
        let payload = json!({
            "mode":"apply",
            "paths":{"shaped_devices_csv":csv,"network_json":""},
            "current_csv_text":"old",
            "proposed_csv_text":"new",
            "current_network_text":"{}",
            "proposed_network_text":"{}",
            "files_changed":true,
            "csv_changed":true,
            "network_changed":false,
            "policy_decision":{"write_allowed":true,"apply_allowed":true},
            "execute":false,
            "allow_file_writes":false
        });
        let (result, errors, _warnings) = execute_apply_transaction_payload(&payload);
        assert!(errors.is_empty());
        assert_eq!(result.get("executed").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("status").and_then(Value::as_str), Some("rehearsal_only"));
    }

    #[test]
    fn executes_file_write_when_explicitly_allowed() {
        let csv = temp_path("shaped.csv");
        fs::write(&csv, "old").unwrap();
        let payload = json!({
            "mode":"apply",
            "paths":{"shaped_devices_csv":csv,"network_json":""},
            "current_csv_text":"old",
            "proposed_csv_text":"new",
            "current_network_text":"{}",
            "proposed_network_text":"{}",
            "files_changed":true,
            "csv_changed":true,
            "network_changed":false,
            "policy_decision":{"write_allowed":true,"apply_allowed":true},
            "execute":true,
            "allow_file_writes":true
        });
        let (result, errors, _warnings) = execute_apply_transaction_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("executed").and_then(Value::as_bool), Some(true));
        let manifest = result.get("manifest").unwrap();
        assert_eq!(manifest.get("status").and_then(Value::as_str), Some("ready"));
    }

    #[test]
    fn refuses_blocked_manifest() {
        let payload = json!({
            "mode":"apply",
            "paths":{"shaped_devices_csv":"/tmp/x","network_json":"/tmp/y"},
            "current_csv_text":"old",
            "proposed_csv_text":"new",
            "files_changed":true,
            "csv_changed":true,
            "network_changed":false,
            "policy_decision":{"write_allowed":false,"apply_allowed":false},
            "execute":true,
            "allow_file_writes":true
        });
        let (result, _errors, _warnings) = execute_apply_transaction_payload(&payload);
        assert_eq!(result.get("executed").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("status").and_then(Value::as_str), Some("not_ready"));
    }
}
