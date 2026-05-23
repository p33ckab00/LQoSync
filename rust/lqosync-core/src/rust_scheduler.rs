use crate::protocol::Diagnostic;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn read_json_file(path: &str) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {path}: {e}"))
}

fn get_path<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cur = v;
    for key in path {
        cur = cur.get(*key)?;
    }
    Some(cur)
}

fn str_path(v: &Value, path: &[&str], default: &str) -> String {
    get_path(v, path)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

fn bool_path(v: &Value, path: &[&str], default: bool) -> bool {
    get_path(v, path)
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn int_path(v: &Value, path: &[&str], default: i64) -> i64 {
    get_path(v, path).and_then(Value::as_i64).unwrap_or(default)
}

fn config_path_from_payload(payload: &Value) -> String {
    payload
        .get("config_path")
        .and_then(Value::as_str)
        .unwrap_or("/opt/libreqos/src/config.json")
        .to_string()
}

fn run_cycle_command_config_key(mode: &str) -> &'static str {
    match mode {
        "manual" | "force_apply" => "manual_run_command",
        _ => "rust_run_cycle_command",
    }
}

fn default_run_cycle_command(mode: &str) -> String {
    format!(
        "/opt/LQoSync/scripts/rust-run-cycle-authority.sh {}",
        shell_escape(mode)
    )
}

fn scheduler_state_paths(config: &Value) -> (String, String, String) {
    let runtime_state = str_path(
        config,
        &["paths", "runtime_state"],
        "/opt/LQoSync/state/runtime_state.json",
    );
    let heartbeat = str_path(
        config,
        &["scheduler", "rust_heartbeat_path"],
        "/opt/LQoSync/state/rust_scheduler_heartbeat.json",
    );
    let lock = str_path(
        config,
        &["scheduler", "rust_lock_path"],
        "/opt/LQoSync/state/rust_scheduler.lock",
    );
    (runtime_state, heartbeat, lock)
}

fn atomic_write_json(path: &str, value: &Value) -> Result<(), String> {
    let p = Path::new(path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create parent {}: {e}", parent.display()))?;
    }
    let tmp = format!("{path}.tmp.{}", std::process::id());
    fs::write(
        &tmp,
        serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("write tmp {tmp}: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename {tmp} -> {path}: {e}"))?;
    Ok(())
}

fn write_scheduler_heartbeat(
    config: &Value,
    status: &str,
    mode: &str,
    detail: Value,
) -> Result<Value, String> {
    let (runtime_state, heartbeat, lock_path) = scheduler_state_paths(config);
    let stamp = json!({
        "schema": "lqosync.rust_scheduler_heartbeat.v1",
        "status": status,
        "mode": mode,
        "created_epoch": now_epoch(),
        "runtime_state": runtime_state,
        "lock_path": lock_path,
        "detail": detail,
        "authority": "rust_scheduler_authority",
    });
    atomic_write_json(&heartbeat, &stamp)?;
    Ok(stamp)
}

fn authority_checks(config: &Value) -> (bool, Vec<Diagnostic>, Vec<Diagnostic>, Value) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let scheduler_engine = str_path(config, &["scheduler", "engine"], "rust");
    let allow_python = bool_path(config, &["scheduler", "allow_python_scheduler"], false);
    let rust_required = bool_path(
        config,
        &["scheduler", "rust_authority_daemon_required"],
        true,
    );
    let full_rust = bool_path(config, &["rust_core", "full_rust_backend_authority"], true);
    let python_fallback = bool_path(config, &["rust_core", "python_mutation_fallback"], false);
    let stable_ready = bool_path(
        config,
        &["rust_core", "rust_set_and_forget_candidate_enabled"],
        true,
    );
    if scheduler_engine != "rust" {
        errors.push(Diagnostic::error(
            "rust_scheduler_engine_not_rust",
            Some("scheduler.engine".to_string()),
            "scheduler.engine must be rust for Rust scheduler authority.",
        ));
    }
    if allow_python {
        errors.push(Diagnostic::error(
            "python_scheduler_not_retired",
            Some("scheduler.allow_python_scheduler".to_string()),
            "Python scheduler loop must be disabled in Rust scheduler authority mode.",
        ));
    }
    if !full_rust {
        errors.push(Diagnostic::error(
            "full_rust_backend_authority_required",
            Some("rust_core.full_rust_backend_authority".to_string()),
            "Rust scheduler authority requires full Rust backend authority.",
        ));
    }
    if python_fallback {
        errors.push(Diagnostic::error(
            "python_mutation_fallback_must_be_false",
            Some("rust_core.python_mutation_fallback".to_string()),
            "Python mutation fallback must be disabled.",
        ));
    }
    if rust_required && !stable_ready {
        warnings.push(Diagnostic::warning(
            "set_and_forget_gate_not_enabled",
            Some("rust_core.rust_set_and_forget_candidate_enabled".to_string()),
            "Set-and-forget readiness gate should be enabled for scheduler authority.",
        ));
    }
    let checks = json!({
        "scheduler_engine": scheduler_engine,
        "allow_python_scheduler": allow_python,
        "rust_authority_daemon_required": rust_required,
        "full_rust_backend_authority": full_rust,
        "python_mutation_fallback": python_fallback,
        "rust_set_and_forget_candidate_enabled": stable_ready,
    });
    (errors.is_empty(), errors, warnings, checks)
}

