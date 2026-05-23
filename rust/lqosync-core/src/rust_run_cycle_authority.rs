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
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
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

fn u64_path(value: &Value, path: &[&str], default: u64) -> u64 {
    let mut current = value;
    for part in path {
        match current.get(*part) {
            Some(next) => current = next,
            None => return default,
        }
    }
    current.as_u64().unwrap_or(default)
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

fn detail_text(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    serde_json::to_string(value).unwrap_or_else(|_| "unavailable".to_string())
}

fn path_from_config(config: &Value, rust_core_key: &str, paths_key: &str, default: &str) -> String {
    let rc = str_path(config, &["rust_core", rust_core_key], "");
    if !rc.trim().is_empty() {
        return rc.to_string();
    }
    let paths = str_path(config, &["paths", paths_key], "");
    if !paths.trim().is_empty() {
        return paths.to_string();
    }
    default.to_string()
}

fn check_entry(
    checks: &mut Vec<Value>,
    failures: &mut Vec<String>,
    name: &str,
    ok: bool,
    detail: impl Into<String>,
) {
    let detail = detail.into();
    checks.push(json!({"name": name, "ok": ok, "detail": detail}));
    if !ok {
        failures.push(format!("{name}: {detail}"));
    }
}

fn quarantine_path(config: &Value) -> PathBuf {
    PathBuf::from(path_from_config(
        config,
        "rust_authority_quarantine_state",
        "rust_authority_quarantine_state",
        "/opt/LQoSync/state/rust_authority_quarantine.json",
    ))
}

fn last_good_snapshot_dir(config: &Value) -> PathBuf {
    PathBuf::from(path_from_config(
        config,
        "rust_authority_last_good_snapshot_dir",
        "rust_authority_last_good_snapshot_dir",
        "/opt/LQoSync/state/rust_authority_last_good",
    ))
}

fn authority_supervisor_preflight(
    config: &Value,
    result_diff: &mut Map<String, Value>,
    errors: &mut Vec<Diagnostic>,
    warnings: &mut Vec<Diagnostic>,
) -> bool {
    if !bool_path(
        config,
        &["rust_core", "full_rust_authority_supervisor_enabled"],
        true,
    ) {
        result_diff.insert(
            "rust_authority_supervisor".to_string(),
            json!({"enabled": false, "status": "not_enabled"}),
        );
        return true;
    }

    if !bool_path(
        config,
        &["rust_core", "require_rust_authority_preflight"],
        false,
    ) {
        result_diff.insert(
            "rust_authority_supervisor".to_string(),
            json!({
                "enabled": true,
                "require_preflight": false,
                "status": "not_required",
            }),
        );
        return true;
    }

    let stamp_path = path_from_config(
        config,
        "rust_authority_preflight_stamp",
        "rust_authority_preflight_stamp",
        "/opt/LQoSync/state/rust_authority_preflight.json",
    );
    let max_age = u64_path(
        config,
        &["rust_core", "rust_authority_preflight_max_age_seconds"],
        900,
    );
    let fail_closed = bool_path(
        config,
        &["rust_core", "fail_closed_on_authority_preflight_failure"],
        true,
    );
    let mut supervisor = json!({
        "enabled": true,
        "require_preflight": true,
        "stamp_path": stamp_path,
        "max_age_seconds": max_age,
        "fail_closed": fail_closed,
        "status": "unknown",
    });

    let failure = match read_json_file(&stamp_path) {
        Ok(stamp) => {
            let created_epoch = stamp
                .get("created_epoch")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let age = now_unix_seconds().saturating_sub(created_epoch);
            if let Some(map) = supervisor.as_object_mut() {
                map.insert(
                    "stamp".to_string(),
                    json!({
                        "status": stamp.get("status").cloned().unwrap_or(Value::Null),
                        "created_at": stamp.get("created_at").cloned().unwrap_or(Value::Null),
                        "created_epoch": stamp.get("created_epoch").cloned().unwrap_or(Value::Null),
                        "self_test_status": stamp.get("self_test_status").cloned().unwrap_or(Value::Null),
                        "git_head": stamp.get("git_head").cloned().unwrap_or(Value::Null),
                    }),
                );
                map.insert("age_seconds".to_string(), json!(age));
            }
            if stamp.get("status").and_then(Value::as_str) != Some("pass") {
                Some(format!(
                    "preflight stamp status is not pass: {}",
                    detail_text(stamp.get("status").unwrap_or(&Value::Null))
                ))
            } else if stamp.get("self_test_status").and_then(Value::as_str) != Some("ok") {
                Some(format!(
                    "preflight stamp self_test_status is not ok: {}",
                    detail_text(stamp.get("self_test_status").unwrap_or(&Value::Null))
                ))
            } else if max_age > 0 && age > max_age {
                Some(format!("preflight stamp is stale: {age}s > {max_age}s"))
            } else {
                None
            }
        }
        Err(err) => Some(err),
    };

    if let Some(reason) = failure {
        if let Some(map) = supervisor.as_object_mut() {
            map.insert("status".to_string(), json!("failed"));
            map.insert("error".to_string(), json!(reason.clone()));
        }
        result_diff.insert("rust_authority_supervisor".to_string(), supervisor);
        let message = format!(
            "Rust authority supervisor preflight failed: {reason}. Run scripts/rust-full-authority-preflight.sh --write-stamp after verifying Rust core."
        );
        if fail_closed {
            errors.push(Diagnostic::error(
                "rust_authority_preflight_required_failed",
                Some("rust_core.require_rust_authority_preflight".to_string()),
                message,
            ));
            return false;
        }
        warnings.push(warning(
            "rust_authority_preflight_warning",
            Some("rust_core.require_rust_authority_preflight".to_string()),
            &message,
        ));
        return true;
    }

    if let Some(map) = supervisor.as_object_mut() {
        map.insert("status".to_string(), json!("ok"));
    }
    result_diff.insert("rust_authority_supervisor".to_string(), supervisor);
    true
}

fn authority_watchdog(
    config: &Value,
    result_diff: &mut Map<String, Value>,
    errors: &mut Vec<Diagnostic>,
    warnings: &mut Vec<Diagnostic>,
) -> bool {
    if !bool_path(
        config,
        &["rust_core", "rust_authority_watchdog_enabled"],
        false,
    ) {
        result_diff.insert(
            "rust_authority_watchdog".to_string(),
            json!({"enabled": false, "status": "not_enabled"}),
        );
        return true;
    }

    let now = now_unix_seconds();
    let fail_closed = bool_path(
        config,
        &["rust_core", "fail_closed_on_authority_watchdog_failure"],
        true,
    );
    let mut checks: Vec<Value> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    let mut watchdog = json!({
        "enabled": true,
        "fail_closed": fail_closed,
        "checks": [],
        "status": "unknown",
    });

    if bool_path(
        config,
        &[
            "rust_core",
            "rust_authority_watchdog_require_fresh_preflight",
        ],
        true,
    ) {
        let stamp_path = path_from_config(
            config,
            "rust_authority_preflight_stamp",
            "rust_authority_preflight_stamp",
            "/opt/LQoSync/state/rust_authority_preflight.json",
        );
        let max_age = u64_path(
            config,
            &[
                "rust_core",
                "rust_authority_watchdog_max_preflight_age_seconds",
            ],
            u64_path(
                config,
                &["rust_core", "rust_authority_preflight_max_age_seconds"],
                900,
            ),
        );
        match read_json_file(&stamp_path) {
            Ok(stamp) => {
                let age = now.saturating_sub(
                    stamp
                        .get("created_epoch")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                );
                if let Some(map) = watchdog.as_object_mut() {
                    map.insert(
                        "preflight_stamp".to_string(),
                        json!({
                            "path": stamp_path,
                            "status": stamp.get("status").cloned().unwrap_or(Value::Null),
                            "self_test_status": stamp.get("self_test_status").cloned().unwrap_or(Value::Null),
                            "age_seconds": age,
                            "max_age_seconds": max_age,
                        }),
                    );
                }
                check_entry(
                    &mut checks,
                    &mut failures,
                    "preflight_stamp_status",
                    stamp.get("status").and_then(Value::as_str) == Some("pass"),
                    detail_text(stamp.get("status").unwrap_or(&Value::Null)),
                );
                check_entry(
                    &mut checks,
                    &mut failures,
                    "preflight_stamp_self_test",
                    stamp.get("self_test_status").and_then(Value::as_str) == Some("ok"),
                    detail_text(stamp.get("self_test_status").unwrap_or(&Value::Null)),
                );
                check_entry(
                    &mut checks,
                    &mut failures,
                    "preflight_stamp_fresh",
                    max_age == 0 || age <= max_age,
                    format!("age={age}s max={max_age}s"),
                );
            }
            Err(err) => {
                if let Some(map) = watchdog.as_object_mut() {
                    map.insert(
                        "preflight_stamp".to_string(),
                        json!({"path": stamp_path, "error": err}),
                    );
                }
                check_entry(
                    &mut checks,
                    &mut failures,
                    "preflight_stamp_readable",
                    false,
                    "unreadable",
                );
            }
        }
    }

    if bool_path(
        config,
        &[
            "rust_core",
            "rust_authority_watchdog_require_recovery_bundle",
        ],
        true,
    ) {
        let root = PathBuf::from(str_path(
            config,
            &["rust_core", "rust_authority_recovery_bundle_dir"],
            "/opt/LQoSync/state/rust_authority_recovery",
        ));
        let latest = fs::read_dir(&root).ok().and_then(|entries| {
            let mut dirs: Vec<PathBuf> = entries
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|path| path.is_dir())
                .collect();
            dirs.sort();
            dirs.pop()
        });
        if let Some(map) = watchdog.as_object_mut() {
            map.insert(
                "recovery_bundle".to_string(),
                json!({
                    "root": root.to_string_lossy().to_string(),
                    "latest": latest.as_ref().map(|p| p.to_string_lossy().to_string()),
                }),
            );
        }
        check_entry(
            &mut checks,
            &mut failures,
            "recovery_bundle_root",
            root.is_dir(),
            root.to_string_lossy().to_string(),
        );
        check_entry(
            &mut checks,
            &mut failures,
            "recovery_bundle_latest",
            latest.is_some(),
            latest
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "none".to_string()),
        );
        check_entry(
            &mut checks,
            &mut failures,
            "recovery_bundle_manifest",
            latest
                .as_ref()
                .map(|p| p.join("MANIFEST.json").is_file())
                .unwrap_or(false),
            latest
                .as_ref()
                .map(|p| p.join("MANIFEST.json").to_string_lossy().to_string())
                .unwrap_or_else(|| "none".to_string()),
        );
    }

    if bool_path(
        config,
        &[
            "rust_core",
            "rust_authority_watchdog_require_transaction_journal_path",
        ],
        true,
    ) {
        let journal = PathBuf::from(str_path(
            config,
            &["paths", "transaction_journal"],
            "/opt/LQoSync/logs/transaction_journal.jsonl",
        ));
        let parent = journal
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("/opt/LQoSync/logs"));
        if let Some(map) = watchdog.as_object_mut() {
            map.insert(
                "transaction_journal".to_string(),
                json!({
                    "path": journal.to_string_lossy().to_string(),
                    "parent": parent.to_string_lossy().to_string(),
                }),
            );
        }
        check_entry(
            &mut checks,
            &mut failures,
            "transaction_journal_parent",
            parent.is_dir(),
            parent.to_string_lossy().to_string(),
        );
        let writable = fs::metadata(&parent)
            .map(|meta| !meta.permissions().readonly())
            .unwrap_or(false);
        check_entry(
            &mut checks,
            &mut failures,
            "transaction_journal_parent_writable",
            parent.is_dir() && writable,
            parent.to_string_lossy().to_string(),
        );
        if bool_path(config, &["rust_core", "append_transaction_journal"], false)
            || bool_path(
                config,
                &["rust_core", "allow_transaction_journal_writes"],
                false,
            )
        {
            check_entry(
                &mut checks,
                &mut failures,
                "transaction_journal_authority_flags",
                bool_path(config, &["rust_core", "append_transaction_journal"], false)
                    && bool_path(
                        config,
                        &["rust_core", "allow_transaction_journal_writes"],
                        false,
                    ),
                "append_transaction_journal and allow_transaction_journal_writes must both be true",
            );
        }
    }

    if let Some(map) = watchdog.as_object_mut() {
        map.insert("checks".to_string(), Value::Array(checks));
        map.insert("failure_count".to_string(), json!(failures.len()));
        map.insert("failures".to_string(), json!(failures.clone()));
        map.insert(
            "status".to_string(),
            json!(if failures.is_empty() { "ok" } else { "failed" }),
        );
    }
    result_diff.insert("rust_authority_watchdog".to_string(), watchdog);

    if failures.is_empty() {
        return true;
    }
    let message = format!(
        "Rust authority watchdog failed: {}",
        failures[..failures.len().min(5)].join("; ")
    );
    if fail_closed {
        errors.push(Diagnostic::error(
            "rust_authority_watchdog_required_failed",
            Some("rust_core.rust_authority_watchdog_enabled".to_string()),
            message,
        ));
        return false;
    }
    warnings.push(warning(
        "rust_authority_watchdog_warning",
        Some("rust_core.rust_authority_watchdog_enabled".to_string()),
        &message,
    ));
    true
}

