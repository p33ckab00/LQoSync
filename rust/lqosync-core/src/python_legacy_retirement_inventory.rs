use crate::protocol::Diagnostic;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const CONFIRM_INVENTORY: &str = "CONFIRM_PYTHON_LEGACY_RETIREMENT_INVENTORY";

const DEFAULT_PYTHON_PATHS: &[&str] = &[
    "app.py",
    "auth/__init__.py",
    "auth/users.py",
    "engine/rust_core.py",
    "engine/config_loader.py",
    "engine/config_schema.py",
    "engine/config_writer.py",
    "engine/dashboard_modules.py",
    "engine/docs_search.py",
    "engine/release_integrity.py",
    "engine/stable_release.py",
    "scheduler/runner.py",
    "builders/shaped_devices.py",
    "builders/network_json.py",
    "rules/cleanup.py",
    "applier/atomic_writer.py",
    "applier/backup.py",
    "applier/rollback.py",
    "collectors/mikrotik_client.py",
    "monitoring/service_monitor.py",
    "templates/dashboard.html",
    "templates/base.html",
    "static/favicon.svg",
];

fn bool_value(v: Option<&Value>, default: bool) -> bool {
    v.and_then(Value::as_bool).unwrap_or(default)
}

fn str_value<'a>(v: Option<&'a Value>, default: &'a str) -> &'a str {
    v.and_then(Value::as_str).unwrap_or(default)
}

fn number_value(v: Option<&Value>, default: u64) -> u64 {
    v.and_then(Value::as_u64).unwrap_or(default)
}

fn config_value<'a>(payload: &'a Value, key: &str) -> Option<&'a Value> {
    payload
        .get("rust_core")
        .and_then(|v| v.get(key))
        .or_else(|| {
            payload
                .get("config")
                .and_then(|c| c.get("rust_core"))
                .and_then(|v| v.get(key))
        })
}

fn first_object<'a>(payload: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    for key in keys {
        if let Some(value) = payload.get(*key) {
            if let Some(result) = value.get("result") {
                if result.is_object() {
                    return Some(result);
                }
            }
            if value.is_object() {
                return Some(value);
            }
        }
    }
    None
}

fn path_strings(payload: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(items) = payload.get("python_paths").and_then(Value::as_array) {
        for item in items {
            if let Some(path) = item.as_str() {
                paths.push(path.to_string());
            } else if let Some(path) = item.get("path").and_then(Value::as_str) {
                paths.push(path.to_string());
            }
        }
    }
    if paths.is_empty() {
        paths.extend(DEFAULT_PYTHON_PATHS.iter().map(|p| (*p).to_string()));
    }
    paths.sort();
    paths.dedup();
    paths
}

fn classify_path(path: &str) -> (&'static str, &'static str, &'static str) {
    if path == "app.py"
        || path.starts_with("templates/")
        || path.starts_with("static/")
        || path.starts_with("auth/")
        || path == "engine/rust_core.py"
        || path == "engine/config_loader.py"
        || path == "engine/config_schema.py"
        || path == "engine/config_writer.py"
        || path == "engine/dashboard_modules.py"
        || path == "engine/docs_search.py"
        || path == "engine/release_integrity.py"
        || path == "engine/stable_release.py"
        || path == "scheduler/runner.py"
        || path == "builders/shaped_devices.py"
        || path == "builders/network_json.py"
        || path == "rules/cleanup.py"
        || path == "applier/atomic_writer.py"
        || path == "applier/backup.py"
        || path == "applier/rollback.py"
        || path == "collectors/mikrotik_client.py"
        || path.starts_with("monitoring/")
    {
        return (
            "webui_shell_required",
            "preserve",
            "Flask WebUI shell, operator diagnostics, or Rust protocol bridge remains in the package.",
        );
    }
    if path.starts_with("collectors/")
        || path.starts_with("builders/")
        || path.starts_with("rules/")
        || path.starts_with("parsers/")
        || path.starts_with("validators/")
        || path.starts_with("applier/")
        || path.starts_with("scheduler/")
    {
        return (
            "legacy_backend_candidate",
            "archive_after_guarded_cutover",
            "Production backend authority is Rust-owned; archive only after post-retirement and rollback gates pass.",
        );
    }
    if path.ends_with(".py") {
        return (
            "python_shell_or_unknown",
            "inspect_before_cleanup",
            "Python path is not in the canonical backend-retirement manifest and needs operator review.",
        );
    }
    (
        "non_python_or_asset",
        "preserve",
        "Non-Python assets are outside backend retirement cleanup.",
    )
}

