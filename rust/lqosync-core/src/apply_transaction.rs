use crate::apply_manifest::build_apply_manifest_payload;
use crate::atomic_state::atomic_write_text;
use crate::protocol::{Diagnostic, Severity};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

fn now_run_id() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{}_{}", now.as_secs(), now.subsec_micros())
}

fn string_array(value: Option<&Value>, default: &[&str]) -> Vec<String> {
    match value.and_then(Value::as_array) {
        Some(items) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        None => default.iter().map(|v| (*v).to_string()).collect(),
    }
}

fn libreqos_command(config: &Value) -> (Vec<String>, String, String, String, u64) {
    let cmd = str_path(config, &["libreqos", "cmd"], "/opt/libreqos/src/LibreQoS.py").to_string();
    let args = string_array(config.pointer("/libreqos/args"), &["--updateonly"]);
    let working_dir = str_path(config, &["libreqos", "working_dir"], "/opt/libreqos/src").to_string();
    let mode = str_path(config, &["libreqos", "run_mode"], "direct").to_string();
    let timeout_seconds = config
        .pointer("/libreqos/timeout_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(300)
        .max(1);
    let use_sudo = bool_path(config, &["libreqos", "sudo"], true);

    if mode == "host_nsenter" {
        let mut inner = String::new();
        inner.push_str("cd ");
        inner.push_str(&shell_quote(&working_dir));
        inner.push_str(" && exec ");
        if cmd.ends_with(".py") {
            inner.push_str("/usr/bin/python3 ");
        }
        inner.push_str(&shell_quote(&cmd));
        for arg in &args {
            inner.push(' ');
            inner.push_str(&shell_quote(arg));
        }
        return (
            vec![
                "/usr/bin/nsenter".to_string(),
                "-t".to_string(),
                "1".to_string(),
                "-m".to_string(),
                "-u".to_string(),
                "-n".to_string(),
                "-i".to_string(),
                "--".to_string(),
                "/bin/bash".to_string(),
                "-lc".to_string(),
                inner,
            ],
            working_dir,
            mode,
            cmd,
            timeout_seconds,
        );
    }

    let mut command: Vec<String> = Vec::new();
    if use_sudo {
        command.push("/usr/bin/sudo".to_string());
    }
    if cmd.ends_with(".py") {
        command.push("/usr/bin/python3".to_string());
        command.push(cmd.clone());
    } else {
        command.push(cmd.clone());
    }
    command.extend(args);
    (command, working_dir, mode, cmd, timeout_seconds)
}

fn shell_quote(value: &str) -> String {
    if value.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | ':' | '=')) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn run_libreqos_update_from_rust(config: &Value) -> (Value, Option<Diagnostic>) {
    let (command, working_dir, mode, cmd, timeout_seconds) = libreqos_command(config);
    let log_dir = str_path(config, &["paths", "libreqos_apply_log_dir"], "/opt/LQoSync/logs/libreqos_apply").to_string();
    let run_id = now_run_id();
    let started_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let stdout_path = Path::new(&log_dir).join(format!("{run_id}.stdout.log"));
    let stderr_path = Path::new(&log_dir).join(format!("{run_id}.stderr.log"));
    let meta_path = Path::new(&log_dir).join(format!("{run_id}.json"));

    let mut diagnostic: Option<Diagnostic> = None;
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut exit_code: i32 = -1;
    let mut ok = false;
    let mut timed_out = false;
    let started = Instant::now();

    if command.is_empty() {
        diagnostic = Some(Diagnostic::error(
            "rust_libreqos_command_empty",
            Some("libreqos.cmd".to_string()),
            "LibreQoS command resolved to an empty command",
        ));
    } else if mode != "host_nsenter" && !Path::new(&working_dir).is_dir() {
        diagnostic = Some(Diagnostic::error(
            "rust_libreqos_working_dir_invalid",
            Some("libreqos.working_dir".to_string()),
            format!("Invalid libreqos.working_dir: {working_dir}"),
        ));
    } else if mode != "host_nsenter" && cmd.ends_with(".py") && !Path::new(&cmd).is_file() {
        diagnostic = Some(Diagnostic::error(
            "rust_libreqos_cmd_missing",
            Some("libreqos.cmd".to_string()),
            format!("LibreQoS.py command does not exist: {cmd}"),
        ));
    } else {
        let mut process = Command::new(&command[0]);
        process.args(&command[1..]);
        if mode != "host_nsenter" {
            process.current_dir(&working_dir);
        }
        process.stdout(Stdio::piped()).stderr(Stdio::piped());
        match process.spawn() {
            Ok(mut child) => {
                loop {
                    match child.try_wait() {
                        Ok(Some(_status)) => break,
                        Ok(None) => {
                            if started.elapsed() > Duration::from_secs(timeout_seconds) {
                                timed_out = true;
                                let _ = child.kill();
                                break;
                            }
                            thread::sleep(Duration::from_millis(100));
                        }
                        Err(e) => {
                            diagnostic = Some(Diagnostic::error(
                                "rust_libreqos_wait_failed",
                                Some("libreqos.cmd".to_string()),
                                format!("Failed while waiting for LibreQoS.py: {e}"),
                            ));
                            let _ = child.kill();
                            break;
                        }
                    }
                }
                match child.wait_with_output() {
                    Ok(output) => {
                        stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        exit_code = output.status.code().unwrap_or(-1);
                        ok = output.status.success() && !timed_out;
                        if timed_out {
                            diagnostic = Some(Diagnostic::error(
                                "rust_libreqos_timeout",
                                Some("libreqos.timeout_seconds".to_string()),
                                format!("LibreQoS.py timed out after {timeout_seconds} seconds"),
                            ));
                        } else if !ok {
                            diagnostic = Some(Diagnostic::error(
                                "rust_libreqos_apply_failed",
                                Some("libreqos.cmd".to_string()),
                                format!("LibreQoS.py exited with code {exit_code}"),
                            ));
                        }
                    }
                    Err(e) => {
                        diagnostic = Some(Diagnostic::error(
                            "rust_libreqos_output_failed",
                            Some("libreqos.cmd".to_string()),
                            format!("Failed to collect LibreQoS.py output: {e}"),
                        ));
                    }
                }
            }
            Err(e) => {
                diagnostic = Some(Diagnostic::error(
                    "rust_libreqos_spawn_failed",
                    Some("libreqos.cmd".to_string()),
                    format!("Failed to spawn LibreQoS.py: {e}"),
                ));
            }
        }
    }

    let duration_ms = started.elapsed().as_millis() as u64;
    let _ = fs::create_dir_all(&log_dir);
    let _ = fs::write(&stdout_path, &stdout);
    let _ = fs::write(&stderr_path, &stderr);
    let meta = json!({
        "run_id": run_id,
        "started_epoch": started_epoch,
        "duration_ms": duration_ms,
        "duration_seconds": (duration_ms as f64) / 1000.0,
        "exit_code": exit_code,
        "ok": ok,
        "timed_out": timed_out,
        "command": command,
        "working_dir": working_dir,
        "mode": mode,
        "executor": "rust",
        "stdout_path": stdout_path.to_string_lossy(),
        "stderr_path": stderr_path.to_string_lossy(),
    });
    let _ = fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap_or_else(|_| "{}".to_string()));

    let result = json!({
        "ok": ok,
        "exit_code": exit_code,
        "stdout": stdout,
        "stderr": stderr,
        "command": meta.get("command").cloned().unwrap_or_else(|| json!([])),
        "working_dir": meta.get("working_dir").cloned().unwrap_or_else(|| json!("")),
        "mode": meta.get("mode").cloned().unwrap_or_else(|| json!("direct")),
        "duration_ms": duration_ms,
        "duration_seconds": (duration_ms as f64) / 1000.0,
        "run_id": meta.get("run_id").cloned().unwrap_or_else(|| json!("")),
        "meta": meta,
    });
    (result, diagnostic)
}