fn authority_live_stable_gate(
    config: &Value,
    state: &Value,
    result_diff: &mut Map<String, Value>,
    errors: &mut Vec<Diagnostic>,
    warnings: &mut Vec<Diagnostic>,
) -> bool {
    if !bool_path(
        config,
        &["rust_core", "rust_live_stable_candidate_enabled"],
        false,
    ) {
        result_diff.insert(
            "rust_live_stable_gate".to_string(),
            json!({"enabled": false, "status": "not_enabled"}),
        );
        return true;
    }

    let fail_closed = bool_path(config, &["rust_core", "rust_live_stable_fail_closed"], true);
    let mut checks: Vec<Value> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    let mut gate = json!({
        "enabled": true,
        "fail_closed": fail_closed,
        "checks": [],
        "status": "unknown",
    });

    let qpath = quarantine_path(config);
    let quarantine_active = if qpath.is_file() {
        match read_json_file(&qpath.to_string_lossy()) {
            Ok(qdata) => {
                let active = qdata
                    .get("active")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if let Some(map) = gate.as_object_mut() {
                    map.insert(
                        "quarantine".to_string(),
                        json!({
                            "path": qpath.to_string_lossy().to_string(),
                            "active": active,
                            "status": qdata.get("status").cloned().unwrap_or(Value::Null),
                            "created_at": qdata.get("created_at").cloned().unwrap_or(Value::Null),
                        }),
                    );
                }
                active
            }
            Err(err) => {
                if let Some(map) = gate.as_object_mut() {
                    map.insert(
                        "quarantine".to_string(),
                        json!({"path": qpath.to_string_lossy().to_string(), "active": true, "error": err}),
                    );
                }
                true
            }
        }
    } else {
        if let Some(map) = gate.as_object_mut() {
            map.insert(
                "quarantine".to_string(),
                json!({"path": qpath.to_string_lossy().to_string(), "active": false, "status": "missing_ok"}),
            );
        }
        false
    };
    check_entry(
        &mut checks,
        &mut failures,
        "quarantine_clear",
        !quarantine_active,
        detail_text(gate.pointer("/quarantine").unwrap_or(&Value::Null)),
    );

    if bool_path(
        config,
        &["rust_core", "rust_live_stable_require_watchdog"],
        true,
    ) {
        let watchdog_ok = result_diff
            .get("rust_authority_watchdog")
            .and_then(|v| v.get("status"))
            .and_then(Value::as_str)
            == Some("ok");
        check_entry(
            &mut checks,
            &mut failures,
            "watchdog_ok",
            watchdog_ok,
            detail_text(
                result_diff
                    .get("rust_authority_watchdog")
                    .and_then(|v| v.get("status"))
                    .unwrap_or(&Value::Null),
            ),
        );
    }

    if bool_path(
        config,
        &["rust_core", "rust_live_stable_require_recovery_bundle"],
        true,
    ) {
        let root = PathBuf::from(str_path(
            config,
            &["rust_core", "rust_authority_recovery_bundle_dir"],
            "/opt/LQoSync/state/rust_authority_recovery",
        ));
        let latest = fs::read_dir(&root).ok().and_then(|entries| {
            let mut dirs: Vec<PathBuf> = entries
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|path| path.is_dir())
                .collect();
            dirs.sort();
            dirs.pop()
        });
        if let Some(map) = gate.as_object_mut() {
            map.insert(
                "recovery_bundle".to_string(),
                json!({
                    "root": root.to_string_lossy().to_string(),
                    "latest": latest.as_ref().map(|p| p.to_string_lossy().to_string()),
                }),
            );
        }
        check_entry(
            &mut checks,
            &mut failures,
            "recovery_bundle_available",
            latest
                .as_ref()
                .map(|p| p.join("MANIFEST.json").is_file())
                .unwrap_or(false),
            latest
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "none".to_string()),
        );
    }

    if bool_path(
        config,
        &["rust_core", "rust_live_stable_require_last_good_snapshot"],
        false,
    ) {
        let root = last_good_snapshot_dir(config);
        let latest = fs::read_dir(&root).ok().and_then(|entries| {
            let mut dirs: Vec<PathBuf> = entries
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|path| path.is_dir())
                .collect();
            dirs.sort();
            dirs.pop()
        });
        if let Some(map) = gate.as_object_mut() {
            map.insert(
                "last_good_snapshot".to_string(),
                json!({
                    "root": root.to_string_lossy().to_string(),
                    "latest": latest.as_ref().map(|p| p.to_string_lossy().to_string()),
                }),
            );
        }
        check_entry(
            &mut checks,
            &mut failures,
            "last_good_snapshot_available",
            latest
                .as_ref()
                .map(|p| p.join("MANIFEST.json").is_file())
                .unwrap_or(false),
            latest
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "none".to_string()),
        );
    }

    let max_failures = u64_path(
        config,
        &["rust_core", "rust_live_stable_max_recent_failures"],
        0,
    ) as usize;
    let recent_failures = state
        .get("rust_authority_recent_failures")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(map) = gate.as_object_mut() {
        map.insert(
            "recent_failures".to_string(),
            json!({
                "count": recent_failures.len(),
                "max": max_failures,
                "items": recent_failures.iter().rev().take(5).cloned().collect::<Vec<Value>>(),
            }),
        );
    }
    check_entry(
        &mut checks,
        &mut failures,
        "recent_failure_budget",
        recent_failures.len() <= max_failures,
        format!("count={} max={max_failures}", recent_failures.len()),
    );

    if let Some(map) = gate.as_object_mut() {
        map.insert("checks".to_string(), Value::Array(checks));
        map.insert("failure_count".to_string(), json!(failures.len()));
        map.insert("failures".to_string(), json!(failures.clone()));
        map.insert(
            "status".to_string(),
            json!(if failures.is_empty() { "ok" } else { "failed" }),
        );
    }
    result_diff.insert("rust_live_stable_gate".to_string(), gate);

    if failures.is_empty() {
        return true;
    }
    let message = format!(
        "Rust live-stable gate failed: {}",
        failures[..failures.len().min(6)].join("; ")
    );
    if fail_closed {
        errors.push(Diagnostic::error(
            "rust_live_stable_gate_failed",
            Some("rust_core.rust_live_stable_candidate_enabled".to_string()),
            message,
        ));
        return false;
    }
    warnings.push(warning(
        "rust_live_stable_gate_warning",
        Some("rust_core.rust_live_stable_candidate_enabled".to_string()),
        &message,
    ));
    true
}

