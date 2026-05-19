use crate::atomic_state::{atomic_write_text, sha256_text};
use crate::protocol::Diagnostic;
use crate::transaction_history::build_rollback_from_journal_payload;
use crate::transaction_journal::build_rollback_manifest_payload;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

fn str_field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn bool_field(value: &Value, key: &str, default_value: bool) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(default_value)
}

fn sha256_file_text(path: &Path) -> anyhow::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    Ok(Some(sha256_text(&text)))
}

fn resolve_rollback_manifest(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    if let Some(manifest) = payload.get("rollback_manifest") {
        let resolved = manifest.get("result").cloned().unwrap_or_else(|| manifest.clone());
        if resolved.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
            return (resolved, Vec::new(), Vec::new());
        }
    }
    if payload.get("journal_id").and_then(Value::as_str).unwrap_or("").trim().is_empty()
        && payload.get("manifest_id").and_then(Value::as_str).unwrap_or("").trim().is_empty()
    {
        return build_rollback_manifest_payload(payload);
    }
    build_rollback_from_journal_payload(payload)
}

/// Execute a rollback manifest in a gated, opt-in manner.
///
/// Defaults to rehearsal-only. Actual restore requires:
/// - execute=true
/// - allow_rollback_file_writes=true
/// - confirmation="CONFIRM_ROLLBACK"
/// - mode != dry_run
/// - rollback manifest status=rollback_available
pub fn execute_rollback_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let (rollback_manifest, mut errors, mut warnings) = resolve_rollback_manifest(payload);
    let execute = bool_field(payload, "execute", false);
    let allow_file_writes = bool_field(payload, "allow_rollback_file_writes", false)
        || bool_field(payload, "allow_file_writes", false);
    let allow_checksum_mismatch = bool_field(payload, "allow_checksum_mismatch", false);
    let dry_run = str_field(payload, "mode") == "dry_run";
    let confirmation = str_field(payload, "confirmation");
    let status = str_field(&rollback_manifest, "status");
    let rollback_available = rollback_manifest.get("rollback_available").and_then(Value::as_bool).unwrap_or(false);
    let operations = rollback_manifest
        .get("operations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut restore_results: Vec<Value> = Vec::new();
    let mut trace: Vec<Value> = Vec::new();

    let can_execute = execute
        && allow_file_writes
        && confirmation == "CONFIRM_ROLLBACK"
        && !dry_run
        && rollback_available
        && status == "rollback_available"
        && errors.is_empty();

    if dry_run {
        trace.push(json!({"step":"rollback_execute", "decision":"dry_run_preview_only"}));
    } else if !rollback_available || status != "rollback_available" {
        trace.push(json!({"step":"rollback_execute", "decision":"not_ready", "rollback_status": status}));
    } else if !execute || !allow_file_writes {
        trace.push(json!({"step":"rollback_execute", "decision":"rehearsal_only", "execute": execute, "allow_rollback_file_writes": allow_file_writes}));
    } else if confirmation != "CONFIRM_ROLLBACK" {
        trace.push(json!({"step":"rollback_execute", "decision":"confirmation_required"}));
        warnings.push(Diagnostic::warning(
            "rollback_confirmation_required",
            Some("confirmation".to_string()),
            "Rollback execution requires confirmation=CONFIRM_ROLLBACK.",
        ));
    } else {
        for (idx, op) in operations.iter().enumerate() {
            let op_name = str_field(op, "op");
            if op_name != "restore_file" {
                trace.push(json!({"step":"rollback_operation", "decision":"skipped_unsupported", "index": idx, "op": op_name}));
                continue;
            }
            let target_path = str_field(op, "target_path");
            let backup_path = str_field(op, "backup_path");
            if target_path.is_empty() || backup_path.is_empty() {
                errors.push(Diagnostic::error(
                    "rollback_restore_path_missing",
                    Some(format!("operations[{idx}]")),
                    "Rollback restore operation requires target_path and backup_path.",
                ));
                continue;
            }
            let target = Path::new(target_path);
            let backup = Path::new(backup_path);
            if !backup.exists() {
                errors.push(Diagnostic::error(
                    "rollback_backup_missing",
                    Some(format!("operations[{idx}].backup_path")),
                    format!("Rollback backup file does not exist: {backup_path}"),
                ));
                continue;
            }
            let restore_text = match fs::read_to_string(backup) {
                Ok(v) => v,
                Err(e) => {
                    errors.push(Diagnostic::error(
                        "rollback_backup_read_failed",
                        Some(format!("operations[{idx}].backup_path")),
                        format!("Failed to read rollback backup file: {e}"),
                    ));
                    continue;
                }
            };
            let restore_sha = sha256_text(&restore_text);
            let expected_restore_sha = op.get("restore_sha256").and_then(Value::as_str).unwrap_or("");
            if !allow_checksum_mismatch && !expected_restore_sha.is_empty() && expected_restore_sha != restore_sha {
                errors.push(Diagnostic::error(
                    "rollback_restore_checksum_mismatch",
                    Some(format!("operations[{idx}].restore_sha256")),
                    format!("Backup checksum mismatch for {backup_path}: expected {expected_restore_sha}, got {restore_sha}"),
                ));
                continue;
            }
            let expected_current = op.get("expected_current_sha256").and_then(Value::as_str).filter(|v| !v.is_empty());
            if !allow_checksum_mismatch {
                match sha256_file_text(target) {
                    Ok(Some(current_sha)) => {
                        if let Some(expected) = expected_current {
                            if current_sha != expected {
                                errors.push(Diagnostic::error(
                                    "rollback_target_checksum_mismatch",
                                    Some(format!("operations[{idx}].expected_current_sha256")),
                                    format!("Target file checksum mismatch for {target_path}: expected {expected}, got {current_sha}"),
                                ));
                                continue;
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        errors.push(Diagnostic::error(
                            "rollback_target_hash_failed",
                            Some(format!("operations[{idx}].target_path")),
                            format!("Failed to read target file before rollback: {e}"),
                        ));
                        continue;
                    }
                }
            }
            let expected_for_atomic = if allow_checksum_mismatch { None } else { expected_current };
            match atomic_write_text(target, &restore_text, true, expected_for_atomic, "rollback_restore") {
                Ok(mut result) => {
                    if let Some(obj) = result.as_object_mut() {
                        obj.insert("rollback_operation".to_string(), json!("restore_file"));
                        obj.insert("source_backup_path".to_string(), json!(backup_path));
                        obj.insert("restore_sha256".to_string(), json!(restore_sha));
                    }
                    restore_results.push(result);
                    trace.push(json!({"step":"restore_file", "decision":"restored", "path": target_path, "backup_path": backup_path}));
                }
                Err(e) => errors.push(Diagnostic::error(
                    "rollback_restore_write_failed",
                    Some(format!("operations[{idx}].target_path")),
                    format!("Rollback restore write failed: {e}"),
                )),
            }
        }
    }

    let executed = can_execute && errors.is_empty() && !restore_results.is_empty();
    let final_status = if !errors.is_empty() {
        "failed"
    } else if executed {
        "executed_file_restores"
    } else if dry_run {
        "dry_run_preview_only"
    } else if !rollback_available || status != "rollback_available" {
        "not_ready"
    } else if !execute || !allow_file_writes {
        "rehearsal_only"
    } else if confirmation != "CONFIRM_ROLLBACK" {
        "confirmation_required"
    } else if operations.is_empty() {
        "no_restore_operations"
    } else {
        "rehearsal_only"
    };

    let result = json!({
        "mode": "rollback_executor",
        "authoritative": executed,
        "executed": executed,
        "status": final_status,
        "rollback_manifest": rollback_manifest,
        "restore_results": restore_results,
        "restore_count": restore_results.len(),
        "execute_requested": execute,
        "allow_rollback_file_writes": allow_file_writes,
        "confirmation_required": true,
        "confirmation_ok": confirmation == "CONFIRM_ROLLBACK",
        "allow_checksum_mismatch": allow_checksum_mismatch,
        "trace": trace,
    });
    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> String {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("lqosync-rollback-{name}-{now}")).to_string_lossy().to_string()
    }

    #[test]
    fn rehearses_without_execute_flag() {
        let target = temp_path("target.csv");
        let backup = temp_path("backup.csv");
        fs::write(&target, "new").unwrap();
        fs::write(&backup, "old").unwrap();
        let payload = json!({
            "rollback_manifest": {"status":"rollback_available", "rollback_available":true, "operations":[{"op":"restore_file", "target_path": target, "backup_path": backup, "expected_current_sha256": sha256_text("new"), "restore_sha256": sha256_text("old")}]},
            "execute": false,
            "allow_rollback_file_writes": false
        });
        let (result, errors, _warnings) = execute_rollback_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("executed").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("status").and_then(Value::as_str), Some("rehearsal_only"));
    }

    #[test]
    fn requires_confirmation_even_when_allowed() {
        let target = temp_path("target.csv");
        let backup = temp_path("backup.csv");
        fs::write(&target, "new").unwrap();
        fs::write(&backup, "old").unwrap();
        let payload = json!({
            "rollback_manifest": {"status":"rollback_available", "rollback_available":true, "operations":[{"op":"restore_file", "target_path": target, "backup_path": backup, "expected_current_sha256": sha256_text("new"), "restore_sha256": sha256_text("old")}]},
            "execute": true,
            "allow_rollback_file_writes": true,
            "confirmation": ""
        });
        let (result, errors, _warnings) = execute_rollback_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("executed").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("status").and_then(Value::as_str), Some("confirmation_required"));
    }

    #[test]
    fn restores_file_when_explicitly_confirmed() {
        let target = temp_path("target.csv");
        let backup = temp_path("backup.csv");
        fs::write(&target, "new").unwrap();
        fs::write(&backup, "old").unwrap();
        let payload = json!({
            "rollback_manifest": {"status":"rollback_available", "rollback_available":true, "operations":[{"op":"restore_file", "target_path": target, "backup_path": backup, "expected_current_sha256": sha256_text("new"), "restore_sha256": sha256_text("old")}]},
            "execute": true,
            "allow_rollback_file_writes": true,
            "confirmation": "CONFIRM_ROLLBACK"
        });
        let (result, errors, _warnings) = execute_rollback_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(result.get("executed").and_then(Value::as_bool), Some(true));
        assert_eq!(result.get("status").and_then(Value::as_str), Some("executed_file_restores"));
        let restored_path = result["restore_results"][0]["path"].as_str().unwrap();
        assert_eq!(fs::read_to_string(restored_path).unwrap(), "old");
    }
}