fn inventory_id(seed: &Value) -> String {
    let text = serde_json::to_string(seed).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("py-legacy-retire-{}", &digest[..16])
}

pub fn build_python_legacy_retirement_inventory_payload(
    payload: &Value,
) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    let requested_execute = bool_value(payload.get("execute"), false)
        || bool_value(payload.get("files_deleted"), false)
        || bool_value(payload.get("python_files_removed"), false)
        || bool_value(payload.get("archive_executed"), false)
        || matches!(
            str_value(payload.get("mode"), "inventory_only"),
            "execute" | "delete" | "remove" | "archive" | "cleanup" | "retire"
        );
    if requested_execute {
        errors.push(Diagnostic::error(
            "python_legacy_retirement_inventory_execute_not_implemented",
            Some("python_legacy_retirement_inventory".to_string()),
            "The Python legacy retirement inventory is non-mutating. It classifies cleanup candidates but never deletes, archives, disables, or moves files.",
        ));
    }

    let allow = bool_value(
        config_value(payload, "allow_python_legacy_retirement_inventory"),
        false,
    );
    let pilot = bool_value(
        config_value(payload, "python_legacy_retirement_inventory_pilot"),
        false,
    );
    let mode = str_value(
        config_value(payload, "python_legacy_retirement_inventory_mode"),
        "inventory_only",
    );
    let require_audit_sentinel = bool_value(
        config_value(payload, "python_legacy_retirement_require_audit_sentinel"),
        true,
    );
    let require_webui_shell = bool_value(
        config_value(payload, "python_legacy_retirement_require_webui_shell"),
        true,
    );
    let require_rollback = bool_value(
        config_value(payload, "python_legacy_retirement_require_rollback_package"),
        true,
    );
    let require_confirmation = bool_value(
        config_value(
            payload,
            "python_legacy_retirement_require_manual_confirmation",
        ),
        true,
    );
    let require_operator_ack = bool_value(
        config_value(payload, "python_legacy_retirement_require_operator_ack"),
        true,
    );
    let require_no_side_effects = bool_value(
        config_value(payload, "python_legacy_retirement_require_no_side_effects"),
        true,
    );
    let max_shadow_age = number_value(
        config_value(payload, "python_legacy_retirement_max_shadow_age_seconds"),
        900,
    );
    let shadow_age = number_value(payload.get("shadow_age_seconds"), 0);

    let confirmation_ok =
        !require_confirmation || str_value(payload.get("confirmation"), "") == CONFIRM_INVENTORY;
    if require_confirmation && !confirmation_ok {
        warnings.push(Diagnostic::warning(
            "python_legacy_retirement_inventory_confirmation_required",
            Some("confirmation".to_string()),
            "Python legacy retirement inventory requires CONFIRM_PYTHON_LEGACY_RETIREMENT_INVENTORY before it can report ready.",
        ));
    }

    let audit = first_object(
        payload,
        &[
            "full_rust_backend_production_audit_sentinel",
            "full_rust_backend_audit_sentinel",
            "production_audit_sentinel",
        ],
    );
    let audit_ready = audit
        .map(|v| {
            v.get("status").and_then(Value::as_str)
                == Some("full_rust_backend_production_audit_sentinel_healthy")
                && bool_value(v.get("full_rust_backend"), false)
                && bool_value(v.get("python_backend_removed"), false)
                && bool_value(v.get("python_backend_retired"), false)
                && !bool_value(v.get("side_effects_allowed"), false)
        })
        .unwrap_or(false);
    if require_audit_sentinel && !audit_ready {
        warnings.push(Diagnostic::warning(
            "python_legacy_retirement_audit_sentinel_not_ready",
            Some("full_rust_backend_production_audit_sentinel".to_string()),
            "Production audit sentinel must verify healthy full-Rust authority before legacy Python cleanup can be considered.",
        ));
    }

    let python_runtime_role = str_value(config_value(payload, "python_runtime_role"), "");
    let webui_shell_ready = python_runtime_role == "flask_webui_shell_only"
        && bool_value(payload.get("webui_ux_unchanged"), false)
        && bool_value(payload.get("webui_static_asset_paths_unchanged"), false)
        && bool_value(payload.get("webui_static_assets_preserved"), false);
    if require_webui_shell && !webui_shell_ready {
        warnings.push(Diagnostic::warning(
            "python_legacy_retirement_webui_shell_not_preserved",
            Some("webui_ux_unchanged".to_string()),
            "Flask WebUI shell role and WebUI/static asset preservation are required.",
        ));
    }

    let rollback_ready = bool_value(payload.get("python_backend_rollback_package_ready"), false)
        && bool_value(payload.get("rollback_test_passed"), false)
        && !str_value(payload.get("rollback_path"), "")
            .trim()
            .is_empty();
    if require_rollback && !rollback_ready {
        warnings.push(Diagnostic::warning(
            "python_legacy_retirement_rollback_package_required",
            Some("python_backend_rollback_package_ready".to_string()),
            "Rollback package, rollback test, and rollback path are required before cleanup candidates can be archived.",
        ));
    }

    let operator_ack = bool_value(payload.get("operator_python_legacy_retirement_ack"), false)
        || bool_value(payload.get("operator_acknowledged"), false);
    if require_operator_ack && !operator_ack {
        warnings.push(Diagnostic::warning(
            "python_legacy_retirement_operator_ack_required",
            Some("operator_python_legacy_retirement_ack".to_string()),
            "Operator acknowledgement is required before legacy Python retirement inventory can report ready.",
        ));
    }

    if shadow_age > max_shadow_age {
        warnings.push(
            Diagnostic::warning(
                "python_legacy_retirement_shadow_stale",
                Some("shadow_age_seconds".to_string()),
                "Full-Rust backend evidence is older than the configured maximum age.",
            )
            .with_value(
                json!({"shadow_age_seconds": shadow_age, "max_shadow_age_seconds": max_shadow_age}),
            ),
        );
    }

    let gates_ready = allow && pilot && mode == "inventory_only";
    if !gates_ready {
        warnings.push(Diagnostic::warning(
            "python_legacy_retirement_inventory_gates_not_enabled",
            Some("rust_core".to_string()),
            "Python legacy retirement inventory gates are not fully enabled.",
        ));
    }

    let mut manifest = Vec::new();
    let mut webui_count = 0_u64;
    let mut legacy_count = 0_u64;
    let mut unknown_count = 0_u64;
    let mut compatibility_count = 0_u64;
    let mut removal_candidates = Vec::new();

    for path in path_strings(payload) {
        let (classification, action, reason) = classify_path(&path);
        match classification {
            "webui_shell_required" => webui_count += 1,
            "legacy_backend_candidate" => {
                legacy_count += 1;
                removal_candidates.push(json!(path));
            }
            "python_shell_or_unknown" => unknown_count += 1,
            _ => compatibility_count += 1,
        }
        manifest.push(json!({
            "path": path,
            "classification": classification,
            "action": action,
            "reason": reason,
        }));
    }

    if unknown_count > 0 {
        warnings.push(
            Diagnostic::warning(
                "python_legacy_retirement_unknown_python_paths",
                Some("python_paths".to_string()),
                "One or more Python paths are outside the canonical retirement manifest and require operator inspection.",
            )
            .with_value(json!({"unknown_python_path_count": unknown_count})),
        );
    }

    let ready = errors.is_empty()
        && gates_ready
        && confirmation_ok
        && (!require_audit_sentinel || audit_ready)
        && (!require_webui_shell || webui_shell_ready)
        && (!require_rollback || rollback_ready)
        && (!require_operator_ack || operator_ack)
        && (!require_no_side_effects || !requested_execute)
        && shadow_age <= max_shadow_age;

    let status = if !errors.is_empty() {
        "blocked"
    } else if ready {
        "python_legacy_retirement_inventory_ready"
    } else if audit_ready && webui_shell_ready {
        "python_legacy_retirement_inventory_review"
    } else {
        "python_legacy_retirement_inventory_blocked"
    };

    let mut seed = Map::new();
    seed.insert("status".to_string(), json!(status));
    seed.insert(
        "legacy_backend_candidate_count".to_string(),
        json!(legacy_count),
    );
    seed.insert("webui_shell_required_count".to_string(), json!(webui_count));
    seed.insert(
        "unknown_python_path_count".to_string(),
        json!(unknown_count),
    );

    let mut result = Map::new();
    result.insert(
        "mode".to_string(),
        json!("python_legacy_retirement_inventory"),
    );
    result.insert("status".to_string(), json!(status));
    result.insert(
        "inventory_id".to_string(),
        json!(inventory_id(&Value::Object(seed))),
    );
    result.insert(
        "python_legacy_retirement_inventory_ready".to_string(),
        json!(ready),
    );
    result.insert("inventory_only".to_string(), json!(true));
    result.insert("non_mutating".to_string(), json!(true));
    result.insert("side_effects_allowed".to_string(), json!(false));
    result.insert("delete_allowed".to_string(), json!(false));
    result.insert("archive_plan_allowed".to_string(), json!(ready));
    result.insert("audit_sentinel_ready".to_string(), json!(audit_ready));
    result.insert("webui_shell_ready".to_string(), json!(webui_shell_ready));
    result.insert("rollback_ready".to_string(), json!(rollback_ready));
    result.insert("operator_acknowledged".to_string(), json!(operator_ack));
    result.insert(
        "python_runtime_role".to_string(),
        json!(python_runtime_role),
    );
    result.insert("total_python_paths".to_string(), json!(manifest.len()));
    result.insert("webui_shell_required_count".to_string(), json!(webui_count));
    result.insert(
        "legacy_backend_candidate_count".to_string(),
        json!(legacy_count),
    );
    result.insert(
        "python_shell_or_unknown_count".to_string(),
        json!(unknown_count),
    );
    result.insert(
        "compatibility_or_asset_count".to_string(),
        json!(compatibility_count),
    );
    result.insert(
        "legacy_backend_candidates".to_string(),
        json!(removal_candidates),
    );
    result.insert("manifest".to_string(), Value::Array(manifest));

    (Value::Object(result), errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_payload() -> Value {
        json!({
            "confirmation": "CONFIRM_PYTHON_LEGACY_RETIREMENT_INVENTORY",
            "shadow_age_seconds": 0,
            "full_rust_backend_production_audit_sentinel": {
                "status": "full_rust_backend_production_audit_sentinel_healthy",
                "full_rust_backend": true,
                "python_backend_removed": true,
                "python_backend_retired": true,
                "side_effects_allowed": false
            },
            "webui_ux_unchanged": true,
            "webui_static_asset_paths_unchanged": true,
            "webui_static_assets_preserved": true,
            "python_backend_rollback_package_ready": true,
            "rollback_test_passed": true,
            "rollback_path": "restore_python_backend_and_flask_routes",
            "operator_python_legacy_retirement_ack": true,
            "python_paths": [
                "app.py",
                "engine/rust_core.py",
                "templates/dashboard.html"
            ],
            "rust_core": {
                "python_runtime_role": "flask_webui_shell_only",
                "allow_python_legacy_retirement_inventory": true,
                "python_legacy_retirement_inventory_pilot": true,
                "python_legacy_retirement_inventory_mode": "inventory_only"
            }
        })
    }

    #[test]
    fn builds_ready_inventory_when_full_rust_evidence_is_present() {
        let (result, errors, _warnings) =
            build_python_legacy_retirement_inventory_payload(&ready_payload());
        assert!(errors.is_empty());
        assert_eq!(
            result.get("status").and_then(Value::as_str),
            Some("python_legacy_retirement_inventory_ready")
        );
        assert_eq!(
            result
                .get("python_legacy_retirement_inventory_ready")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            result.get("archive_plan_allowed").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            result.get("delete_allowed").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            result
                .get("legacy_backend_candidate_count")
                .and_then(Value::as_u64),
            Some(0)
        );
    }

    #[test]
    fn blocks_inventory_when_gates_are_missing() {
        let (result, errors, warnings) =
            build_python_legacy_retirement_inventory_payload(&json!({}));
        assert!(errors.is_empty());
        assert_eq!(
            result.get("status").and_then(Value::as_str),
            Some("python_legacy_retirement_inventory_blocked")
        );
        assert!(warnings
            .iter()
            .any(|w| w.code == "python_legacy_retirement_inventory_gates_not_enabled"));
    }

    #[test]
    fn refuses_execute_or_delete_requests() {
        let mut payload = ready_payload();
        payload["execute"] = json!(true);
        let (_result, errors, _warnings) =
            build_python_legacy_retirement_inventory_payload(&payload);
        assert!(errors
            .iter()
            .any(|e| { e.code == "python_legacy_retirement_inventory_execute_not_implemented" }));
    }
}