pub fn scheduler_status_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let config_path = config_path_from_payload(payload);
    let config = match read_json_file(&config_path) {
        Ok(v) => v,
        Err(e) => {
            return (
                json!({"status":"config_unavailable","config_path":config_path,"error":e}),
                vec![Diagnostic::error(
                    "scheduler_config_unavailable",
                    Some("config_path".to_string()),
                    "Unable to read scheduler config.",
                )],
                vec![],
            )
        }
    };
    let (ok, errors, warnings, checks) = authority_checks(&config);
    let (runtime_state, heartbeat, lock_path) = scheduler_state_paths(&config);
    let heartbeat_exists = Path::new(&heartbeat).exists();
    let lock_exists = Path::new(&lock_path).exists();
    (
        json!({
            "schema": "lqosync.rust_scheduler_status.v1",
            "status": if ok {"ok"} else {"blocked"},
            "config_path": config_path,
            "scheduler_enabled": bool_path(&config, &["scheduler", "enabled"], false),
            "authority": "rust_scheduler_authority",
            "runtime_state": runtime_state,
            "heartbeat_path": heartbeat,
            "heartbeat_exists": heartbeat_exists,
            "lock_path": lock_path,
            "lock_exists": lock_exists,
            "checks": checks,
        }),
        errors,
        warnings,
    )
}

pub fn scheduler_heartbeat_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let config_path = config_path_from_payload(payload);
    let mode = payload
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("heartbeat");
    let config = match read_json_file(&config_path) {
        Ok(v) => v,
        Err(e) => {
            return (
                json!({"status":"config_unavailable","error":e}),
                vec![Diagnostic::error(
                    "scheduler_config_unavailable",
                    Some("config_path".to_string()),
                    "Unable to read scheduler config.",
                )],
                vec![],
            )
        }
    };
    let (ok, mut errors, warnings, checks) = authority_checks(&config);
    if !ok {
        return (
            json!({"status":"blocked","checks":checks}),
            errors,
            warnings,
        );
    }
    match write_scheduler_heartbeat(&config, "ok", mode, checks) {
        Ok(stamp) => (json!({"status":"ok","heartbeat":stamp}), errors, warnings),
        Err(e) => {
            errors.push(Diagnostic::error(
                "scheduler_heartbeat_write_failed",
                Some("scheduler.rust_heartbeat_path".to_string()),
                e.clone(),
            ));
            (json!({"status":"failed","error":e}), errors, warnings)
        }
    }
}

pub fn scheduler_decision_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let config_path = config_path_from_payload(payload);
    let files_changed = payload
        .get("files_changed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let last_error = payload
        .get("last_error")
        .and_then(Value::as_str)
        .unwrap_or("");
    let config = match read_json_file(&config_path) {
        Ok(v) => v,
        Err(e) => {
            return (
                json!({"status":"config_unavailable","error":e}),
                vec![Diagnostic::error(
                    "scheduler_config_unavailable",
                    Some("config_path".to_string()),
                    "Unable to read scheduler config.",
                )],
                vec![],
            )
        }
    };
    let (ok, errors, warnings, checks) = authority_checks(&config);
    let interval = if !last_error.is_empty() {
        int_path(&config, &["scheduler", "error_retry_interval_seconds"], 30)
    } else if files_changed {
        int_path(&config, &["scheduler", "active_interval_seconds"], 30)
    } else {
        int_path(&config, &["scheduler", "idle_interval_seconds"], 120)
    };
    (
        json!({
            "schema": "lqosync.rust_scheduler_decision.v1",
            "status": if ok {"run_allowed"} else {"blocked"},
            "run_allowed": ok,
            "next_interval_seconds": interval,
            "files_changed": files_changed,
            "last_error": last_error,
            "checks": checks,
            "authority": "rust_scheduler_authority",
        }),
        errors,
        warnings,
    )
}

