use crate::apply_manifest::build_apply_manifest_payload;
use crate::diff::diff_files_payload;
use crate::policy::evaluate_policy_payload;
use crate::protocol::Diagnostic;
use crate::sync_plan::evaluate_sync_plan_payload;
use crate::validators::validate_files_payload;
use serde_json::{json, Value};
use std::time::Instant;

fn merge_diags(target: &mut Vec<Diagnostic>, mut source: Vec<Diagnostic>) {
    target.append(&mut source);
}

fn response_envelope(
    op: &str,
    result: Value,
    errors: &[Diagnostic],
    warnings: &[Diagnostic],
) -> Value {
    json!({
        "op": op,
        "ok": errors.is_empty(),
        "available": true,
        "result": result,
        "errors": errors,
        "warnings": warnings
    })
}

fn envelope_or_build(
    payload: &Value,
    key: &str,
    op: &str,
    builder: impl FnOnce() -> (Value, Vec<Diagnostic>, Vec<Diagnostic>),
    errors: &mut Vec<Diagnostic>,
    warnings: &mut Vec<Diagnostic>,
) -> Value {
    if let Some(existing) = payload.get(key).filter(|value| {
        value.is_object() && (value.get("result").is_some() || value.get("op").is_some())
    }) {
        return existing.clone();
    }

    let (result, op_errors, op_warnings) = builder();
    merge_diags(errors, op_errors.clone());
    merge_diags(warnings, op_warnings.clone());
    response_envelope(op, result, &op_errors, &op_warnings)
}

fn rust_core_authority(config: &Value) -> (bool, bool, String) {
    let rc = config.get("rust_core").unwrap_or(&Value::Null);
    let authority_mode = rc
        .get("authority_mode")
        .and_then(Value::as_str)
        .unwrap_or("shadow")
        .to_string();
    let enforced = rc
        .get("enforce_sync_plan")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || authority_mode == "enforce_blockers";
    let fail_closed_when_enforced = rc
        .get("fail_closed_when_enforced")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mode = if authority_mode.is_empty() {
        if enforced {
            "enforce_blockers".to_string()
        } else {
            "shadow".to_string()
        }
    } else {
        authority_mode
    };
    (enforced, fail_closed_when_enforced, mode)
}

fn build_authority_gate(config: &Value, mode: &str, rust_sync_plan: &Value) -> Value {
    let (enforced, fail_closed_when_enforced, authority_mode) = rust_core_authority(config);
    let dry_run = mode.eq_ignore_ascii_case("dry_run");
    let available = rust_sync_plan
        .get("available")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && !rust_sync_plan
            .get("skipped")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let result = rust_sync_plan.get("result").unwrap_or(&Value::Null);
    let verdict = result
        .get("verdict")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let blocker_count = result
        .get("blockers")
        .and_then(Value::as_array)
        .map(|items| items.len())
        .unwrap_or(0);

    let (should_block, reason) = if dry_run {
        (false, "dry_run_preview_only")
    } else if enforced && !available && fail_closed_when_enforced {
        (true, "rust_sync_plan_unavailable_fail_closed")
    } else if enforced && verdict == "blocked_by_shadow_plan" {
        (true, "rust_sync_plan_blocked")
    } else if enforced {
        (false, "rust_sync_plan_allowed")
    } else {
        (false, "shadow_only")
    };

    json!({
        "enabled": enforced,
        "authoritative": enforced && !dry_run,
        "dry_run": dry_run,
        "available": available,
        "fail_closed_when_enforced": fail_closed_when_enforced,
        "authority_mode": authority_mode,
        "verdict": verdict,
        "blocker_count": blocker_count,
        "should_block": should_block,
        "reason": reason,
        "message": if should_block {
            "Rust sync-plan authority gate blocked this non-dry-run cycle."
        } else {
            "Rust sync-plan authority gate is in preview/shadow or allowed this cycle."
        },
    })
}

