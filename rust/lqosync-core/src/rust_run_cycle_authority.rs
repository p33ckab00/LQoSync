use crate::apply_transaction::execute_apply_transaction_payload;
use crate::atomic_state::atomic_write_json_state_payload;
use crate::protocol::{Diagnostic, Severity};
use crate::rust_native_dry_run_preview::{
    build_rust_native_dry_run_preview_payload, empty_csv_text, load_current_rows, merge_diags,
    response_envelope, rows_to_csv_text, sha256_text,
};
use crate::rust_sync_engine_shadow_preview::build_rust_sync_engine_shadow_preview_payload;
use crate::transaction_journal::{
    append_transaction_journal_payload, build_rollback_manifest_payload,
    build_transaction_journal_payload,
};
use serde_json::{json, Value};
use std::fs;
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
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

fn read_json_file(path: &str) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {path}: {e}"))
}

fn runtime_state_path(config: &Value) -> String {
    str_path(
        config,
        &["paths", "runtime_state"],
        "/opt/LQoSync/state/runtime_state.json",
    )
    .to_string()
}

fn diag_messages(items: &[Diagnostic]) -> Vec<String> {
    items
        .iter()
        .map(|item| {
            if item.message.trim().is_empty() {
                item.code.clone()
            } else {
                format!("Rust core: {}", item.message)
            }
        })
        .collect()
}

fn config_path_from_payload(payload: &Value) -> String {
    payload
        .get("config_path")
        .and_then(Value::as_str)
        .unwrap_or("/opt/libreqos/src/config.json")
        .to_string()
}

fn load_config(payload: &Value) -> Result<Value, Diagnostic> {
    if let Some(config) = payload.get("config").filter(|value| value.is_object()) {
        return Ok(config.clone());
    }
    let config_path = config_path_from_payload(payload);
    read_json_file(&config_path).map_err(|err| {
        Diagnostic::error(
            "rust_run_cycle_authority_config_unavailable",
            Some("config_path".to_string()),
            format!("Unable to load config for Rust run-cycle authority: {err}"),
        )
    })
}

fn default_runtime_state(config: &Value) -> Value {
    json!({
        "scheduler_state": "idle",
        "scheduler_enabled": bool_path(config, &["scheduler", "enabled"], false),
        "sync_running": false,
        "libreqos_running": false,
        "last_run": Value::Null,
        "last_dry_run": Value::Null,
        "last_error": Value::Null,
        "last_libreqos_apply_success": Value::Null,
        "last_libreqos_apply_failed": false,
        "pending_libreqos_apply": false,
        "last_libreqos_apply_reason": Value::Null,
        "last_libreqos_exit_code": Value::Null,
    })
}

fn load_runtime_state(config: &Value) -> Value {
    let path = runtime_state_path(config);
    match read_json_file(&path) {
        Ok(state) if state.is_object() => state,
        _ => default_runtime_state(config),
    }
}