fn rust_set_and_forget_gate(
    config: &Value,
    result_diff: &mut Map<String, Value>,
    errors: &mut Vec<Diagnostic>,
    warnings: &mut Vec<Diagnostic>,
) -> bool {
    if !bool_path(
        config,
        &["rust_core", "rust_set_and_forget_candidate_enabled"],
        false,
    ) {
        result_diff.insert(
            "rust_set_and_forget_gate".to_string(),
            json!({"enabled": false, "status": "not_enabled"}),
        );
        return true;
    }

    let fail_closed = bool_path(
        config,
        &["rust_core", "rust_set_and_forget_fail_closed"],
        true,
    );
    let evidence_path = PathBuf::from(str_path(
        config,
        &["rust_core", "rust_set_and_forget_readiness_evidence"],
        "/opt/LQoSync/state/rust_set_and_forget_readiness.json",
    ));
    let mut gate = json!({
        "enabled": true,
        "fail_closed": fail_closed,
        "checks": [],
        "status": "unknown",
    });
    let mut checks: Vec<Value> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    let now = now_unix_seconds();
    let evidence = match read_json_file(&evidence_path.to_string_lossy()) {
        Ok(evidence) => {
            let age = now.saturating_sub(
                evidence
                    .get("created_epoch")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
            );
            let max_age = u64_path(
                config,
                &["rust_core", "rust_set_and_forget_max_evidence_age_seconds"],
                1800,
            );
            if let Some(map) = gate.as_object_mut() {
                map.insert(
                    "evidence".to_string(),
                    json!({
                        "path": evidence_path.to_string_lossy().to_string(),
                        "status": evidence.get("status").cloned().unwrap_or(Value::Null),
                        "created_at": evidence.get("created_at").cloned().unwrap_or(Value::Null),
                    }),
                );
            }
            check_entry(
                &mut checks,
                &mut failures,
                "readiness_evidence_pass",
                evidence.get("status").and_then(Value::as_str) == Some("pass"),
                detail_text(evidence.get("status").unwrap_or(&Value::Null)),
            );
            check_entry(
                &mut checks,
                &mut failures,
                "readiness_evidence_fresh",
                max_age == 0 || age <= max_age,
                format!("age={age} max={max_age}"),
            );
            evidence
        }
        Err(err) => {
            if let Some(map) = gate.as_object_mut() {
                map.insert(
                    "evidence".to_string(),
                    json!({"path": evidence_path.to_string_lossy().to_string(), "error": err}),
                );
            }
            check_entry(
                &mut checks,
                &mut failures,
                "readiness_evidence_readable",
                false,
                "unreadable",
            );
            json!({})
        }
    };

    let qpath = quarantine_path(config);
    if qpath.is_file() {
        match read_json_file(&qpath.to_string_lossy()) {
            Ok(qdata) => {
                let active = qdata
                    .get("active")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if let Some(map) = gate.as_object_mut() {
                    map.insert(
                        "quarantine".to_string(),
                        json!({
                            "path": qpath.to_string_lossy().to_string(),
                            "active": active,
                            "status": qdata.get("status").cloned().unwrap_or(Value::Null),
                        }),
                    );
                }
                check_entry(
                    &mut checks,
                    &mut failures,
                    "quarantine_clear",
                    !active,
                    detail_text(gate.pointer("/quarantine").unwrap_or(&Value::Null)),
                );
            }
            Err(err) => {
                check_entry(
                    &mut checks,
                    &mut failures,
                    "quarantine_readable",
                    false,
                    err,
                );
            }
        }
    } else {
        if let Some(map) = gate.as_object_mut() {
            map.insert(
                "quarantine".to_string(),
                json!({"path": qpath.to_string_lossy().to_string(), "active": false, "status": "missing_ok"}),
            );
        }
        check_entry(
            &mut checks,
            &mut failures,
            "quarantine_clear",
            true,
            "missing_ok",
        );
    }

    let evidence_checks = evidence
        .get("checks")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let required_checks = [
        (
            "rust_set_and_forget_require_live_soak_monitor",
            "live_soak_monitor",
        ),
        ("rust_set_and_forget_require_journal_audit", "journal_audit"),
        (
            "rust_set_and_forget_require_rollback_drill",
            "rollback_drill",
        ),
        (
            "rust_set_and_forget_require_last_good_snapshot",
            "last_good_snapshot",
        ),
    ];
    for (flag, key) in required_checks {
        if bool_path(config, &["rust_core", flag], true) {
            let item = evidence_checks
                .get(key)
                .cloned()
                .unwrap_or_else(|| json!({}));
            check_entry(
                &mut checks,
                &mut failures,
                &format!("evidence_{key}"),
                item.get("ok").and_then(Value::as_bool) == Some(true),
                detail_text(&item),
            );
        }
    }

    if let Some(map) = gate.as_object_mut() {
        map.insert("checks".to_string(), Value::Array(checks));
        map.insert("failure_count".to_string(), json!(failures.len()));
        map.insert("failures".to_string(), json!(failures.clone()));
        map.insert(
            "status".to_string(),
            json!(if failures.is_empty() { "ok" } else { "failed" }),
        );
    }
    result_diff.insert("rust_set_and_forget_gate".to_string(), gate);

    if failures.is_empty() {
        return true;
    }
    let message = format!(
        "Rust set-and-forget gate failed: {}",
        failures[..failures.len().min(6)].join("; ")
    );
    if fail_closed {
        errors.push(Diagnostic::error(
            "rust_set_and_forget_gate_failed",
            Some("rust_core.rust_set_and_forget_candidate_enabled".to_string()),
            message,
        ));
        return false;
    }
    warnings.push(warning(
        "rust_set_and_forget_gate_warning",
        Some("rust_core.rust_set_and_forget_candidate_enabled".to_string()),
        &message,
    ));
    true
}