/// Build a single Rust sync-engine shadow bundle for run_cycle apply/dry-run orchestration.
///
/// This keeps preflight and mutation ownership where they already are today, but moves
/// the read-only sync-engine shadow sequence into Rust: diff, validation, policy,
/// sync-plan authority gate, and apply-manifest preview.
pub fn build_rust_sync_engine_shadow_preview_payload(
    payload: &Value,
) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let started = Instant::now();
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let config = payload.get("config").cloned().unwrap_or_else(|| json!({}));
    let mode = payload
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("apply");
    let current_csv_text = payload
        .get("current_csv_text")
        .and_then(Value::as_str)
        .unwrap_or("");
    let proposed_csv_text = payload
        .get("proposed_csv_text")
        .and_then(Value::as_str)
        .unwrap_or("");
    let current_network_text = payload
        .get("current_network_text")
        .and_then(Value::as_str)
        .unwrap_or("{}");
    let proposed_network_text = payload
        .get("proposed_network_text")
        .and_then(Value::as_str)
        .unwrap_or("{}");
    let csv_changed = payload
        .get("csv_changed")
        .and_then(Value::as_bool)
        .unwrap_or(current_csv_text != proposed_csv_text);
    let network_changed = payload
        .get("network_changed")
        .and_then(Value::as_bool)
        .unwrap_or(current_network_text != proposed_network_text);
    let files_changed = payload
        .get("files_changed")
        .and_then(Value::as_bool)
        .unwrap_or(csv_changed || network_changed);
    let preflight = payload
        .get("preflight")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let collector_trust = payload
        .get("collector_trust")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let cleanup = payload.get("cleanup").cloned().unwrap_or_else(|| json!({}));
    let rust_circuit_shadow = payload
        .get("rust_circuit_shadow")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let paths = payload
        .get("paths")
        .cloned()
        .unwrap_or_else(|| config.get("paths").cloned().unwrap_or_else(|| json!({})));
    let state = payload.get("state").cloned().unwrap_or_else(|| json!({}));
    let python_policy_decision = payload
        .get("policy_decision")
        .cloned()
        .or_else(|| payload.get("python_policy_decision").cloned())
        .unwrap_or_else(|| json!({}));
    let diff_summary = payload.get("diff_summary").cloned().unwrap_or_else(|| {
        json!({
            "csv_changed": csv_changed,
            "network_changed": network_changed,
            "client_change_summary": payload.get("client_change_summary").cloned().unwrap_or_else(|| json!({}))
        })
    });

    let rust_diff = envelope_or_build(
        payload,
        "rust_diff",
        "diff-files",
        || {
            diff_files_payload(&json!({
                "current_csv_text": current_csv_text,
                "proposed_csv_text": proposed_csv_text,
                "current_network_text": current_network_text,
                "proposed_network_text": proposed_network_text,
            }))
        },
        &mut errors,
        &mut warnings,
    );

    let rust_validation = envelope_or_build(
        payload,
        "rust_validation",
        "validate-files",
        || {
            validate_files_payload(&json!({
                "config": config.clone(),
                "csv_text": proposed_csv_text,
                "network_text": proposed_network_text,
            }))
        },
        &mut errors,
        &mut warnings,
    );

    let (policy_result, policy_errors, policy_warnings) = evaluate_policy_payload(&json!({
        "config": config.clone(),
        "preflight": preflight.clone(),
        "collector_trust": collector_trust.clone(),
        "cleanup": cleanup.clone(),
        "rust_validation": rust_validation.clone(),
        "python_policy_decision": python_policy_decision.clone(),
        "diff_summary": diff_summary.clone(),
    }));
    merge_diags(&mut errors, policy_errors.clone());
    merge_diags(&mut warnings, policy_warnings.clone());
    let rust_policy_shadow = response_envelope(
        "evaluate-policy",
        policy_result.clone(),
        &policy_errors,
        &policy_warnings,
    );

    let (enforced, fail_closed_when_enforced, authority_mode) = rust_core_authority(&config);
    let (sync_plan_result, sync_plan_errors, sync_plan_warnings) =
        evaluate_sync_plan_payload(&json!({
            "config": config.clone(),
            "mode": mode,
            "files_changed": files_changed,
            "csv_changed": csv_changed,
            "network_changed": network_changed,
            "rust_diff": rust_diff.clone(),
            "rust_validation": rust_validation.clone(),
            "rust_policy_shadow": rust_policy_shadow.clone(),
            "rust_circuit_shadow": rust_circuit_shadow.clone(),
            "collector_trust": collector_trust.clone(),
            "preflight": preflight.clone(),
            "cleanup": cleanup.clone(),
            "authority": {
                "enabled": enforced,
                "fail_closed_when_enforced": fail_closed_when_enforced,
                "authority_mode": authority_mode.clone(),
            },
        }));
    merge_diags(&mut errors, sync_plan_errors.clone());
    merge_diags(&mut warnings, sync_plan_warnings.clone());
    let rust_sync_plan = response_envelope(
        "evaluate-sync-plan",
        sync_plan_result.clone(),
        &sync_plan_errors,
        &sync_plan_warnings,
    );

    let rust_authority_gate = build_authority_gate(&config, mode, &rust_sync_plan);

    let (apply_manifest_result, apply_manifest_errors, apply_manifest_warnings) =
        build_apply_manifest_payload(&json!({
            "config": config.clone(),
            "mode": mode,
            "paths": paths.clone(),
            "state": state.clone(),
            "current_csv_text": current_csv_text,
            "proposed_csv_text": proposed_csv_text,
            "current_network_text": current_network_text,
            "proposed_network_text": proposed_network_text,
            "files_changed": files_changed,
            "csv_changed": csv_changed,
            "network_changed": network_changed,
            "policy_decision": python_policy_decision.clone(),
            "rust_sync_plan": rust_sync_plan.clone(),
            "rust_authority_gate": rust_authority_gate.clone(),
        }));
    merge_diags(&mut errors, apply_manifest_errors.clone());
    merge_diags(&mut warnings, apply_manifest_warnings.clone());
    let rust_apply_manifest = response_envelope(
        "build-apply-manifest",
        apply_manifest_result.clone(),
        &apply_manifest_errors,
        &apply_manifest_warnings,
    );
    let authority_block = rust_authority_gate
        .get("should_block")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let authority_reason = rust_authority_gate
        .get("reason")
        .cloned()
        .unwrap_or_else(|| json!("shadow_only"));
    let manifest_status = apply_manifest_result
        .get("status")
        .cloned()
        .unwrap_or_else(|| json!("unknown"));
    let duration_ms = round_duration_ms(started.elapsed().as_secs_f64());

    let result = json!({
        "mode": mode,
        "authoritative": false,
        "status": if authority_block {
            "blocked_by_authority_gate"
        } else {
            rust_apply_manifest.pointer("/result/status").and_then(Value::as_str).unwrap_or("shadow_preview_ready")
        },
        "files_changed": files_changed,
        "csv_changed": csv_changed,
        "network_changed": network_changed,
        "rust_core_diff": rust_diff,
        "rust_core_validation": rust_validation,
        "rust_policy_shadow": rust_policy_shadow,
        "rust_sync_plan": rust_sync_plan,
        "rust_authority_gate": rust_authority_gate,
        "rust_apply_manifest": rust_apply_manifest,
        "summary": {
            "sync_plan_verdict": sync_plan_result.get("verdict").cloned().unwrap_or_else(|| json!("unknown")),
            "sync_plan_risk_level": sync_plan_result.get("risk_level").cloned().unwrap_or_else(|| json!("unknown")),
            "authority_reason": authority_reason,
            "authority_block": authority_block,
            "manifest_status": manifest_status,
            "duration_ms": duration_ms,
        },
    });

    (result, errors, warnings)
}