fn write_runtime_state(
    config: &Value,
    existing: &Value,
    cycle_result: &Value,
    mode: &str,
) -> Result<Value, String> {
    let mut state = if existing.is_object() {
        existing.clone()
    } else {
        default_runtime_state(config)
    };
    let status = cycle_result
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let libreqos_triggered = cycle_result
        .get("libreqos_triggered")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let libreqos_exit_code = cycle_result
        .get("libreqos_exit_code")
        .and_then(Value::as_i64);
    let files_changed = cycle_result
        .get("files_changed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let errors_present = cycle_result
        .get("errors")
        .and_then(Value::as_array)
        .map(|items| !items.is_empty())
        .unwrap_or(false);

    if let Some(map) = state.as_object_mut() {
        map.insert(
            "scheduler_enabled".to_string(),
            json!(bool_path(config, &["scheduler", "enabled"], false)),
        );
        map.insert(
            "scheduler_state".to_string(),
            json!(if errors_present { "error" } else { "idle" }),
        );
        map.insert("sync_running".to_string(), json!(false));
        map.insert("libreqos_running".to_string(), json!(false));
        map.insert(
            if mode == "dry_run" {
                "last_dry_run".to_string()
            } else {
                "last_run".to_string()
            },
            cycle_result.clone(),
        );
        map.insert(
            "last_error".to_string(),
            if errors_present {
                json!(status)
            } else {
                Value::Null
            },
        );
        map.insert(
            "pending_libreqos_apply".to_string(),
            json!(files_changed && !libreqos_triggered && !errors_present),
        );
        map.insert(
            "last_libreqos_apply_reason".to_string(),
            if libreqos_triggered {
                json!("rust_run_cycle_authority")
            } else {
                Value::Null
            },
        );
        map.insert(
            "last_libreqos_apply_failed".to_string(),
            json!(libreqos_triggered && libreqos_exit_code.unwrap_or(0) != 0),
        );
        map.insert(
            "last_libreqos_exit_code".to_string(),
            libreqos_exit_code.map(Value::from).unwrap_or(Value::Null),
        );
        if libreqos_triggered && libreqos_exit_code.unwrap_or(0) == 0 {
            map.insert(
                "last_libreqos_apply_success".to_string(),
                json!(now_unix_seconds()),
            );
        }
    }

    atomic_write_json_state_payload(&json!({
        "path": runtime_state_path(config),
        "state_type": "runtime_state",
        "state": state,
        "create_backup": false
    }))
    .map_err(|err| err.to_string())
}

fn python_fallback_command(mode: &str) -> Vec<String> {
    vec![
        "/opt/LQoSync/venv/bin/python".to_string(),
        "/opt/LQoSync/scripts/run_cycle_once.py".to_string(),
        mode.to_string(),
    ]
}

fn run_python_fallback(config_path: &str, mode: &str) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors = Vec::new();
    let warnings = Vec::new();
    let command = python_fallback_command(mode);
    let output = Command::new(&command[0])
        .args(&command[1..])
        .env("CONFIG_PATH", config_path)
        .output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            match serde_json::from_str::<Value>(&stdout) {
                Ok(mut result) => {
                    if let Some(map) = result.as_object_mut() {
                        map.insert("source".to_string(), json!("python_run_cycle_fallback"));
                        map.insert(
                            "meta".to_string(),
                            json!({
                                "engine": "rust_run_cycle_authority_bridge",
                                "fallback": "python_run_cycle",
                                "exit_code": out.status.code().unwrap_or(-1),
                            }),
                        );
                    }
                    if !out.status.success() {
                        errors.push(Diagnostic::error(
                            "rust_run_cycle_authority_python_fallback_failed",
                            Some(
                                "rust_core.native_run_cycle_authority_python_fallback".to_string(),
                            ),
                            format!(
                                "Python run-cycle fallback exited with code {}{}",
                                out.status.code().unwrap_or(-1),
                                if stderr.trim().is_empty() {
                                    String::new()
                                } else {
                                    format!(": {}", stderr.trim())
                                }
                            ),
                        ));
                    }
                    (result, errors, warnings)
                }
                Err(err) => {
                    errors.push(Diagnostic::error(
                        "rust_run_cycle_authority_python_fallback_parse_failed",
                        Some("python_fallback.stdout".to_string()),
                        format!("Python fallback did not return valid JSON: {err}"),
                    ));
                    (
                        json!({
                            "status": "python_fallback_failed",
                            "mode": mode,
                            "source": "python_run_cycle_fallback",
                            "errors": [format!("Python fallback parse failed: {err}")],
                            "warnings": [],
                            "stdout": stdout,
                            "stderr": stderr,
                        }),
                        errors,
                        warnings,
                    )
                }
            }
        }
        Err(err) => {
            errors.push(Diagnostic::error(
                "rust_run_cycle_authority_python_fallback_spawn_failed",
                Some("python_fallback".to_string()),
                format!("Unable to spawn Python run-cycle fallback: {err}"),
            ));
            (
                json!({
                    "status": "python_fallback_unavailable",
                    "mode": mode,
                    "source": "python_run_cycle_fallback",
                    "errors": [format!("Python fallback unavailable: {err}")],
                    "warnings": [],
                }),
                errors,
                warnings,
            )
        }
    }
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Execute one Rust-backed run-cycle authority attempt.
///
/// This operation enters Rust first for scheduler/manual run targets. When
/// `rust_core.native_run_cycle_authority_enabled=true`, it uses the Rust live
/// read + shadow + sync-engine + apply pipeline. When that flag is false and
/// `rust_core.native_run_cycle_authority_python_fallback=true`, it preserves the
/// current behavior by delegating to the Python run-cycle bridge.
pub fn run_rust_cycle_authority_payload(
    payload: &Value,
) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let started = Instant::now();
    let mode = payload
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("manual");
    let execute_requested = payload
        .get("execute")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let config_path = config_path_from_payload(payload);

    let config = match load_config(payload) {
        Ok(config) => config,
        Err(err) => {
            return (
                json!({"status": "config_unavailable", "mode": mode}),
                vec![err],
                vec![],
            )
        }
    };
    let rc = config
        .get("rust_core")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let native_enabled = rc
        .get("native_run_cycle_authority_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let python_fallback = rc
        .get("native_run_cycle_authority_python_fallback")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    if !native_enabled {
        if python_fallback {
            return run_python_fallback(&config_path, mode);
        }
        return (
            json!({
                "status": "rust_run_cycle_authority_not_enabled",
                "mode": mode,
                "source": "rust_run_cycle_authority",
                "errors": ["Rust native run-cycle authority is disabled and Python fallback is not allowed."],
                "warnings": [],
            }),
            vec![Diagnostic::error(
                "rust_run_cycle_authority_disabled",
                Some("rust_core.native_run_cycle_authority_enabled".to_string()),
                "Rust native run-cycle authority is disabled and Python fallback is not allowed.",
            )],
            vec![],
        );
    }

    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let current_state = load_runtime_state(&config);
    let mut native_preview_payload = payload.clone();
    if let Some(map) = native_preview_payload.as_object_mut() {
        map.insert("config".to_string(), config.clone());
        map.insert("state".to_string(), current_state.clone());
    }

    let (native_preview_result, preview_errors, preview_warnings) =
        build_rust_native_dry_run_preview_payload(&native_preview_payload);
    merge_diags(&mut errors, preview_errors.clone());
    merge_diags(&mut warnings, preview_warnings.clone());
    let native_preview_envelope = response_envelope(
        "build-rust-native-dry-run-preview",
        native_preview_result.clone(),
        &preview_errors,
        &preview_warnings,
    );

    if mode == "dry_run" {
        let state_write =
            write_runtime_state(&config, &current_state, &native_preview_result, "dry_run")
                .map_err(|err| {
                    warnings.push(warning(
                        "rust_run_cycle_authority_state_write_failed",
                        Some("paths.runtime_state".to_string()),
                        &format!("Runtime state write failed after dry-run: {err}"),
                    ));
                })
                .ok();
        let mut result = native_preview_result;
        if let Some(map) = result.as_object_mut() {
            map.insert("source".to_string(), json!("rust_run_cycle_authority"));
            map.insert(
                "meta".to_string(),
                json!({
                    "engine": "rust_run_cycle_authority",
                    "mode": "dry_run",
                    "state_write": state_write,
                }),
            );
            map.insert("errors".to_string(), json!(string_list(map.get("errors"))));
            map.insert(
                "warnings".to_string(),
                json!(string_list(map.get("warnings"))),
            );
        }
        return (result, errors, warnings);
    }

    if !preview_errors.is_empty() {
        let finished_at = now_unix_seconds();
        let duration_seconds = started.elapsed().as_secs_f64();
        let result = json!({
            "mode": mode,
            "status": "rust_run_cycle_preview_blocked",
            "source": "rust_run_cycle_authority",
            "started_at": format!("{}", finished_at.saturating_sub(duration_seconds.floor() as u64)),
            "finished_at": format!("{}", finished_at),
            "duration_seconds": ((duration_seconds * 1000.0).round() / 1000.0),
            "routers_processed": 0,
            "router_errors": [],
            "warnings": diag_messages(&warnings),
            "errors": diag_messages(&errors),
            "counts": {
                "csv_rows": 0,
                "nodes": 0
            },
            "csv_changed": native_preview_result.get("csv_changed").cloned().unwrap_or_else(|| json!(false)),
            "network_changed": native_preview_result.get("network_changed").cloned().unwrap_or_else(|| json!(false)),
            "files_changed": native_preview_result.get("files_changed").cloned().unwrap_or_else(|| json!(false)),
            "libreqos_triggered": false,
            "libreqos_exit_code": Value::Null,
            "libreqos_stdout": "",
            "libreqos_stderr": "",
            "diff": {
                "csv": native_preview_result.pointer("/diff/csv").cloned().unwrap_or_else(|| json!({})),
                "network": native_preview_result.pointer("/diff/network").cloned().unwrap_or_else(|| json!({})),
                "rust_native_preview": native_preview_result.pointer("/diff/rust_native_preview").cloned().unwrap_or_else(|| json!({})),
                "rust_run_cycle_authority": {
                    "native_preview": native_preview_envelope,
                    "blocked_before_apply": true,
                }
            },
            "meta": {
                "engine": "rust_run_cycle_authority",
                "native_run_cycle_authority_enabled": native_enabled,
                "native_run_cycle_authority_python_fallback": python_fallback,
                "preview_blocked": true,
            },
            "node_math": {},
            "file_hashes": {},
            "timings": {
                "rust_run_cycle_authority_ms": ((duration_seconds * 1000.0 * 1000.0).round() / 1000.0),
                "rust_native_preview_ms": native_preview_result.pointer("/timings/rust_native_dry_run_preview_ms").cloned().unwrap_or_else(|| json!(0)),
            },
            "timeline": [],
        });
        if let Err(err) = write_runtime_state(&config, &current_state, &result, mode) {
            warnings.push(warning(
                "rust_run_cycle_authority_state_write_failed",
                Some("paths.runtime_state".to_string()),
                &format!("Runtime state write failed: {err}"),
            ));
        }
        return (result, errors, warnings);
    }

    let native_preview = native_preview_result
        .pointer("/diff/rust_native_preview")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let shadow_bundle = native_preview
        .get("shadow_bundle")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let shadow_result = shadow_bundle
        .get("result")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let network_shadow = native_preview
        .get("network_shadow")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let network_result = network_shadow
        .get("result")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let (current_rows, current_csv_source) =
        load_current_rows(&json!({"config": config.clone()}), &mut warnings);
    let current_csv_text = rows_to_csv_text(&current_rows).unwrap_or_else(|_| empty_csv_text());
    let proposed_rows: Vec<Value> = shadow_result
        .get("normalized_rows")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|item| item.is_object())
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let proposed_csv_text = rows_to_csv_text(&proposed_rows).unwrap_or_else(|_| empty_csv_text());
    let current_network_text = network_result
        .get("current_network_text")
        .and_then(Value::as_str)
        .unwrap_or("{}\n")
        .to_string();
    let proposed_network_text = network_result
        .get("network_text")
        .and_then(Value::as_str)
        .unwrap_or("{}\n")
        .to_string();

    let (shadow_result_value, shadow_errors, shadow_warnings) =
        build_rust_sync_engine_shadow_preview_payload(&json!({
            "config": config.clone(),
            "mode": "apply",
            "paths": config.get("paths").cloned().unwrap_or_else(|| json!({})),
            "state": current_state.clone(),
            "current_csv_text": current_csv_text,
            "proposed_csv_text": proposed_csv_text,
            "current_network_text": current_network_text,
            "proposed_network_text": proposed_network_text,
            "files_changed": native_preview_result.get("files_changed").cloned().unwrap_or_else(|| json!(false)),
            "csv_changed": native_preview_result.get("csv_changed").cloned().unwrap_or_else(|| json!(false)),
            "network_changed": native_preview_result.get("network_changed").cloned().unwrap_or_else(|| json!(false)),
            "collector_trust": [],
            "preflight": {"errors": [], "warnings": []},
            "cleanup": {"removed": 0, "queued": 0, "preserved": 0, "candidates": 0},
        }));
    merge_diags(&mut errors, shadow_errors.clone());
    merge_diags(&mut warnings, shadow_warnings.clone());
    let shadow_envelope = response_envelope(
        "build-rust-sync-engine-shadow-preview",
        shadow_result_value.clone(),
        &shadow_errors,
        &shadow_warnings,
    );

    let rust_sync_plan = shadow_result_value
        .get("rust_sync_plan")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let rust_authority_gate = shadow_result_value
        .get("rust_authority_gate")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let rust_policy_shadow_result = shadow_result_value
        .pointer("/rust_policy_shadow/result")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let allow_file_writes = rc
        .get("execute_apply_manifest")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && rc
            .get("allow_rust_file_writes")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let allow_libreqos_apply = rc
        .get("allow_rust_libreqos_apply")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let (transaction_result, transaction_errors, transaction_warnings) =
        execute_apply_transaction_payload(&json!({
            "config": config.clone(),
            "mode": "apply",
            "paths": config.get("paths").cloned().unwrap_or_else(|| json!({})),
            "state": current_state.clone(),
            "current_csv_text": current_csv_text,
            "proposed_csv_text": proposed_csv_text,
            "current_network_text": current_network_text,
            "proposed_network_text": proposed_network_text,
            "files_changed": shadow_result_value.get("files_changed").cloned().unwrap_or_else(|| json!(false)),
            "csv_changed": shadow_result_value.get("csv_changed").cloned().unwrap_or_else(|| json!(false)),
            "network_changed": shadow_result_value.get("network_changed").cloned().unwrap_or_else(|| json!(false)),
            "policy_decision": rust_policy_shadow_result,
            "rust_sync_plan": rust_sync_plan,
            "rust_authority_gate": rust_authority_gate,
            "execute": execute_requested,
            "allow_file_writes": allow_file_writes,
            "allow_libreqos_apply": allow_libreqos_apply,
        }));
    merge_diags(&mut errors, transaction_errors.clone());
    merge_diags(&mut warnings, transaction_warnings.clone());
    let transaction_envelope = response_envelope(
        "execute-apply-transaction",
        transaction_result.clone(),
        &transaction_errors,
        &transaction_warnings,
    );

    let (journal_result, journal_errors, journal_warnings) = build_transaction_journal_payload(
        &json!({
            "config": config.clone(),
            "mode": "apply",
            "paths": config.get("paths").cloned().unwrap_or_else(|| json!({})),
            "rust_apply_manifest": shadow_result_value.get("rust_apply_manifest").cloned().unwrap_or_else(|| json!({})),
            "rust_apply_transaction": transaction_envelope.clone(),
            "rust_sync_plan": rust_sync_plan.clone(),
            "rust_authority_gate": shadow_result_value.get("rust_authority_gate").cloned().unwrap_or_else(|| json!({})),
            "policy_decision": shadow_result_value.pointer("/rust_policy_shadow/result").cloned().unwrap_or_else(|| json!({})),
        }),
    );
    merge_diags(&mut errors, journal_errors.clone());
    merge_diags(&mut warnings, journal_warnings.clone());
    let journal_envelope = response_envelope(
        "build-transaction-journal",
        journal_result.clone(),
        &journal_errors,
        &journal_warnings,
    );

    let append_allowed = rc
        .get("append_transaction_journal")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && rc
            .get("allow_transaction_journal_writes")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let (journal_append_result, journal_append_errors, journal_append_warnings) =
        append_transaction_journal_payload(&json!({
            "config": config.clone(),
            "mode": "apply",
            "paths": config.get("paths").cloned().unwrap_or_else(|| json!({})),
            "rust_apply_manifest": shadow_result_value.get("rust_apply_manifest").cloned().unwrap_or_else(|| json!({})),
            "rust_apply_transaction": transaction_envelope.clone(),
            "rust_sync_plan": rust_sync_plan.clone(),
            "rust_authority_gate": shadow_result_value.get("rust_authority_gate").cloned().unwrap_or_else(|| json!({})),
            "policy_decision": shadow_result_value.pointer("/rust_policy_shadow/result").cloned().unwrap_or_else(|| json!({})),
            "rust_transaction_journal": journal_envelope.clone(),
            "append": execute_requested,
            "allow_journal_write": append_allowed,
            "include_rehearsal_entries": rc.get("include_rehearsal_journal_entries").and_then(Value::as_bool).unwrap_or(false),
            "allow_dry_run_journal": rc.get("allow_dry_run_journal_entries").and_then(Value::as_bool).unwrap_or(false),
        }));
    merge_diags(&mut errors, journal_append_errors.clone());
    merge_diags(&mut warnings, journal_append_warnings.clone());
    let journal_append_envelope = response_envelope(
        "append-transaction-journal",
        journal_append_result.clone(),
        &journal_append_errors,
        &journal_append_warnings,
    );

    let (rollback_result, rollback_errors, rollback_warnings) = build_rollback_manifest_payload(
        &json!({
            "rust_apply_manifest": shadow_result_value.get("rust_apply_manifest").cloned().unwrap_or_else(|| json!({})),
            "rust_apply_transaction": transaction_envelope.clone(),
            "rust_transaction_journal": journal_envelope.clone(),
        }),
    );
    merge_diags(&mut errors, rollback_errors.clone());
    merge_diags(&mut warnings, rollback_warnings.clone());
    let rollback_envelope = response_envelope(
        "build-rollback-manifest",
        rollback_result.clone(),
        &rollback_errors,
        &rollback_warnings,
    );

    let tx_result = transaction_result.clone();
    let file_writes_executed = tx_result
        .get("file_writes_executed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let libreqos_apply_result = tx_result
        .get("libreqos_apply_result")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let libreqos_triggered = tx_result
        .get("libreqos_apply_executed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let libreqos_exit_code = libreqos_apply_result
        .get("exit_code")
        .and_then(Value::as_i64);
    let authority_block = shadow_result_value
        .pointer("/rust_authority_gate/should_block")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let files_changed = shadow_result_value
        .get("files_changed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let csv_changed = shadow_result_value
        .get("csv_changed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let network_changed = shadow_result_value
        .get("network_changed")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let status = if authority_block {
        "rust_authority_blocked"
    } else if !errors.is_empty() {
        "rust_run_cycle_failed"
    } else if libreqos_triggered && libreqos_exit_code.unwrap_or(0) == 0 {
        "success"
    } else if file_writes_executed {
        "success"
    } else if !files_changed {
        "no_changes"
    } else if execute_requested {
        "rust_run_cycle_rehearsal_only"
    } else {
        "rust_run_cycle_preview_only"
    };

    let finished_at = now_unix_seconds();
    let duration_seconds = started.elapsed().as_secs_f64();
    let result = json!({
        "mode": mode,
        "status": status,
        "source": "rust_run_cycle_authority",
        "started_at": format!("{}", finished_at.saturating_sub(duration_seconds.floor() as u64)),
        "finished_at": format!("{}", finished_at),
        "duration_seconds": ((duration_seconds * 1000.0).round() / 1000.0),
        "routers_processed": shadow_result.get("bundle_count").cloned().unwrap_or_else(|| json!(0)),
        "router_errors": [],
        "warnings": diag_messages(&warnings),
        "errors": diag_messages(&errors),
        "counts": {
            "csv_rows": proposed_rows.len(),
            "nodes": network_result.get("node_count").cloned().unwrap_or_else(|| json!(0))
        },
        "csv_changed": csv_changed,
        "network_changed": network_changed,
        "files_changed": files_changed,
        "libreqos_triggered": libreqos_triggered,
        "libreqos_exit_code": libreqos_exit_code,
        "libreqos_stdout": libreqos_apply_result.get("stdout").cloned().unwrap_or_else(|| json!("")),
        "libreqos_stderr": libreqos_apply_result.get("stderr").cloned().unwrap_or_else(|| json!("")),
        "diff": {
            "csv": native_preview_result.pointer("/diff/csv").cloned().unwrap_or_else(|| json!({})),
            "network": native_preview_result.pointer("/diff/network").cloned().unwrap_or_else(|| json!({})),
            "rust_native_preview": native_preview.clone(),
            "rust_sync_engine_shadow_preview": shadow_envelope,
            "rust_core_diff": shadow_result_value.get("rust_core_diff").cloned().unwrap_or_else(|| json!({})),
            "rust_core_validation": shadow_result_value.get("rust_core_validation").cloned().unwrap_or_else(|| json!({})),
            "rust_policy_shadow": shadow_result_value.get("rust_policy_shadow").cloned().unwrap_or_else(|| json!({})),
            "rust_sync_plan": shadow_result_value.get("rust_sync_plan").cloned().unwrap_or_else(|| json!({})),
            "rust_authority_gate": shadow_result_value.get("rust_authority_gate").cloned().unwrap_or_else(|| json!({})),
            "rust_apply_manifest": shadow_result_value.get("rust_apply_manifest").cloned().unwrap_or_else(|| json!({})),
            "rust_apply_transaction": transaction_envelope,
            "rust_transaction_journal": journal_envelope,
            "rust_transaction_journal_append": journal_append_envelope,
            "rust_rollback_manifest": rollback_envelope,
            "rust_run_cycle_authority": {
                "native_preview": native_preview_envelope,
                "current_csv_source": current_csv_source,
                "file_writes_executed": file_writes_executed,
                "append_transaction_journal_allowed": append_allowed,
            }
        },
        "meta": {
            "engine": "rust_run_cycle_authority",
            "current_csv_source": current_csv_source,
            "native_run_cycle_authority_enabled": native_enabled,
            "native_run_cycle_authority_python_fallback": python_fallback,
        },
        "node_math": network_result.get("node_math").cloned().unwrap_or_else(|| json!({})),
        "file_hashes": {
            "current_csv": sha256_text(&current_csv_text),
            "current_network": sha256_text(&current_network_text),
            "proposed_csv": sha256_text(&proposed_csv_text),
            "proposed_network": sha256_text(&proposed_network_text),
        },
        "timings": {
            "rust_run_cycle_authority_ms": ((duration_seconds * 1000.0 * 1000.0).round() / 1000.0),
            "rust_native_preview_ms": native_preview_result.pointer("/timings/rust_native_dry_run_preview_ms").cloned().unwrap_or_else(|| json!(0)),
        },
        "timeline": [],
    });

    if let Err(err) = write_runtime_state(&config, &current_state, &result, mode) {
        warnings.push(warning(
            "rust_run_cycle_authority_state_write_failed",
            Some("paths.runtime_state".to_string()),
            &format!("Runtime state write failed: {err}"),
        ));
    }

    (result, errors, warnings)
}