fn mark_quarantine(
    config: &Value,
    status: &str,
    result: &Value,
    details: Value,
) -> Result<Value, String> {
    if !bool_path(
        config,
        &["rust_core", "rust_authority_quarantine_enabled"],
        false,
    ) {
        return Ok(json!({"active": false, "status": "not_enabled"}));
    }
    if !bool_path(
        config,
        &["rust_core", "rust_authority_auto_quarantine_on_failure"],
        true,
    ) {
        return Ok(json!({"active": false, "status": "auto_quarantine_disabled"}));
    }
    if let Some(statuses) = config
        .get("rust_core")
        .and_then(|v| v.get("rust_authority_failure_quarantine_statuses"))
        .and_then(Value::as_array)
    {
        let allowed = statuses
            .iter()
            .filter_map(Value::as_str)
            .any(|value| value == status);
        if !statuses.is_empty() && !allowed {
            return Ok(json!({"active": false, "status": "status_not_in_allowlist"}));
        }
    }

    let path = quarantine_path(config);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create quarantine dir: {err}"))?;
    }

    let payload = json!({
        "schema": "lqosync.rust_authority_quarantine.v1",
        "active": true,
        "status": status,
        "created_epoch": now_unix_seconds(),
        "created_at": format!("{}", now_unix_seconds()),
        "reason": "critical Rust authority failure; live-stable mutation blocked until operator review",
        "last_error": status,
        "result_status": result.get("status").cloned().unwrap_or(Value::Null),
        "errors": result.get("errors").cloned().unwrap_or_else(|| json!([])),
        "warnings": result.get("warnings").cloned().unwrap_or_else(|| json!([])),
        "details": details,
    });
    let text = serde_json::to_string_pretty(&payload)
        .map(|mut text| {
            text.push('\n');
            text
        })
        .map_err(|err| format!("encode quarantine payload: {err}"))?;
    fs::write(&path, text).map_err(|err| format!("write quarantine payload: {err}"))?;

    Ok(json!({
        "active": true,
        "path": path.to_string_lossy().to_string(),
        "status": status,
        "schema": "lqosync.rust_authority_quarantine.v1",
    }))
}

