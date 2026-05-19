use crate::protocol::{Diagnostic, Severity};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

fn sha256_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

fn compact_hash(value: &Value) -> String {
    sha256_text(&serde_json::to_string(value).unwrap_or_default())
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn response_result(value: Option<&Value>) -> Value {
    match value {
        Some(v) => v.get("result").cloned().unwrap_or_else(|| v.clone()),
        None => json!({}),
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

fn array_len(value: &Value, key: &str) -> usize {
    value.get(key).and_then(Value::as_array).map(|v| v.len()).unwrap_or(0)
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

/// Build a non-mutating transaction journal entry from the Rust shadow/manifest/transaction context.
///
/// v1.2 intentionally does not append the journal by itself. Python or a later
/// authority mode may append the returned event through append-audit-jsonl or a
/// dedicated journal writer after operator review.
pub fn build_transaction_journal_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let paths = payload
        .get("paths")
        .cloned()
        .or_else(|| payload.get("config").and_then(|c| c.get("paths")).cloned())
        .unwrap_or_else(|| json!({}));
    let manifest = response_result(payload.get("rust_apply_manifest").or_else(|| payload.get("apply_manifest")).or_else(|| payload.get("manifest")));
    let transaction = response_result(payload.get("rust_apply_transaction").or_else(|| payload.get("apply_transaction")).or_else(|| payload.get("transaction")));
    let sync_plan = response_result(payload.get("rust_sync_plan").or_else(|| payload.get("sync_plan")));
    let authority_gate = payload.get("rust_authority_gate").cloned().unwrap_or_else(|| json!({}));
    let policy = payload.get("policy_decision").cloned().unwrap_or_else(|| json!({}));
    let mode = payload.get("mode").and_then(Value::as_str).unwrap_or("apply");
    let journal_path = payload
        .get("journal_path")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| str_path(&paths, &["transaction_journal"], "/opt/lqosync/logs/transaction_journal.jsonl"));
    let manifest_id = manifest.get("manifest_id").and_then(Value::as_str).unwrap_or("unknown");
    let transaction_status = transaction.get("status").and_then(Value::as_str).unwrap_or("not_run");
    let executed = transaction.get("executed").and_then(Value::as_bool).unwrap_or(false);
    let write_count = transaction.get("write_count").and_then(Value::as_u64).unwrap_or(0);
    let operation_count = manifest.get("operation_count").and_then(Value::as_u64).unwrap_or_else(|| array_len(&manifest, "operations") as u64);
    let rollback_available = transaction
        .get("write_results")
        .and_then(Value::as_array)
        .map(|items| items.iter().any(|item| item.get("backup_path").and_then(Value::as_str).map(|s| !s.is_empty()).unwrap_or(false)))
        .unwrap_or(false);

    if executed && !rollback_available {
        warnings.push(warning(
            "transaction_journal_no_restore_points",
            Some("rust_apply_transaction.write_results".to_string()),
            "Transaction executed file writes but no backup_path restore points were found in write_results.",
        ));
    }

    let basis = json!({
        "manifest_id": manifest_id,
        "transaction_status": transaction_status,
        "executed": executed,
        "write_count": write_count,
        "operation_count": operation_count,
        "sync_verdict": sync_plan.get("verdict"),
        "authority_gate": authority_gate.get("reason"),
    });
    let journal_id = format!("txj-{}", &compact_hash(&basis)[..16]);
    let event = json!({
        "schema_version": "1",
        "event": "rust_apply_transaction_journal",
        "journal_id": journal_id,
        "generated_at_unix": now_unix_seconds(),
        "mode": mode,
        "manifest_id": manifest_id,
        "manifest_status": manifest.get("status"),
        "transaction_status": transaction_status,
        "executed": executed,
        "write_count": write_count,
        "operation_count": operation_count,
        "rollback_available": rollback_available,
        "policy_verdict": policy.get("verdict"),
        "sync_plan_verdict": sync_plan.get("verdict"),
        "authority_gate": authority_gate,
        "manifest": manifest,
        "transaction": transaction,
    });

    let result = json!({
        "mode": "transaction_journal_preview",
        "authoritative": false,
        "journal_id": event.get("journal_id"),
        "journal_path": journal_path,
        "append_required": executed,
        "append_executed": false,
        "rollback_available": rollback_available,
        "manifest_id": manifest_id,
        "transaction_status": transaction_status,
        "executed": executed,
        "write_count": write_count,
        "operation_count": operation_count,
        "event": event,
    });
    (result, errors, warnings)
}

/// Build a non-mutating rollback manifest from a transaction result.
///
/// The rollback plan is intentionally preview-only. It lists restore_file steps
/// for write results that have backup_path metadata. Operators can inspect this
/// plan before a future Rust rollback executor is introduced.
pub fn build_rollback_manifest_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let transaction = response_result(payload.get("rust_apply_transaction").or_else(|| payload.get("apply_transaction")).or_else(|| payload.get("transaction")));
    let manifest = response_result(payload.get("rust_apply_manifest").or_else(|| payload.get("apply_manifest")).or_else(|| payload.get("manifest")));
    let executed = transaction.get("executed").and_then(Value::as_bool).unwrap_or(false);
    let mut operations: Vec<Value> = Vec::new();

    if let Some(items) = transaction.get("write_results").and_then(Value::as_array) {
        for item in items {
            let target_path = item.get("path").and_then(Value::as_str).unwrap_or("");
            let backup_path = item.get("backup_path").and_then(Value::as_str).unwrap_or("");
            if target_path.is_empty() || backup_path.is_empty() {
                continue;
            }
            operations.push(json!({
                "op": "restore_file",
                "phase": "rollback",
                "target_path": target_path,
                "backup_path": backup_path,
                "expected_current_sha256": item.get("after_sha256"),
                "restore_sha256": item.get("before_sha256"),
                "file_kind": item.get("file_kind"),
                "allowed_now": false,
            }));
        }
    }

    let status = if !operations.is_empty() {
        "rollback_available"
    } else if executed {
        warnings.push(warning(
            "rollback_no_restore_points",
            Some("transaction.write_results".to_string()),
            "Transaction executed but no usable backup_path entries were found for rollback.",
        ));
        "no_restore_points"
    } else {
        "not_executed"
    };

    let basis = json!({
        "manifest_id": manifest.get("manifest_id"),
        "transaction_status": transaction.get("status"),
        "operations": operations.clone(),
    });
    let rollback_id = format!("rollback-{}", &compact_hash(&basis)[..16]);
    let result = json!({
        "mode": "rollback_manifest_preview",
        "authoritative": false,
        "rollback_id": rollback_id,
        "status": status,
        "manifest_id": manifest.get("manifest_id"),
        "transaction_status": transaction.get("status"),
        "executed": executed,
        "rollback_available": !operations.is_empty(),
        "operation_count": operations.len(),
        "operations": operations,
        "requires_operator_confirmation": true,
        "execute_supported": false,
    });
    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_transaction_journal_preview() {
        let payload = json!({
            "mode": "apply",
            "paths": {"transaction_journal": "/opt/lqosync/logs/transaction_journal.jsonl"},
            "rust_apply_manifest": {"result": {"manifest_id":"apply-abc", "status":"ready", "operation_count":2, "operations":[]}},
            "rust_apply_transaction": {"result": {"status":"rehearsal_only", "executed":false, "write_count":0, "write_results":[]}},
            "rust_sync_plan": {"result": {"verdict":"ready_by_shadow_plan"}}
        });
        let (result, errors, _warnings) = build_transaction_journal_payload(&payload);
        assert!(errors.is_empty());
        assert_eq!(result.get("append_executed").and_then(Value::as_bool), Some(false));
        assert!(result.get("journal_id").and_then(Value::as_str).unwrap_or("").starts_with("txj-"));
    }

    #[test]
    fn builds_rollback_manifest_from_backup_paths() {
        let payload = json!({
            "rust_apply_manifest": {"result": {"manifest_id":"apply-abc"}},
            "rust_apply_transaction": {"result": {
                "status":"executed_file_writes",
                "executed":true,
                "write_results":[{"path":"/tmp/ShapedDevices.csv", "backup_path":"/tmp/ShapedDevices.csv.bak", "before_sha256":"old", "after_sha256":"new", "file_kind":"ShapedDevices.csv"}]
            }}
        });
        let (result, errors, _warnings) = build_rollback_manifest_payload(&payload);
        assert!(errors.is_empty());
        assert_eq!(result.get("status").and_then(Value::as_str), Some("rollback_available"));
        assert_eq!(result.get("operation_count").and_then(Value::as_u64), Some(1));
    }
}