fn round_duration_ms(seconds: f64) -> f64 {
    (seconds * 1000.0 * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_sync_engine_shadow_preview_bundle() {
        let payload = json!({
            "config": {
                "paths": {
                    "shaped_devices_csv": "/opt/libreqos/src/ShapedDevices.csv",
                    "network_json": "/opt/libreqos/src/network.json",
                    "runtime_state": "/opt/LQoSync/state/runtime_state.json"
                }
            },
            "mode": "apply",
            "current_csv_text": "a",
            "proposed_csv_text": "b",
            "current_network_text": "{}",
            "proposed_network_text": "{\"n\":1}",
            "files_changed": true,
            "csv_changed": true,
            "network_changed": true,
            "preflight": {"errors": [], "warnings": []},
            "collector_trust": [],
            "cleanup": {"removed": 0, "queued": 0, "preserved": 0, "candidates": 0},
            "policy_decision": {"write_allowed": true, "apply_allowed": true, "verdict": "safe_to_apply", "risk_level": "low"},
            "rust_circuit_shadow": {"available": true, "ok": true, "result": {"normalized_count": 1}, "errors": [], "warnings": []},
            "rust_validation": {"op": "validate-files", "available": true, "ok": true, "result": {"write_allowed": true, "apply_allowed": true}, "errors": [], "warnings": []},
            "rust_diff": {"op": "diff-files", "available": true, "ok": true, "result": {"csv": {"added_count": 1, "updated_count": 0, "removed_count": 0}, "network": {"changed": true}}, "errors": [], "warnings": []}
        });

        let (result, errors, warnings) = build_rust_sync_engine_shadow_preview_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            result
                .pointer("/rust_sync_plan/result/verdict")
                .and_then(Value::as_str),
            Some("ready_by_shadow_plan")
        );
        assert_eq!(
            result
                .pointer("/rust_apply_manifest/result/status")
                .and_then(Value::as_str),
            Some("ready")
        );
        assert_eq!(
            result
                .pointer("/rust_authority_gate/should_block")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn enforces_authority_gate_when_sync_plan_blocks() {
        let payload = json!({
            "config": {
                "rust_core": {
                    "enforce_sync_plan": true,
                    "fail_closed_when_enforced": true,
                    "authority_mode": "enforce_blockers"
                },
                "paths": {
                    "shaped_devices_csv": "/opt/libreqos/src/ShapedDevices.csv",
                    "network_json": "/opt/libreqos/src/network.json",
                    "runtime_state": "/opt/LQoSync/state/runtime_state.json"
                }
            },
            "mode": "apply",
            "current_csv_text": "a",
            "proposed_csv_text": "b",
            "current_network_text": "{}",
            "proposed_network_text": "{\"n\":1}",
            "files_changed": true,
            "csv_changed": true,
            "network_changed": true,
            "preflight": {"errors": ["duplicate ip"], "warnings": []},
            "collector_trust": [],
            "cleanup": {"removed": 0, "queued": 0, "preserved": 0, "candidates": 0},
            "policy_decision": {"write_allowed": false, "apply_allowed": false, "verdict": "blocked_by_policy", "risk_level": "critical"},
            "rust_circuit_shadow": {"available": true, "ok": true, "result": {"normalized_count": 1}, "errors": [], "warnings": []},
            "rust_validation": {"op": "validate-files", "available": true, "ok": true, "result": {"write_allowed": true, "apply_allowed": true}, "errors": [], "warnings": []},
            "rust_diff": {"op": "diff-files", "available": true, "ok": true, "result": {"csv": {"added_count": 1, "updated_count": 0, "removed_count": 0}, "network": {"changed": true}}, "errors": [], "warnings": []}
        });

        let (result, errors, warnings) = build_rust_sync_engine_shadow_preview_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            result
                .pointer("/rust_sync_plan/result/verdict")
                .and_then(Value::as_str),
            Some("blocked_by_shadow_plan")
        );
        assert_eq!(
            result
                .pointer("/rust_authority_gate/should_block")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            result
                .pointer("/rust_apply_manifest/result/status")
                .and_then(Value::as_str),
            Some("blocked_by_authority_gate")
        );
        assert_eq!(
            result.get("status").and_then(Value::as_str),
            Some("blocked_by_authority_gate")
        );
    }
}