/// Execute the Rust-owned apply transaction.
///
/// This operation is still explicitly gated. With `execute=false` it rehearses.
/// With `execute=true`, `allow_file_writes=true`, and a ready manifest it owns
/// the atomic ShapedDevices/network writes. With `allow_libreqos_apply=true`, it
/// also invokes LibreQoS.py and records the same apply-run log family that the
/// Python executor writes.
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
    let mut file_writes_executed = false;
    let mut libreqos_apply_executed = false;
    let mut libreqos_apply_result = json!({});

    if dry_run {
        trace.push(json!({"step":"execute","decision":"dry_run_preview_only"}));
    } else if status != "ready" {
        trace.push(json!({"step":"execute","decision":"not_ready","manifest_status":status}));
    } else if !execute {
        trace.push(json!({"step":"execute","decision":"rehearsal_only","execute":execute}));
    } else {
        if allow_file_writes {
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
                            file_writes_executed = true;
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
                        "Cannot execute network.json write because network_json path is empty",
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
                            file_writes_executed = true;
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
        } else {
            trace.push(json!({"step":"file_writes","decision":"not_allowed","allow_file_writes":false}));
        }

        if allow_libreqos_apply && errors.is_empty() {
            let (apply_result, apply_error) = run_libreqos_update_from_rust(&config);
            libreqos_apply_executed = true;
            libreqos_apply_result = apply_result;
            trace.push(json!({
                "step":"libreqos_apply",
                "decision":"executed_by_rust",
                "ok": libreqos_apply_result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "exit_code": libreqos_apply_result.get("exit_code").and_then(Value::as_i64).unwrap_or(-1),
                "run_id": libreqos_apply_result.get("run_id").and_then(Value::as_str).unwrap_or("")
            }));
            if let Some(diag) = apply_error {
                errors.push(diag);
            }
        } else if allow_libreqos_apply {
            trace.push(json!({"step":"libreqos_apply","decision":"skipped_due_to_previous_errors"}));
        } else {
            trace.push(json!({"step":"libreqos_apply","decision":"not_allowed","allow_libreqos_apply":false}));
        }
    }

    let authoritative = execute && !dry_run && status == "ready" && (allow_file_writes || allow_libreqos_apply);
    let executed = file_writes_executed || libreqos_apply_executed;
    let final_status = if !errors.is_empty() {
        "failed"
    } else if libreqos_apply_executed {
        "executed_full_apply"
    } else if file_writes_executed {
        "executed_file_writes"
    } else if dry_run {
        "dry_run_preview_only"
    } else if status != "ready" {
        "not_ready"
    } else {
        "rehearsal_only"
    };

    if allow_libreqos_apply && execute && !libreqos_apply_executed && final_status == "rehearsal_only" {
        warnings.push(warning(
            "rust_libreqos_apply_not_executed",
            Some("allow_libreqos_apply".to_string()),
            "Rust was allowed to apply LibreQoS, but transaction did not reach the execution phase.",
        ));
    }

    let result = json!({
        "mode": "transaction_executor",
        "authoritative": authoritative,
        "executed": executed,
        "file_writes_executed": file_writes_executed,
        "status": final_status,
        "manifest": manifest,
        "write_results": write_results,
        "write_count": write_results.len(),
        "execute_requested": execute,
        "allow_file_writes": allow_file_writes,
        "allow_libreqos_apply": allow_libreqos_apply,
        "libreqos_apply_executed": libreqos_apply_executed,
        "libreqos_apply_result": libreqos_apply_result,
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
            "allow_file_writes":true,
            "allow_libreqos_apply":false
        });
        let (result, errors, _warnings) = execute_apply_transaction_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("executed").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("file_writes_executed").and_then(Value::as_bool), Some(true));
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
            "allow_file_writes":true,
            "allow_libreqos_apply":false
        });
        let (result, _errors, _warnings) = execute_apply_transaction_payload(&payload);
        assert_eq!(result.get("executed").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("status").and_then(Value::as_str), Some("not_ready"));
    }
}
