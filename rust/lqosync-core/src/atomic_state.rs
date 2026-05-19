use crate::protocol::{Diagnostic, Severity};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub fn sha256_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

fn sha256_file(path: &Path) -> std::io::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let mut f = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(Some(hex::encode(hasher.finalize())))
}

fn fsync_parent(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        // Directory fsync is supported on Linux and protects the rename itself.
        if let Ok(dir) = File::open(parent) {
            dir.sync_all()?;
        }
    }
    Ok(())
}

fn tmp_path_for(target: &Path) -> PathBuf {
    let pid = std::process::id();
    let file_name = target.file_name().and_then(|v| v.to_str()).unwrap_or("lqosync-state");
    target.with_file_name(format!(".{file_name}.tmp.{pid}"))
}

pub fn atomic_write_text_payload(payload: &Value) -> anyhow::Result<Value> {
    let path = payload.get("path").and_then(Value::as_str).unwrap_or("").trim();
    if path.is_empty() {
        anyhow::bail!("path is required");
    }
    let content = payload.get("content").and_then(Value::as_str).unwrap_or("");
    let create_backup = payload.get("create_backup").and_then(Value::as_bool).unwrap_or(false);
    let expected_sha256 = payload.get("expected_sha256").and_then(Value::as_str).map(str::to_string);
    let file_kind = payload.get("file_kind").and_then(Value::as_str).unwrap_or("text");
    atomic_write_text(Path::new(path), content, create_backup, expected_sha256.as_deref(), file_kind)
}

pub fn atomic_write_json_state_payload(payload: &Value) -> anyhow::Result<Value> {
    let path = payload.get("path").and_then(Value::as_str).unwrap_or("").trim();
    if path.is_empty() {
        anyhow::bail!("path is required");
    }
    let state_type = payload.get("state_type").and_then(Value::as_str).unwrap_or("generic");
    let state = payload.get("state").cloned().unwrap_or_else(|| json!({}));
    let (_result, errors, _warnings) = validate_json_state(&state, state_type);
    if !errors.is_empty() {
        let messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        anyhow::bail!("state validation failed: {}", messages.join("; "));
    }
    let create_backup = payload.get("create_backup").and_then(Value::as_bool).unwrap_or(false);
    let expected_sha256 = payload.get("expected_sha256").and_then(Value::as_str).map(str::to_string);
    let mut text = serde_json::to_string_pretty(&state)?;
    text.push('\n');
    atomic_write_text(Path::new(path), &text, create_backup, expected_sha256.as_deref(), state_type)
}

pub fn append_audit_jsonl_payload(payload: &Value) -> anyhow::Result<Value> {
    let path = payload.get("path").and_then(Value::as_str).unwrap_or("").trim();
    if path.is_empty() {
        anyhow::bail!("path is required");
    }
    let event = payload.get("event").cloned().unwrap_or_else(|| json!({}));
    let target = Path::new(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(&event)? + "\n";
    let before_sha256 = sha256_file(target)?;
    let mut f = OpenOptions::new().create(true).append(true).open(target)?;
    f.write_all(line.as_bytes())?;
    f.flush()?;
    f.sync_all()?;
    fsync_parent(target)?;
    let after_sha256 = sha256_file(target)?;
    Ok(json!({
        "path": path,
        "operation": "append_audit_jsonl",
        "bytes_appended": line.len(),
        "before_sha256": before_sha256,
        "after_sha256": after_sha256,
        "wrote": true,
    }))
}

pub fn atomic_write_text(target: &Path, content: &str, create_backup: bool, expected_sha256: Option<&str>, file_kind: &str) -> anyhow::Result<Value> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let before_sha256 = sha256_file(target)?;
    if let Some(expected) = expected_sha256 {
        if let Some(before) = before_sha256.as_deref() {
            if before != expected {
                anyhow::bail!("sha256 mismatch for {}: expected {}, got {}", target.display(), expected, before);
            }
        }
    }
    let backup_path = if create_backup && target.exists() {
        let backup = target.with_extension(format!("{}bak", target.extension().and_then(|e| e.to_str()).map(|e| format!("{e}." )).unwrap_or_default()));
        fs::copy(target, &backup)?;
        Some(backup.to_string_lossy().to_string())
    } else {
        None
    };
    let tmp = tmp_path_for(target);
    {
        let mut f = File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.flush()?;
        f.sync_all()?;
    }
    fs::rename(&tmp, target)?;
    fsync_parent(target)?;
    let after_sha256 = sha256_file(target)?;
    Ok(json!({
        "path": target.to_string_lossy(),
        "file_kind": file_kind,
        "operation": "atomic_write",
        "bytes_written": content.len(),
        "before_sha256": before_sha256,
        "after_sha256": after_sha256,
        "backup_path": backup_path,
        "wrote": true,
    }))
}

pub fn validate_json_state_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let state_type = payload.get("state_type").and_then(Value::as_str).unwrap_or("generic");
    let state = payload.get("state").unwrap_or(payload);
    validate_json_state(state, state_type)
}

pub fn validate_json_state(state: &Value, state_type: &str) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    if !state.is_object() {
        errors.push(Diagnostic::error(
            "state_not_object",
            Some("state".to_string()),
            format!("{state_type} must be a JSON object"),
        ));
    }
    match state_type {
        "runtime_state" => {
            for key in ["scheduler_state", "sync_running", "last_error"] {
                if state.get(key).is_none() {
                    warnings.push(Diagnostic {
                        code: "runtime_state_missing_recommended_key".to_string(),
                        severity: Severity::Warning,
                        path: Some(format!("state.{key}")),
                        message: format!("runtime_state is missing recommended key: {key}"),
                        value: None,
                        safe_for_cleanup: None,
                    });
                }
            }
        }
        "policy_state" => {
            for key in ["pending_confirmations", "cleanup_queue"] {
                if !state.get(key).map(Value::is_array).unwrap_or(false) {
                    errors.push(Diagnostic::error(
                        "policy_state_invalid_array",
                        Some(format!("state.{key}")),
                        format!("policy_state.{key} must be an array"),
                    ));
                }
            }
        }
        "collector_cache" => {
            if !state.get("sources").map(Value::is_object).unwrap_or(false) {
                errors.push(Diagnostic::error(
                    "collector_cache_invalid_sources",
                    Some("state.sources".to_string()),
                    "collector_cache.sources must be an object".to_string(),
                ));
            }
        }
        "config" => {
            if state.pointer("/paths/shaped_devices_csv").and_then(Value::as_str).unwrap_or("").is_empty() {
                errors.push(Diagnostic::error(
                    "config_missing_shaped_devices_path",
                    Some("state.paths.shaped_devices_csv".to_string()),
                    "config.paths.shaped_devices_csv is required".to_string(),
                ));
            }
        }
        _ => {}
    }
    let result = json!({
        "state_type": state_type,
        "valid": errors.is_empty(),
        "write_allowed": errors.is_empty(),
        "apply_allowed": errors.is_empty(),
    });
    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_collector_cache_shape() {
        let (_result, errors, _warnings) = validate_json_state(&json!({"sources": {}}), "collector_cache");
        assert!(errors.is_empty());
        let (_result, errors, _warnings) = validate_json_state(&json!({"sources": []}), "collector_cache");
        assert!(!errors.is_empty());
    }
}