fn record_last_good_snapshot(config: &Value, result: &Value) -> Result<Value, String> {
    if !bool_path(
        config,
        &["rust_core", "rust_live_stable_candidate_enabled"],
        false,
    ) {
        return Ok(json!({"status": "not_enabled"}));
    }

    let root = last_good_snapshot_dir(config);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| format!("{}_{}", duration.as_secs(), duration.subsec_millis()))
        .unwrap_or_else(|_| now_unix_seconds().to_string());
    let dir = root.join(stamp);
    fs::create_dir_all(&dir).map_err(|err| format!("create last-good snapshot dir: {err}"))?;

    let paths = config.get("paths").cloned().unwrap_or_else(|| json!({}));
    let mut manifest = json!({
        "schema": "lqosync.rust_authority_last_good.v1",
        "created_epoch": now_unix_seconds(),
        "created_at": format!("{}", now_unix_seconds()),
        "status": result.get("status").cloned().unwrap_or(Value::Null),
        "files_changed": result.get("files_changed").cloned().unwrap_or_else(|| json!(false)),
        "libreqos_triggered": result.get("libreqos_triggered").cloned().unwrap_or_else(|| json!(false)),
        "libreqos_exit_code": result.get("libreqos_exit_code").cloned().unwrap_or(Value::Null),
        "file_hashes": result.get("file_hashes").cloned().unwrap_or_else(|| json!({})),
        "paths": {
            "shaped_devices_csv": paths.get("shaped_devices_csv").cloned().unwrap_or(Value::Null),
            "network_json": paths.get("network_json").cloned().unwrap_or(Value::Null),
            "runtime_state": paths.get("runtime_state").cloned().unwrap_or(Value::Null),
            "transaction_journal": paths.get("transaction_journal").cloned().unwrap_or(Value::Null),
        }
    });
    let mut included_files: Vec<Value> = Vec::new();
    let copies = [
        ("shaped_devices_csv", "ShapedDevices.csv"),
        ("network_json", "network.json"),
        ("runtime_state", "runtime_state.json"),
    ];
    for (key, target_name) in copies {
        if let Some(source) = paths.get(key).and_then(Value::as_str) {
            let source_path = Path::new(source);
            if source_path.is_file() {
                let target = dir.join(target_name);
                fs::copy(source_path, &target).map_err(|err| {
                    format!(
                        "copy {} to last-good snapshot: {err}",
                        source_path.display()
                    )
                })?;
                included_files.push(json!(target_name));
            }
        }
    }
    if let Some(map) = manifest.as_object_mut() {
        map.insert("included_files".to_string(), Value::Array(included_files));
    }

    let manifest_path = dir.join("MANIFEST.json");
    let text = serde_json::to_string_pretty(&manifest)
        .map(|mut text| {
            text.push('\n');
            text
        })
        .map_err(|err| format!("encode last-good manifest: {err}"))?;
    fs::write(&manifest_path, text).map_err(|err| format!("write last-good manifest: {err}"))?;

    Ok(json!({
        "path": dir.to_string_lossy().to_string(),
        "status": "created",
        "schema": "lqosync.rust_authority_last_good.v1",
    }))
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
/// read + shadow + sync-engine + apply pipeline. Python fallback execution has
/// been retired from this authority boundary.
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
        return (
            json!({
                "status": "rust_run_cycle_authority_not_enabled",
                "mode": mode,
                "source": "rust_run_cycle_authority",
                "errors": ["Rust native run-cycle authority is disabled. Python fallback execution has been retired from this boundary."],
                "warnings": [],
            }),
            vec![Diagnostic::error(
                "rust_run_cycle_authority_disabled",
                Some("rust_core.native_run_cycle_authority_enabled".to_string()),
                "Rust native run-cycle authority is disabled. Python fallback execution has been retired from this boundary.",
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
    let proposed_csv_text = native_preview_result
        .get("proposed_csv_text")
        .and_then(Value::as_str)
        .map(|text| text.to_string())
        .unwrap_or_else(|| rows_to_csv_text(&proposed_rows).unwrap_or_else(|_| empty_csv_text()));
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
    let rust_apply_manifest_result = shadow_result_value
        .pointer("/rust_apply_manifest/result")
        .cloned()
        .unwrap_or_else(|| json!({}));
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
    let apply_required = rust_apply_manifest_result
        .get("apply_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let file_hashes = json!({
        "current_csv": sha256_text(&current_csv_text),
        "current_network": sha256_text(&current_network_text),
        "proposed_csv": sha256_text(&proposed_csv_text),
        "proposed_network": sha256_text(&proposed_network_text),
    });

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
        .unwrap_or(false)
        && apply_required;
    let full_rust_authority_required = bool_path(
        &config,
        &["rust_core", "full_rust_backend_authority"],
        false,
    ) || bool_path(
        &config,
        &["rust_core", "fail_closed_without_rust_authority"],
        false,
    ) || matches!(
        str_path(&config, &["rust_core", "transaction_authority"], ""),
        "rust_full_authoritative" | "rust_apply_authoritative"
    );
    let mut authority_extra_diff = Map::new();
    if full_rust_authority_required {
        authority_extra_diff.insert(
            "rust_full_authority_lock".to_string(),
            json!({
                "enabled": true,
                "rust_file_write_authority": allow_file_writes,
                "rust_apply_authority": allow_libreqos_apply,
                "python_mutation_fallback_allowed": false,
                "transaction_authority": rc.get("transaction_authority").cloned().unwrap_or(Value::Null),
            }),
        );
    }
    let build_blocked_result = |status: &str,
                                errors_ref: &[Diagnostic],
                                warnings_ref: &[Diagnostic],
                                extra_diff: Map<String, Value>|
     -> Value {
        let finished_at = now_unix_seconds();
        let duration_seconds = started.elapsed().as_secs_f64();
        let mut diff = Map::new();
        diff.insert(
            "csv".to_string(),
            native_preview_result
                .pointer("/diff/csv")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        diff.insert(
            "network".to_string(),
            native_preview_result
                .pointer("/diff/network")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        diff.insert("rust_native_preview".to_string(), native_preview.clone());
        diff.insert(
            "rust_sync_engine_shadow_preview".to_string(),
            shadow_envelope.clone(),
        );
        diff.insert(
            "rust_core_diff".to_string(),
            shadow_result_value
                .get("rust_core_diff")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        diff.insert(
            "rust_core_validation".to_string(),
            shadow_result_value
                .get("rust_core_validation")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        diff.insert(
            "rust_policy_shadow".to_string(),
            shadow_result_value
                .get("rust_policy_shadow")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        diff.insert(
            "rust_sync_plan".to_string(),
            shadow_result_value
                .get("rust_sync_plan")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        diff.insert(
            "rust_authority_gate".to_string(),
            shadow_result_value
                .get("rust_authority_gate")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        diff.insert(
            "rust_apply_manifest".to_string(),
            shadow_result_value
                .get("rust_apply_manifest")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        diff.insert(
            "rust_run_cycle_authority".to_string(),
            json!({
                "native_preview": native_preview_envelope,
                "current_csv_source": current_csv_source,
                "blocked_before_apply": true,
            }),
        );
        for (key, value) in extra_diff {
            diff.insert(key, value);
        }

        json!({
            "mode": mode,
            "status": status,
            "source": "rust_run_cycle_authority",
            "started_at": format!("{}", finished_at.saturating_sub(duration_seconds.floor() as u64)),
            "finished_at": format!("{}", finished_at),
            "duration_seconds": ((duration_seconds * 1000.0).round() / 1000.0),
            "routers_processed": shadow_result.get("bundle_count").cloned().unwrap_or_else(|| json!(0)),
            "router_errors": [],
            "warnings": diag_messages(warnings_ref),
            "errors": diag_messages(errors_ref),
            "counts": {
                "csv_rows": proposed_rows.len(),
                "nodes": network_result.get("node_count").cloned().unwrap_or_else(|| json!(0))
            },
            "csv_changed": csv_changed,
            "network_changed": network_changed,
            "files_changed": files_changed,
            "libreqos_triggered": false,
            "libreqos_exit_code": Value::Null,
            "libreqos_stdout": "",
            "libreqos_stderr": "",
            "diff": Value::Object(diff),
            "meta": {
                "engine": "rust_run_cycle_authority",
                "current_csv_source": current_csv_source,
                "native_run_cycle_authority_enabled": native_enabled,
                "native_run_cycle_authority_python_fallback": python_fallback,
            },
            "node_math": network_result.get("node_math").cloned().unwrap_or_else(|| json!({})),
            "file_hashes": file_hashes.clone(),
            "timings": {
                "rust_run_cycle_authority_ms": ((duration_seconds * 1000.0 * 1000.0).round() / 1000.0),
                "rust_native_preview_ms": native_preview_result.pointer("/timings/rust_native_dry_run_preview_ms").cloned().unwrap_or_else(|| json!(0)),
            },
            "timeline": [],
        })
    };

    if full_rust_authority_required && files_changed && !allow_file_writes {
        errors.push(Diagnostic::error(
            "rust_full_authority_missing_file_write_flags",
            Some("rust_core.allow_rust_file_writes".to_string()),
            "Rust full authority lock: file changes require execute_apply_manifest=true and allow_rust_file_writes=true.",
        ));
        let mut result = build_blocked_result(
            "rust_full_authority_missing_file_write_flags",
            &errors,
            &warnings,
            authority_extra_diff.clone(),
        );
        match mark_quarantine(
            &config,
            "rust_full_authority_missing_file_write_flags",
            &result,
            authority_extra_diff
                .get("rust_full_authority_lock")
                .cloned()
                .unwrap_or_else(|| json!({})),
        ) {
            Ok(quarantine) => {
                if let Some(diff) = result.get_mut("diff").and_then(Value::as_object_mut) {
                    diff.insert("rust_authority_quarantine".to_string(), quarantine);
                }
            }
            Err(err) => warnings.push(warning(
                "rust_authority_quarantine_write_failed",
                Some("rust_core.rust_authority_quarantine_state".to_string()),
                &format!("Failed to write Rust authority quarantine marker: {err}"),
            )),
        }
        let _ = write_runtime_state(&config, &current_state, &result, mode);
        return (result, errors, warnings);
    }

    if full_rust_authority_required && apply_required && !allow_libreqos_apply {
        errors.push(Diagnostic::error(
            "rust_full_authority_missing_apply_flag",
            Some("rust_core.allow_rust_libreqos_apply".to_string()),
            "Rust full authority lock: LibreQoS apply requires allow_rust_libreqos_apply=true.",
        ));
        let mut result = build_blocked_result(
            "rust_full_authority_missing_apply_flag",
            &errors,
            &warnings,
            authority_extra_diff.clone(),
        );
        match mark_quarantine(
            &config,
            "rust_full_authority_missing_apply_flag",
            &result,
            authority_extra_diff
                .get("rust_full_authority_lock")
                .cloned()
                .unwrap_or_else(|| json!({})),
        ) {
            Ok(quarantine) => {
                if let Some(diff) = result.get_mut("diff").and_then(Value::as_object_mut) {
                    diff.insert("rust_authority_quarantine".to_string(), quarantine);
                }
            }
            Err(err) => warnings.push(warning(
                "rust_authority_quarantine_write_failed",
                Some("rust_core.rust_authority_quarantine_state".to_string()),
                &format!("Failed to write Rust authority quarantine marker: {err}"),
            )),
        }
        let _ = write_runtime_state(&config, &current_state, &result, mode);
        return (result, errors, warnings);
    }

    if full_rust_authority_required {
        if !authority_supervisor_preflight(
            &config,
            &mut authority_extra_diff,
            &mut errors,
            &mut warnings,
        ) {
            let mut result = build_blocked_result(
                "rust_authority_preflight_required_failed",
                &errors,
                &warnings,
                authority_extra_diff.clone(),
            );
            match mark_quarantine(
                &config,
                "rust_authority_preflight_required_failed",
                &result,
                authority_extra_diff
                    .get("rust_authority_supervisor")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            ) {
                Ok(quarantine) => {
                    if let Some(diff) = result.get_mut("diff").and_then(Value::as_object_mut) {
                        diff.insert("rust_authority_quarantine".to_string(), quarantine);
                    }
                }
                Err(err) => warnings.push(warning(
                    "rust_authority_quarantine_write_failed",
                    Some("rust_core.rust_authority_quarantine_state".to_string()),
                    &format!("Failed to write Rust authority quarantine marker: {err}"),
                )),
            }
            let _ = write_runtime_state(&config, &current_state, &result, mode);
            return (result, errors, warnings);
        }

        if !authority_watchdog(
            &config,
            &mut authority_extra_diff,
            &mut errors,
            &mut warnings,
        ) {
            let mut result = build_blocked_result(
                "rust_authority_watchdog_required_failed",
                &errors,
                &warnings,
                authority_extra_diff.clone(),
            );
            match mark_quarantine(
                &config,
                "rust_authority_watchdog_required_failed",
                &result,
                authority_extra_diff
                    .get("rust_authority_watchdog")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            ) {
                Ok(quarantine) => {
                    if let Some(diff) = result.get_mut("diff").and_then(Value::as_object_mut) {
                        diff.insert("rust_authority_quarantine".to_string(), quarantine);
                    }
                }
                Err(err) => warnings.push(warning(
                    "rust_authority_quarantine_write_failed",
                    Some("rust_core.rust_authority_quarantine_state".to_string()),
                    &format!("Failed to write Rust authority quarantine marker: {err}"),
                )),
            }
            let _ = write_runtime_state(&config, &current_state, &result, mode);
            return (result, errors, warnings);
        }

        if !authority_live_stable_gate(
            &config,
            &current_state,
            &mut authority_extra_diff,
            &mut errors,
            &mut warnings,
        ) {
            let result = build_blocked_result(
                "rust_live_stable_gate_failed",
                &errors,
                &warnings,
                authority_extra_diff.clone(),
            );
            let _ = write_runtime_state(&config, &current_state, &result, mode);
            return (result, errors, warnings);
        }

        if !rust_set_and_forget_gate(
            &config,
            &mut authority_extra_diff,
            &mut errors,
            &mut warnings,
        ) {
            let mut result = build_blocked_result(
                "rust_set_and_forget_gate_failed",
                &errors,
                &warnings,
                authority_extra_diff.clone(),
            );
            match mark_quarantine(
                &config,
                "rust_set_and_forget_gate_failed",
                &result,
                authority_extra_diff
                    .get("rust_set_and_forget_gate")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            ) {
                Ok(quarantine) => {
                    if let Some(diff) = result.get_mut("diff").and_then(Value::as_object_mut) {
                        diff.insert("rust_authority_quarantine".to_string(), quarantine);
                    }
                }
                Err(err) => warnings.push(warning(
                    "rust_authority_quarantine_write_failed",
                    Some("rust_core.rust_authority_quarantine_state".to_string()),
                    &format!("Failed to write Rust authority quarantine marker: {err}"),
                )),
            }
            let _ = write_runtime_state(&config, &current_state, &result, mode);
            return (result, errors, warnings);
        }
    }

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
    let mut status = if authority_block {
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
    let mut quarantine_status: Option<&str> = None;
    if full_rust_authority_required && execute_requested && files_changed && !file_writes_executed {
        errors.push(Diagnostic::error(
            "rust_full_authority_file_write_not_executed",
            Some("rust_core.allow_rust_file_writes".to_string()),
            "Rust full authority lock: Rust transaction did not execute file writes; Python fallback is disabled.",
        ));
        status = "rust_full_authority_file_write_not_executed";
        quarantine_status = Some(status);
    } else if full_rust_authority_required
        && execute_requested
        && apply_required
        && !libreqos_triggered
    {
        errors.push(Diagnostic::error(
            "rust_full_authority_libreqos_apply_not_executed",
            Some("rust_core.allow_rust_libreqos_apply".to_string()),
            "Rust full authority lock: Rust did not execute LibreQoS apply; Python fallback is disabled.",
        ));
        status = "rust_full_authority_libreqos_apply_not_executed";
        quarantine_status = Some(status);
    }

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
        "file_hashes": file_hashes,
        "timings": {
            "rust_run_cycle_authority_ms": ((duration_seconds * 1000.0 * 1000.0).round() / 1000.0),
            "rust_native_preview_ms": native_preview_result.pointer("/timings/rust_native_dry_run_preview_ms").cloned().unwrap_or_else(|| json!(0)),
        },
        "timeline": [],
    });
    let mut result = result;
    if let Some(diff) = result.get_mut("diff").and_then(Value::as_object_mut) {
        for (key, value) in authority_extra_diff {
            diff.insert(key, value);
        }
    }
    if let Some(status) = quarantine_status {
        match mark_quarantine(
            &config,
            status,
            &result,
            result
                .pointer("/diff/rust_full_authority_lock")
                .cloned()
                .unwrap_or_else(|| json!({})),
        ) {
            Ok(quarantine) => {
                if let Some(diff) = result.get_mut("diff").and_then(Value::as_object_mut) {
                    diff.insert("rust_authority_quarantine".to_string(), quarantine);
                }
            }
            Err(err) => warnings.push(warning(
                "rust_authority_quarantine_write_failed",
                Some("rust_core.rust_authority_quarantine_state".to_string()),
                &format!("Failed to write Rust authority quarantine marker: {err}"),
            )),
        }
    } else if status == "success" {
        match record_last_good_snapshot(&config, &result) {
            Ok(snapshot) => {
                if let Some(diff) = result.get_mut("diff").and_then(Value::as_object_mut) {
                    diff.insert("rust_authority_last_good_snapshot".to_string(), snapshot);
                }
            }
            Err(err) => warnings.push(warning(
                "rust_authority_last_good_snapshot_failed",
                Some("rust_core.rust_authority_last_good_snapshot_dir".to_string()),
                &format!("Failed to create Rust authority last-good snapshot: {err}"),
            )),
        }
    }

    if let Err(err) = write_runtime_state(&config, &current_state, &result, mode) {
        warnings.push(warning(
            "rust_run_cycle_authority_state_write_failed",
            Some("paths.runtime_state".to_string()),
            &format!("Runtime state write failed: {err}"),
        ));
    }

    (result, errors, warnings)
}