pub fn scheduler_run_once_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let config_path = config_path_from_payload(payload);
    let mode = payload
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("manual");
    let execute = payload
        .get("execute")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let config = match read_json_file(&config_path) {
        Ok(v) => v,
        Err(e) => {
            return (
                json!({"status":"config_unavailable","error":e}),
                vec![Diagnostic::error(
                    "scheduler_config_unavailable",
                    Some("config_path".to_string()),
                    "Unable to read scheduler config.",
                )],
                vec![],
            )
        }
    };
    let (ok, mut errors, warnings, checks) = authority_checks(&config);
    let enabled = bool_path(&config, &["scheduler", "enabled"], false)
        || mode == "manual"
        || mode == "force_apply";
    if !enabled {
        errors.push(Diagnostic::error(
            "scheduler_disabled",
            Some("scheduler.enabled".to_string()),
            "Scheduler is disabled.",
        ));
    }
    if !ok || !enabled {
        return (
            json!({"schema":"lqosync.rust_scheduler_run_once.v1","status":"blocked","run_allowed":false,"checks":checks}),
            errors,
            warnings,
        );
    }
    if !execute {
        let _ = write_scheduler_heartbeat(&config, "authorized", mode, checks.clone());
        return (
            json!({"schema":"lqosync.rust_scheduler_run_once.v1","status":"authorized","run_allowed":true,"execute":false,"checks":checks}),
            errors,
            warnings,
        );
    }
    let command_key = run_cycle_command_config_key(mode);
    let default_cmd = default_run_cycle_command(mode);
    let command = payload
        .get("command")
        .and_then(Value::as_str)
        .or_else(|| get_path(&config, &["scheduler", command_key]).and_then(Value::as_str))
        .unwrap_or(&default_cmd)
        .to_string();
    let _ = write_scheduler_heartbeat(
        &config,
        "running",
        mode,
        json!({"command":command,"checks":checks}),
    );
    let started = now_epoch();
    let output = Command::new("sh").arg("-lc").arg(&command).output();
    match output {
        Ok(out) => {
            let code = out.status.code().unwrap_or(-1);
            let success = out.status.success();
            let stdout = String::from_utf8_lossy(&out.stdout)
                .chars()
                .take(20000)
                .collect::<String>();
            let stderr = String::from_utf8_lossy(&out.stderr)
                .chars()
                .take(20000)
                .collect::<String>();
            let status = if success { "ok" } else { "failed" };
            let _ = write_scheduler_heartbeat(
                &config,
                status,
                mode,
                json!({"exit_code":code,"started_epoch":started,"finished_epoch":now_epoch()}),
            );
            if !success {
                errors.push(Diagnostic::error(
                    "rust_scheduler_run_once_command_failed",
                    Some(format!("scheduler.{command_key}")),
                    format!("Run-cycle command failed with exit code {code}."),
                ));
            }
            (
                json!({
                    "schema":"lqosync.rust_scheduler_run_once.v1",
                    "status": status,
                    "run_allowed": true,
                    "executed": true,
                    "exit_code": code,
                    "stdout": stdout,
                    "stderr": stderr,
                    "command": command,
                    "authority": "rust_scheduler_authority",
                }),
                errors,
                warnings,
            )
        }
        Err(e) => {
            let msg = e.to_string();
            let _ = write_scheduler_heartbeat(&config, "failed", mode, json!({"error":msg}));
            errors.push(Diagnostic::error(
                "rust_scheduler_run_once_spawn_failed",
                Some(format!("scheduler.{command_key}")),
                msg.clone(),
            ));
            (
                json!({"schema":"lqosync.rust_scheduler_run_once.v1","status":"failed","error":msg}),
                errors,
                warnings,
            )
        }
    }
}

fn shell_escape(s: &str) -> String {
    let safe = s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c));
    if safe {
        s.to_string()
    } else {
        "manual".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scheduler_authority_blocks_python_scheduler() {
        let cfg = json!({"scheduler":{"engine":"rust","allow_python_scheduler":false},"rust_core":{"full_rust_backend_authority":true,"python_mutation_fallback":false,"rust_set_and_forget_candidate_enabled":true}});
        let (ok, errors, _warnings, checks) = authority_checks(&cfg);
        assert!(ok, "errors={errors:?} checks={checks}");
    }
}
