import json
import os
import time
from pathlib import Path

from engine.config_loader import load_config
from engine.context import SyncContext
from engine.result import SyncResult
from engine.diff import diff_rows, diff_network
from engine.hash_utils import sha256_text
from engine.state import update_state, load_state
from engine.logging_utils import log_event

from collectors.mikrotik_client import connect_to_router
from collectors.pppoe import process_pppoe_users
from collectors.hotspot import process_hotspot_users
from collectors.dhcp import process_dhcp_leases

from builders.shaped_devices import read_shaped_devices_csv, render_shaped_devices_csv, count_by_comment
from builders.network_json import read_network_json, render_network_json, ensure_router_root, ensure_router_node, count_nodes
from rules.network_mode import get_network_mode, is_deep_hierarchy
from rules.cleanup import remove_inactive_entries, remove_inactive_entries_by_source
from validators.preflight import run_preflight
from applier.backup import create_backup
from applier.atomic_writer import atomic_write_text
from applier.libreqos_runner import run_libreqos_update
from engine.lockfile import InterProcessLock, LockBusy
from engine.audit import write_audit
from engine.change_summary import build_client_change_summary
from engine.collector_cache import cache_path as collector_cache_path, load_cache as load_collector_cache, save_cache as save_collector_cache
from engine.policy_state import load_policy_state, save_policy_state, prune_expired, cleanup_queue_remove
from engine.policy_engine import (
    build_cleanup_candidates, evaluate_cleanup_policy, evaluate_apply_guards,
    evaluate_auto_apply_policy,
    existing_source_counts, update_successful_source_counts,
)
from engine.insights import compute_smart_insights
from engine.lifecycle import update_lifecycle_state
from engine.rust_core import (
    diagnostics_to_messages, validate_collector_output,
    collector_output_envelope, rust_normalize_circuits, rust_execute_apply_transaction, rust_build_transaction_journal, rust_append_transaction_journal, rust_build_rollback_manifest, rust_build_run_cycle_rust_shadow_report, rust_build_sync_engine_shadow_preview,
    rust_native_dry_run_preview,
)


class Timeline:
    def __init__(self, result: SyncResult):
        self.result = result

    def record(self, name: str, start: float, status: str = "ok", details: dict | None = None):
        ms = round((time.perf_counter() - start) * 1000, 3)
        self.result.timings[name] = ms
        self.result.timeline.append({"step": name, "duration_ms": ms, "status": status, "details": details or {}})
        return ms


def read_text(path):
    p = Path(path)
    if not p.exists():
        return ""
    return p.read_text(encoding="utf-8", errors="ignore")


def _rust_native_dry_run_enabled(config: dict) -> bool:
    return bool((config.get("rust_core") or {}).get("native_dry_run_preview_enabled", False))


def _sync_result_from_rust_native_preview(preview: dict) -> SyncResult:
    result = SyncResult(mode="dry_run")
    result.csv_changed = bool(preview.get("csv_changed", False))
    result.network_changed = bool(preview.get("network_changed", False))
    result.files_changed = bool(preview.get("files_changed", result.csv_changed or result.network_changed))
    result.warnings = [str(item) for item in (preview.get("warnings") or []) if str(item).strip()]
    result.errors = [str(item) for item in (preview.get("errors") or []) if str(item).strip()]
    if isinstance(preview.get("counts"), dict):
        for key, value in preview.get("counts", {}).items():
            try:
                result.counts[str(key)] = int(value)
            except Exception:
                continue
    if isinstance(preview.get("router_errors"), list):
        result.router_errors = [item for item in preview.get("router_errors", []) if isinstance(item, dict)]
    result.routers_processed = int(preview.get("routers_processed") or 0)
    result.diff = preview.get("diff") if isinstance(preview.get("diff"), dict) else {}
    result.node_math = preview.get("node_math") if isinstance(preview.get("node_math"), dict) else {}
    result.timings = preview.get("timings") if isinstance(preview.get("timings"), dict) else {}
    result.meta = {"source": str(preview.get("source") or "rust_native_preview")}
    result.finish(str(preview.get("status") or "dry_run_complete"))
    try:
        result.duration_seconds = float(preview.get("duration_seconds") or result.duration_seconds or 0.0)
    except Exception:
        pass
    return result


def _drift_check(config: dict, state: dict, current_csv_text: str, current_network_text: str, result: SyncResult) -> bool:
    """Return True if apply may continue. Drift is only enforced when previous hashes exist."""
    policy = config.get("app", {}).get("file_drift_policy", "overwrite_with_backup")
    last_hashes = state.get("last_file_hashes") or {}
    if not last_hashes:
        return True
    current_hashes = {
        "csv": sha256_text(current_csv_text),
        "network": sha256_text(current_network_text),
    }
    drifted = []
    if last_hashes.get("csv") and last_hashes.get("csv") != current_hashes["csv"]:
        drifted.append("ShapedDevices.csv")
    if last_hashes.get("network") and last_hashes.get("network") != current_hashes["network"]:
        drifted.append("network.json")
    if not drifted:
        return True
    msg = "External file drift detected: " + ", ".join(drifted)
    if policy == "block":
        result.errors.append(msg + "; apply blocked by file_drift_policy=block")
        return False
    if policy == "warn_only":
        result.warnings.append(msg + "; continuing by policy warn_only")
        return True
    result.warnings.append(msg + "; current files will be backed up before overwrite")
    return True


def _libreqos_should_apply(config: dict, state: dict, result: SyncResult, mode: str) -> tuple[bool, str]:
    """Decide whether LibreQoS.py should run after a non-dry-run cycle.

    Policy:
      - Dry-run never applies.
      - If app.auto_apply is enabled and files changed, apply immediately.
      - If the last LibreQoS apply failed, keep a pending apply marker and retry
        even when files are unchanged. This closes the gap where files were
        written successfully but LibreQoS.py failed on the first apply attempt.
      - Manual force_apply mode always applies.
    """
    if mode == "dry_run":
        return False, "dry_run"
    lib = config.get("libreqos", {})
    if mode == "force_apply":
        return True, "force_apply"
    if not bool(config.get("app", {}).get("auto_apply", True)):
        return False, "auto_apply_disabled"
    if result.files_changed:
        return True, "files_changed"
    retry_failed = bool(lib.get("retry_if_last_apply_failed", True))
    pending = bool(state.get("pending_libreqos_apply") or state.get("last_libreqos_apply_failed"))
    if retry_failed and pending:
        return True, "retry_pending_failed_apply"
    return False, "no_changes"


def _mark_libreqos_state(state_path: str, result: SyncResult, ok: bool, reason: str, run_id: str | None = None):
    update_state(
        state_path,
        last_libreqos_apply_success=bool(ok),
        last_libreqos_apply_failed=not bool(ok),
        pending_libreqos_apply=not bool(ok),
        last_libreqos_apply_reason=reason,
        last_libreqos_exit_code=result.libreqos_exit_code,
        last_libreqos_run_id=run_id,
    )


def _rust_authority_supervisor_preflight(config: dict, result: SyncResult) -> bool:
    """Validate the Rust authority preflight stamp when explicitly required.

    This is intentionally opt-in through rust_core.require_rust_authority_preflight.
    Promotion scripts enable it after writing a fresh stamp. Existing installs that
    merely upgrade packages are not broken by a missing stamp unless the operator
    has opted into the supervisor gate.
    """
    rc = config.get("rust_core", {}) or {}
    if not bool(rc.get("full_rust_authority_supervisor_enabled", True)):
        return True
    if not bool(rc.get("require_rust_authority_preflight", False)):
        result.diff["rust_authority_supervisor"] = {
            "enabled": True,
            "require_preflight": False,
            "status": "not_required",
        }
        return True

    paths = config.get("paths", {}) or {}
    stamp_path = str(rc.get("rust_authority_preflight_stamp") or paths.get("rust_authority_preflight_stamp") or "/opt/LQoSync/state/rust_authority_preflight.json")
    max_age = int(rc.get("rust_authority_preflight_max_age_seconds") or 900)
    fail_closed = bool(rc.get("fail_closed_on_authority_preflight_failure", True))
    supervisor = {
        "enabled": True,
        "require_preflight": True,
        "stamp_path": stamp_path,
        "max_age_seconds": max_age,
        "fail_closed": fail_closed,
        "status": "unknown",
    }
    try:
        stamp = json.loads(Path(stamp_path).read_text(encoding="utf-8"))
        supervisor["stamp"] = {
            "status": stamp.get("status"),
            "created_at": stamp.get("created_at"),
            "created_epoch": stamp.get("created_epoch"),
            "self_test_status": stamp.get("self_test_status"),
            "git_head": stamp.get("git_head"),
        }
        age = max(0, int(time.time()) - int(stamp.get("created_epoch") or 0))
        supervisor["age_seconds"] = age
        if stamp.get("status") != "pass":
            supervisor["status"] = "failed_stamp"
            raise ValueError("preflight stamp status is not pass")
        if stamp.get("self_test_status") != "ok":
            supervisor["status"] = "failed_self_test"
            raise ValueError("preflight stamp self_test_status is not ok")
        if max_age > 0 and age > max_age:
            supervisor["status"] = "stale"
            raise ValueError(f"preflight stamp is stale: {age}s > {max_age}s")
        supervisor["status"] = "ok"
        result.diff["rust_authority_supervisor"] = supervisor
        return True
    except Exception as exc:
        supervisor["error"] = str(exc)
        result.diff["rust_authority_supervisor"] = supervisor
        msg = f"Rust authority supervisor preflight failed: {exc}. Run scripts/rust-full-authority-preflight.sh --write-stamp after verifying Rust core."
        if fail_closed:
            result.errors.append(msg)
            return False
        result.warnings.append(msg)
        return True



def _rust_authority_watchdog(config: dict, result: SyncResult) -> bool:
    """Non-mutating runtime watchdog for promoted Rust full-authority mode.

    The watchdog is intentionally opt-in through rust_core.rust_authority_watchdog_enabled.
    Promotion scripts enable it after creating a recovery bundle and a fresh preflight
    stamp. This keeps package upgrades safe while making promoted live mutation fail
    closed when the recovery/journal evidence is missing.
    """
    rc = config.get("rust_core", {}) or {}
    if not bool(rc.get("rust_authority_watchdog_enabled", False)):
        result.diff["rust_authority_watchdog"] = {"enabled": False, "status": "not_enabled"}
        return True

    paths = config.get("paths", {}) or {}
    now = int(time.time())
    fail_closed = bool(rc.get("fail_closed_on_authority_watchdog_failure", True))
    watchdog = {
        "enabled": True,
        "fail_closed": fail_closed,
        "checks": [],
        "status": "unknown",
    }
    failures: list[str] = []

    def check(name: str, ok: bool, detail: str = ""):
        watchdog["checks"].append({"name": name, "ok": bool(ok), "detail": detail})
        if not ok:
            failures.append(f"{name}: {detail}")

    if bool(rc.get("rust_authority_watchdog_require_fresh_preflight", True)):
        stamp_path = str(rc.get("rust_authority_preflight_stamp") or paths.get("rust_authority_preflight_stamp") or "/opt/LQoSync/state/rust_authority_preflight.json")
        max_age = int(rc.get("rust_authority_watchdog_max_preflight_age_seconds") or rc.get("rust_authority_preflight_max_age_seconds") or 900)
        try:
            stamp = json.loads(Path(stamp_path).read_text(encoding="utf-8"))
            age = max(0, now - int(stamp.get("created_epoch") or 0))
            watchdog["preflight_stamp"] = {
                "path": stamp_path,
                "status": stamp.get("status"),
                "self_test_status": stamp.get("self_test_status"),
                "age_seconds": age,
                "max_age_seconds": max_age,
            }
            check("preflight_stamp_status", stamp.get("status") == "pass", str(stamp.get("status")))
            check("preflight_stamp_self_test", stamp.get("self_test_status") == "ok", str(stamp.get("self_test_status")))
            check("preflight_stamp_fresh", max_age <= 0 or age <= max_age, f"age={age}s max={max_age}s")
        except Exception as exc:
            watchdog["preflight_stamp"] = {"path": stamp_path, "error": str(exc)}
            check("preflight_stamp_readable", False, str(exc))

    if bool(rc.get("rust_authority_watchdog_require_recovery_bundle", True)):
        root = Path(str(rc.get("rust_authority_recovery_bundle_dir") or "/opt/LQoSync/state/rust_authority_recovery"))
        latest = None
        if root.exists() and root.is_dir():
            candidates = sorted([p for p in root.iterdir() if p.is_dir()], key=lambda p: p.name, reverse=True)
            latest = candidates[0] if candidates else None
        watchdog["recovery_bundle"] = {"root": str(root), "latest": str(latest) if latest else None}
        check("recovery_bundle_root", root.exists() and root.is_dir(), str(root))
        check("recovery_bundle_latest", latest is not None, str(latest) if latest else "none")
        if latest is not None:
            manifest = latest / "MANIFEST.json"
            check("recovery_bundle_manifest", manifest.exists() and manifest.is_file(), str(manifest))

    if bool(rc.get("rust_authority_watchdog_require_transaction_journal_path", True)):
        journal = Path(str(paths.get("transaction_journal") or "/opt/LQoSync/logs/transaction_journal.jsonl"))
        parent = journal.parent
        watchdog["transaction_journal"] = {"path": str(journal), "parent": str(parent)}
        check("transaction_journal_parent", parent.exists() and parent.is_dir(), str(parent))
        check("transaction_journal_parent_writable", parent.exists() and parent.is_dir() and os.access(parent, os.W_OK), str(parent))
        if bool(rc.get("append_transaction_journal")) or bool(rc.get("allow_transaction_journal_writes")):
            check("transaction_journal_authority_flags", bool(rc.get("append_transaction_journal")) and bool(rc.get("allow_transaction_journal_writes")), "append_transaction_journal and allow_transaction_journal_writes must both be true")

    watchdog["failure_count"] = len(failures)
    watchdog["failures"] = failures
    watchdog["status"] = "ok" if not failures else "failed"
    result.diff["rust_authority_watchdog"] = watchdog
    if failures and fail_closed:
        result.errors.append("Rust authority watchdog failed: " + "; ".join(failures[:5]))
        return False
    if failures:
        result.warnings.append("Rust authority watchdog warnings: " + "; ".join(failures[:5]))
    return True



def _rust_authority_quarantine_path(config: dict) -> Path:
    rc = config.get("rust_core", {}) or {}
    paths = config.get("paths", {}) or {}
    return Path(str(rc.get("rust_authority_quarantine_state") or paths.get("rust_authority_quarantine_state") or "/opt/LQoSync/state/rust_authority_quarantine.json"))


def _rust_authority_mark_quarantine(config: dict, status: str, result: SyncResult, details: dict | None = None) -> None:
    """Write a non-destructive quarantine marker after critical Rust authority failures.

    Quarantine only activates when rust_core.rust_authority_quarantine_enabled and
    rust_core.rust_authority_auto_quarantine_on_failure are both true. This keeps
    package upgrades no-breakage while promoted live-stable installs can fail closed
    after a critical authority/apply failure.
    """
    rc = config.get("rust_core", {}) or {}
    if not bool(rc.get("rust_authority_quarantine_enabled", False)):
        return
    if not bool(rc.get("rust_authority_auto_quarantine_on_failure", True)):
        return
    statuses = rc.get("rust_authority_failure_quarantine_statuses") or []
    if statuses and status not in set(str(s) for s in statuses):
        return
    try:
        path = _rust_authority_quarantine_path(config)
        path.parent.mkdir(parents=True, exist_ok=True)
        payload = {
            "schema": "lqosync.rust_authority_quarantine.v1",
            "active": True,
            "status": status,
            "created_epoch": int(time.time()),
            "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "reason": "critical Rust authority failure; live-stable mutation blocked until operator review",
            "last_error": status,
            "result_status": getattr(result, "status", None),
            "errors": list(getattr(result, "errors", []) or [])[-10:],
            "warnings": list(getattr(result, "warnings", []) or [])[-10:],
            "details": details or {},
        }
        path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
        result.diff["rust_authority_quarantine"] = {"active": True, "path": str(path), "status": status}
    except Exception as exc:
        result.warnings.append(f"Failed to write Rust authority quarantine marker: {exc}")


def _rust_authority_live_stable_gate(config: dict, state: dict, result: SyncResult) -> bool:
    """Fail-closed production gate for the v7.7 live-stable candidate path.

    This gate is intentionally opt-in through rust_core.rust_live_stable_candidate_enabled.
    It blocks live mutation when quarantine is active, watchdog evidence is missing,
    recovery bundles are absent, or recent failure counters exceed the configured threshold.
    """
    rc = config.get("rust_core", {}) or {}
    if not bool(rc.get("rust_live_stable_candidate_enabled", False)):
        result.diff["rust_live_stable_gate"] = {"enabled": False, "status": "not_enabled"}
        return True

    fail_closed = bool(rc.get("rust_live_stable_fail_closed", True))
    gate = {"enabled": True, "fail_closed": fail_closed, "checks": [], "status": "unknown"}
    failures: list[str] = []

    def check(name: str, ok: bool, detail: str = ""):
        gate["checks"].append({"name": name, "ok": bool(ok), "detail": detail})
        if not ok:
            failures.append(f"{name}: {detail}")

    qpath = _rust_authority_quarantine_path(config)
    quarantine_active = False
    if qpath.exists():
        try:
            qdata = json.loads(qpath.read_text(encoding="utf-8"))
            quarantine_active = bool(qdata.get("active", False))
            gate["quarantine"] = {"path": str(qpath), "active": quarantine_active, "status": qdata.get("status"), "created_at": qdata.get("created_at")}
        except Exception as exc:
            quarantine_active = True
            gate["quarantine"] = {"path": str(qpath), "active": True, "error": str(exc)}
    else:
        gate["quarantine"] = {"path": str(qpath), "active": False, "status": "missing_ok"}
    check("quarantine_clear", not quarantine_active, str(gate.get("quarantine")))

    if bool(rc.get("rust_live_stable_require_watchdog", True)):
        watchdog = result.diff.get("rust_authority_watchdog") or {}
        check("watchdog_ok", watchdog.get("status") == "ok", str(watchdog.get("status")))

    if bool(rc.get("rust_live_stable_require_recovery_bundle", True)):
        root = Path(str(rc.get("rust_authority_recovery_bundle_dir") or "/opt/LQoSync/state/rust_authority_recovery"))
        latest = None
        if root.exists() and root.is_dir():
            dirs = sorted([d for d in root.iterdir() if d.is_dir()], key=lambda d: d.name, reverse=True)
            latest = dirs[0] if dirs else None
        gate["recovery_bundle"] = {"root": str(root), "latest": str(latest) if latest else None}
        check("recovery_bundle_available", latest is not None and (latest / "MANIFEST.json").exists(), str(latest) if latest else "none")

    if bool(rc.get("rust_live_stable_require_last_good_snapshot", False)):
        root = Path(str(rc.get("rust_authority_last_good_snapshot_dir") or "/opt/LQoSync/state/rust_authority_last_good"))
        latest = None
        if root.exists() and root.is_dir():
            dirs = sorted([d for d in root.iterdir() if d.is_dir()], key=lambda d: d.name, reverse=True)
            latest = dirs[0] if dirs else None
        gate["last_good_snapshot"] = {"root": str(root), "latest": str(latest) if latest else None}
        check("last_good_snapshot_available", latest is not None and (latest / "MANIFEST.json").exists(), str(latest) if latest else "none")

    max_failures = int(rc.get("rust_live_stable_max_recent_failures") or 0)
    recent_failures = state.get("rust_authority_recent_failures") or []
    if isinstance(recent_failures, list):
        count = len(recent_failures)
        gate["recent_failures"] = {"count": count, "max": max_failures, "items": recent_failures[-5:]}
        check("recent_failure_budget", count <= max_failures, f"count={count} max={max_failures}")

    gate["failure_count"] = len(failures)
    gate["failures"] = failures
    gate["status"] = "ok" if not failures else "failed"
    result.diff["rust_live_stable_gate"] = gate
    if failures and fail_closed:
        result.errors.append("Rust live-stable gate failed: " + "; ".join(failures[:6]))
        return False
    if failures:
        result.warnings.append("Rust live-stable gate warnings: " + "; ".join(failures[:6]))
    return True



def _rust_set_and_forget_gate(config: dict, result: SyncResult) -> bool:
    """Fail-closed v7.8 set-and-forget evidence gate.

    This gate is opt-in through rust_core.rust_set_and_forget_candidate_enabled.
    It requires an evidence bundle generated by scripts/rust-set-and-forget-readiness.sh
    and optionally verifies live-soak, journal-audit, rollback-drill, and last-good evidence.
    """
    rc = config.get("rust_core", {}) or {}
    if not bool(rc.get("rust_set_and_forget_candidate_enabled", False)):
        result.diff["rust_set_and_forget_gate"] = {"enabled": False, "status": "not_enabled"}
        return True

    fail_closed = bool(rc.get("rust_set_and_forget_fail_closed", True))
    gate = {"enabled": True, "fail_closed": fail_closed, "checks": [], "status": "unknown"}
    failures: list[str] = []
    now = int(time.time())

    def check(name: str, ok: bool, detail: str = ""):
        gate["checks"].append({"name": name, "ok": bool(ok), "detail": detail})
        if not ok:
            failures.append(f"{name}: {detail}")

    evidence_path = Path(str(rc.get("rust_set_and_forget_readiness_evidence") or "/opt/LQoSync/state/rust_set_and_forget_readiness.json"))
    evidence = {}
    try:
        evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
        gate["evidence"] = {"path": str(evidence_path), "status": evidence.get("status"), "created_at": evidence.get("created_at")}
        max_age = int(rc.get("rust_set_and_forget_max_evidence_age_seconds") or 1800)
        age = max(0, now - int(evidence.get("created_epoch") or 0))
        check("readiness_evidence_pass", evidence.get("status") == "pass", str(evidence.get("status")))
        check("readiness_evidence_fresh", max_age <= 0 or age <= max_age, f"age={age} max={max_age}")
    except Exception as exc:
        gate["evidence"] = {"path": str(evidence_path), "error": str(exc)}
        check("readiness_evidence_readable", False, str(exc))

    qpath = _rust_authority_quarantine_path(config)
    if qpath.exists():
        try:
            qdata = json.loads(qpath.read_text(encoding="utf-8"))
            active = bool(qdata.get("active", False))
            gate["quarantine"] = {"path": str(qpath), "active": active, "status": qdata.get("status")}
            check("quarantine_clear", not active, str(gate["quarantine"]))
        except Exception as exc:
            check("quarantine_readable", False, str(exc))
    else:
        gate["quarantine"] = {"path": str(qpath), "active": False, "status": "missing_ok"}
        check("quarantine_clear", True, "missing_ok")

    checks = evidence.get("checks") if isinstance(evidence, dict) else {}
    if not isinstance(checks, dict):
        checks = {}
    mapping = [
        ("rust_set_and_forget_require_live_soak_monitor", "live_soak_monitor"),
        ("rust_set_and_forget_require_journal_audit", "journal_audit"),
        ("rust_set_and_forget_require_rollback_drill", "rollback_drill"),
        ("rust_set_and_forget_require_last_good_snapshot", "last_good_snapshot"),
    ]
    for flag, evidence_key in mapping:
        if bool(rc.get(flag, True)):
            item = checks.get(evidence_key) or {}
            check(f"evidence_{evidence_key}", item.get("ok") is True, str(item))

    gate["failure_count"] = len(failures)
    gate["failures"] = failures
    gate["status"] = "ok" if not failures else "failed"
    result.diff["rust_set_and_forget_gate"] = gate
    if failures and fail_closed:
        result.errors.append("Rust set-and-forget gate failed: " + "; ".join(failures[:6]))
        return False
    if failures:
        result.warnings.append("Rust set-and-forget gate warnings: " + "; ".join(failures[:6]))
    return True

def _rust_authority_record_last_good_snapshot(config: dict, result: SyncResult) -> None:
    rc = config.get("rust_core", {}) or {}
    if not bool(rc.get("rust_live_stable_candidate_enabled", False)):
        return
    try:
        paths = config.get("paths", {}) or {}
        root = Path(str(rc.get("rust_authority_last_good_snapshot_dir") or "/opt/LQoSync/state/rust_authority_last_good"))
        ts = time.strftime("%Y%m%d_%H%M%S", time.gmtime())
        d = root / ts
        d.mkdir(parents=True, exist_ok=True)
        manifest = {
            "schema": "lqosync.rust_authority_last_good.v1",
            "created_epoch": int(time.time()),
            "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "status": result.status,
            "files_changed": bool(result.files_changed),
            "libreqos_triggered": bool(result.libreqos_triggered),
            "libreqos_exit_code": result.libreqos_exit_code,
            "file_hashes": result.file_hashes,
            "paths": {k: paths.get(k) for k in ("shaped_devices_csv", "network_json", "runtime_state", "transaction_journal")},
        }
        for key, src_name in (("shaped_devices_csv", "ShapedDevices.csv"), ("network_json", "network.json"), ("runtime_state", "runtime_state.json")):
            src = paths.get(key)
            if src and Path(str(src)).exists():
                target = d / src_name
                target.write_text(Path(str(src)).read_text(encoding="utf-8", errors="ignore"), encoding="utf-8")
                manifest.setdefault("included_files", []).append(str(target.name))
        (d / "MANIFEST.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        result.diff["rust_authority_last_good_snapshot"] = {"path": str(d), "status": "created"}
    except Exception as exc:
        result.warnings.append(f"Failed to create Rust authority last-good snapshot: {exc}")

def _run_libreqos_apply(config: dict, state_path: str, result: SyncResult, timeline: Timeline, reason: str):
    t = time.perf_counter()
    lq = run_libreqos_update(config)
    timeline.record("libreqos_apply", t, status="ok" if lq.get("ok") else "failed", details={"exit_code": lq.get("exit_code"), "run_id": lq.get("run_id"), "reason": reason, "working_dir": lq.get("working_dir")})
    result.libreqos_triggered = True
    result.libreqos_exit_code = lq["exit_code"]
    result.libreqos_stdout = lq["stdout"]
    result.libreqos_stderr = lq["stderr"]
    result.diff["libreqos_command"] = lq.get("command")
    result.diff["libreqos_run_id"] = lq.get("run_id")
    result.diff["libreqos_duration_ms"] = lq.get("duration_ms")
    result.diff["libreqos_apply_reason"] = reason
    result.diff["libreqos_working_dir"] = lq.get("working_dir")
    if lq["ok"]:
        _mark_libreqos_state(state_path, result, True, reason, lq.get("run_id"))
    else:
        _mark_libreqos_state(state_path, result, False, reason, lq.get("run_id"))
    return lq


def _run_cycle_unlocked(mode="apply", config_path=None):
    cycle_start = time.perf_counter()
    result = SyncResult(mode=mode)
    timeline = Timeline(result)
    t = time.perf_counter()
    config = load_config(config_path)
    timeline.record("config_load", t)
    paths = config["paths"]
    state_path = paths.get("runtime_state", "state/runtime_state.json")

    if mode == "dry_run" and _rust_native_dry_run_enabled(config):
        log_event(config, "info", "Starting Rust-native dry-run cycle")
        write_audit(config, "sync_started", details={"mode": mode, "engine": "rust_native_preview"})
        try:
            update_state(state_path, sync_running=True, scheduler_state="running")
        except Exception:
            pass
        preview = rust_native_dry_run_preview(config)
        result = _sync_result_from_rust_native_preview(preview)
        result.diff.setdefault("rust_native_dry_run_short_circuit", {
            "enabled": True,
            "source": preview.get("source", "rust_native_preview"),
        })
        log_event(
            config,
            "info",
            f"Rust-native dry-run complete: csv_changed={result.csv_changed} network_changed={result.network_changed} status={result.status}",
        )
        write_audit(
            config,
            "dry_run_complete",
            details={
                "engine": "rust_native_preview",
                "csv_changed": result.csv_changed,
                "network_changed": result.network_changed,
                "status": result.status,
                "timings": result.timings,
            },
        )
        update_state(state_path, sync_running=False, scheduler_state="idle", last_dry_run=result.to_dict(), last_error=None)
        return result

    t = time.perf_counter()
    state_before = load_state(state_path)
    timeline.record("state_load", t)

    t = time.perf_counter()
    cache_file = collector_cache_path(config)
    collector_cache = load_collector_cache(cache_file)
    timeline.record("collector_cache_load", t, details={"path": cache_file})

    t = time.perf_counter()
    policy_state = load_policy_state(config)
    prune_expired(policy_state)
    timeline.record("policy_state_load", t, details={"pending_confirmations": len(policy_state.get("pending_confirmations", [])), "queued_cleanup": len(policy_state.get("cleanup_queue", []))})

    log_event(config, "info", f"Starting sync cycle mode={mode}")
    write_audit(config, "sync_started", details={"mode": mode})
    try:
        update_state(state_path, sync_running=True, scheduler_state="running")
    except Exception:
        pass

    try:
        csv_path = paths["shaped_devices_csv"]
        network_path = paths["network_json"]

        t = time.perf_counter()
        current_csv_text = read_text(csv_path)
        timeline.record("csv_read_text", t, details={"path": csv_path})

        t = time.perf_counter()
        current_network_text = read_text(network_path)
        timeline.record("network_read_text", t, details={"path": network_path})

        t = time.perf_counter()
        current_rows = read_shaped_devices_csv(csv_path)
        existing_data = {k: dict(v) for k, v in current_rows.items()}
        timeline.record("csv_parse", t, details={"rows": len(existing_data)})

        t = time.perf_counter()
        current_network = read_network_json(network_path)
        timeline.record("network_parse", t)

        t = time.perf_counter()
        network_mode = get_network_mode(config)
        preserve_network = bool(config.get("preserve_network_config", False))
        if network_mode == "flat_no_parent":
            network_config = json.loads(json.dumps(current_network)) if preserve_network else {}
        elif network_mode == "flat_router_root":
            network_config = json.loads(json.dumps(current_network)) if preserve_network else {}
        else:
            network_config = json.loads(json.dumps(current_network))
        timeline.record("network_mode_prepare", t, details={"network_mode": network_mode})

        ctx = SyncContext(config=config, existing_data=existing_data, network_config=network_config, network_mode=network_mode)
        ctx.cache = collector_cache
        ctx.cache_path = cache_file
        enabled_routers = [r for r in config.get("routers", []) if r.get("enabled", True)]
        if not enabled_routers:
            result.warnings.append("No enabled routers configured. Nothing to sync.")

        t_routers = time.perf_counter()
        for router in enabled_routers:
            router_t = time.perf_counter()
            if network_mode != "flat_no_parent":
                router_node = ensure_router_node(ctx.network_config, router, allow_parent=is_deep_hierarchy(config))
                ctx.router_nodes[router["name"]] = router_node
                if network_mode == "flat_router_root":
                    router_node["children"] = {}
            connect_t = time.perf_counter()
            pool, api, err = connect_to_router(router)
            timeline.record(f"router.{router.get('name','unknown')}.connect", connect_t, status="ok" if api else "failed")
            if not api:
                result.router_errors.append({"router": router.get("name"), "error": err})
                ctx.errors.append(f"Router {router.get('name')} failed: {err}")
                timeline.record(f"router.{router.get('name','unknown')}.total", router_t, status="failed")
                continue
            try:
                router_active = set()
                router_updated = False
                source_success = set()
                for pname, processor in (("pppoe", process_pppoe_users), ("hotspot", process_hotspot_users), ("dhcp", process_dhcp_leases)):
                    pt = time.perf_counter()
                    try:
                        active_codes, updated = processor(api, router, ctx)
                        source_label = {"pppoe": "PPP", "dhcp": "DHCP", "hotspot": "HS"}.get(pname, pname.upper())
                        metric_key = f"{router.get('name','unknown')}.{pname}"
                        source_metrics = ctx.collector_metrics.get(metric_key, {}) if isinstance(ctx.collector_metrics, dict) else {}
                        previous_success_count = int((policy_state.get("last_successful_source_counts", {}) or {}).get(source_label, 0) or 0)
                        trust_envelope = collector_output_envelope(
                            router,
                            source_label,
                            active_codes,
                            previous_success_count=previous_success_count,
                            status="ok",
                            read_counts={k: v for k, v in (source_metrics or {}).items() if k.endswith("_loaded") or k.endswith("_sessions") or k.endswith("_leases") or k.endswith("_users") or k.endswith("_matched")},
                            metrics=source_metrics,
                        )
                        trust = validate_collector_output(config, trust_envelope)
                        result.diff.setdefault("collector_trust", []).append(trust)
                        safe_for_cleanup = bool((trust.get("result") or {}).get("safe_for_cleanup", True))
                        trust_errors, trust_warnings = diagnostics_to_messages(trust)
                        if safe_for_cleanup:
                            source_success.add(source_label)
                        else:
                            result.warnings.append(f"Collector trust guard held cleanup for {router.get('name')}/{source_label}")
                            result.warnings.extend(trust_errors)
                        result.warnings.extend(trust_warnings)
                        ctx.collector_metrics[f"{metric_key}.trust"] = {
                            "source": source_label,
                            "safe_for_cleanup": safe_for_cleanup,
                            "row_count": len(active_codes),
                            "previous_success_count": previous_success_count,
                            "status": (trust.get("result") or {}).get("status"),
                            "warnings": len(trust.get("warnings") or []),
                            "errors": len(trust.get("errors") or []),
                        }
                        timeline.record(
                            f"router.{router.get('name','unknown')}.{pname}",
                            pt,
                            status="ok" if safe_for_cleanup else "cleanup_held",
                            details={
                                "active": len(active_codes),
                                "updated": bool(updated),
                                "source_success": safe_for_cleanup,
                                "safe_for_cleanup": safe_for_cleanup,
                                "trust_warnings": len(trust.get("warnings") or []),
                                "trust_errors": len(trust.get("errors") or []),
                            },
                        )
                        router_active.update(active_codes)
                        router_updated = router_updated or updated
                    except Exception as source_error:
                        source_label = {"pppoe": "PPP", "dhcp": "DHCP", "hotspot": "HS"}.get(pname, pname.upper())
                        result.router_errors.append({"router": router.get("name"), "source": source_label, "error": str(source_error)})
                        ctx.errors.append(f"Router {router.get('name')} {source_label} processing error: {source_error}")
                        timeline.record(f"router.{router.get('name','unknown')}.{pname}", pt, status="failed", details={"error": str(source_error), "source_success": False})
                ctx.active_codes.update(router_active)
                ctx.active_codes_by_router[router["name"]] = router_active
                ctx.source_success_by_router[router["name"]] = source_success
                if source_success:
                    ctx.router_success_names.add(router["name"])
                result.routers_processed += 1
                timeline.record(f"router.{router.get('name','unknown')}.total", router_t, details={"active": len(router_active), "updated": bool(router_updated), "source_success": sorted(source_success)})
            except Exception as e:
                result.router_errors.append({"router": router.get("name"), "error": str(e)})
                ctx.errors.append(f"Router {router.get('name')} processing error: {e}")
                timeline.record(f"router.{router.get('name','unknown')}.total", router_t, status="failed", details={"error": str(e)})
            finally:
                try:
                    pool.disconnect()
                except Exception:
                    pass
        timeline.record("routers_total", t_routers, details={"processed": result.routers_processed})

        t = time.perf_counter()
        enabled_names = {r.get("name") for r in enabled_routers}
        cleanup_sources = set()
        if enabled_names:
            for source in ("PPP", "DHCP", "HS"):
                if all(source in ctx.source_success_by_router.get(name, set()) for name in enabled_names):
                    cleanup_sources.add(source)
        else:
            result.warnings.append("Skipping inactive cleanup: no enabled routers.")

        active_counts_by_source = {src: len(ctx.active_codes_by_source.get(src, set())) for src in ("PPP", "DHCP", "HS")}
        existing_counts_before_cleanup = existing_source_counts(ctx.existing_data, config["defaults"].get("static_comment_value", "static"))
        cleanup_candidates = build_cleanup_candidates(ctx.existing_data, ctx.active_codes_by_source, cleanup_sources, config["defaults"].get("static_comment_value", "static"))
        policy_decision = evaluate_cleanup_policy(config, policy_state, cleanup_candidates, cleanup_sources, active_counts_by_source, existing_counts_before_cleanup)
        remove_codes = set(policy_decision.remove_codes)
        if remove_codes:
            for code in list(remove_codes):
                if code in ctx.existing_data:
                    del ctx.existing_data[code]
            cleanup_queue_remove(policy_state, remove_codes)
            result.files_changed = True
        cleanup_stats = {
            "sources": sorted(cleanup_sources),
            "candidates": len(cleanup_candidates),
            "removed": len(remove_codes),
            "queued": len(policy_decision.queued_codes),
            "preserved": len(policy_decision.preserve_codes),
            "verdict": policy_decision.verdict,
            "risk_level": policy_decision.risk_level,
        }
        ctx.collector_metrics["cleanup"] = cleanup_stats
        skipped_sources = sorted(set(["PPP", "DHCP", "HS"]) - cleanup_sources)
        if skipped_sources:
            result.warnings.append(f"Source-aware cleanup skipped for: {', '.join(skipped_sources)} because not all enabled routers scanned those sources successfully.")
        timeline.record("cleanup_policy", t, details=cleanup_stats)

        t = time.perf_counter()
        proposed_csv_text = render_shaped_devices_csv(ctx.existing_data)
        timeline.record("csv_render", t, details={"rows": len(ctx.existing_data)})

        t = time.perf_counter()
        proposed_network_text = render_network_json(ctx.network_config)
        timeline.record("network_render", t, details={"nodes": count_nodes(ctx.network_config)})

        t = time.perf_counter()
        result.csv_changed = current_csv_text != proposed_csv_text
        result.network_changed = current_network_text != proposed_network_text
        result.files_changed = result.csv_changed or result.network_changed
        collector_trust_results = result.diff.get("collector_trust", []) if isinstance(result.diff, dict) else []
        result.diff = {
            "csv": diff_rows(current_rows, ctx.existing_data),
            "network": diff_network(current_network, ctx.network_config),
            "collector_trust": collector_trust_results,
        }
        client_change_summary = build_client_change_summary(result.diff.get("csv", {}), ctx.meta)
        result.diff["client_change_summary"] = client_change_summary
        result.diff["client_changes"] = client_change_summary.get("changes", [])
        timeline.record("diff", t, details={"csv_changed": result.csv_changed, "network_changed": result.network_changed, "client_changes": client_change_summary.get("counts", {})})

        t = time.perf_counter()
        result.warnings.extend(ctx.warnings)
        result.errors.extend(ctx.errors)
        result.counts.update(ctx.counts)
        result.counts["csv_rows"] = len(ctx.existing_data)
        result.counts["nodes"] = count_nodes(ctx.network_config)
        result.counts.update({f"rows_{k}": v for k, v in count_by_comment(ctx.existing_data).items()})
        result.meta = ctx.meta
        result.node_math = ctx.node_math
        result.diff["collector_metrics"] = ctx.collector_metrics
        result.diff["speed_source_breakdown"] = ctx.speed_source_counts
        result.diff["cache_metrics"] = ctx.cache_metrics
        result.counts["cache_hits"] = int(ctx.cache_metrics.get("hits", 0))
        result.counts["cache_misses"] = int(ctx.cache_metrics.get("misses", 0))

        t_circuit_shadow = time.perf_counter()
        rust_circuit_shadow = rust_normalize_circuits(config, ctx.existing_data, meta=ctx.meta, source="mixed", router="mixed")
        result.diff["rust_circuit_shadow"] = rust_circuit_shadow
        circuit_shadow_errors, circuit_shadow_warnings = diagnostics_to_messages(rust_circuit_shadow)
        if circuit_shadow_errors:
            result.warnings.extend([f"Rust circuit shadow: {msg}" for msg in circuit_shadow_errors])
        if rust_circuit_shadow.get("available"):
            result.warnings.extend([f"Rust circuit shadow: {msg}" for msg in circuit_shadow_warnings])
        timeline.record(
            "rust_circuit_shadow",
            t_circuit_shadow,
            status="ok" if rust_circuit_shadow.get("ok") else ("unavailable" if not rust_circuit_shadow.get("available") else "check"),
            details={
                "available": bool(rust_circuit_shadow.get("available")),
                "ok": bool(rust_circuit_shadow.get("ok")),
                "normalized_count": (rust_circuit_shadow.get("result") or {}).get("normalized_count"),
                "errors": len(rust_circuit_shadow.get("errors") or []),
                "warnings": len(rust_circuit_shadow.get("warnings") or []),
            },
        )
        t_run_cycle_shadow = time.perf_counter()
        rust_run_cycle_shadow = rust_build_run_cycle_rust_shadow_report(
            config,
            {
                "mode": "shadow",
                "python_rows": list(ctx.existing_data.values()),
                "existing_rows": list(ctx.existing_data.values()),
                "collector_trust": result.diff.get("collector_trust", []),
                "collector_metrics": ctx.collector_metrics,
                "collector_parity": {"parity_score": 0.0, "verdict": "not_available"},
            },
        )
        result.diff["rust_run_cycle_shadow_report"] = rust_run_cycle_shadow
        run_cycle_shadow_errors, run_cycle_shadow_warnings = diagnostics_to_messages(rust_run_cycle_shadow)
        if run_cycle_shadow_errors:
            result.warnings.extend([f"Rust run_cycle shadow: {msg}" for msg in run_cycle_shadow_errors])
        if rust_run_cycle_shadow.get("available"):
            result.warnings.extend([f"Rust run_cycle shadow: {msg}" for msg in run_cycle_shadow_warnings])
        timeline.record(
            "rust_run_cycle_shadow_report",
            t_run_cycle_shadow,
            status="ok" if rust_run_cycle_shadow.get("ok") else ("unavailable" if not rust_run_cycle_shadow.get("available") else "check"),
            details={
                "available": bool(rust_run_cycle_shadow.get("available")),
                "ok": bool(rust_run_cycle_shadow.get("ok")),
                "status": (rust_run_cycle_shadow.get("result") or {}).get("status"),
                "python_authoritative": (rust_run_cycle_shadow.get("result") or {}).get("python_run_cycle_authoritative"),
                "rust_row_count": (rust_run_cycle_shadow.get("result") or {}).get("rust_row_count"),
                "live_read_shadow_ready": (rust_run_cycle_shadow.get("result") or {}).get("live_read_shadow_ready"),
                "live_read_shadow_status": (rust_run_cycle_shadow.get("result") or {}).get("live_read_shadow_status"),
                "live_read_shadow_row_count": (rust_run_cycle_shadow.get("result") or {}).get("live_read_shadow_row_count"),
                "errors": len(rust_run_cycle_shadow.get("errors") or []),
                "warnings": len(rust_run_cycle_shadow.get("warnings") or []),
            },
        )

        result.file_hashes = {
            "current_csv": sha256_text(current_csv_text),
            "current_network": sha256_text(current_network_text),
            "proposed_csv": sha256_text(proposed_csv_text),
            "proposed_network": sha256_text(proposed_network_text),
        }
        timeline.record("metadata_hashes", t)

        t = time.perf_counter()
        try:
            save_collector_cache(cache_file, ctx.cache)
            ctx.cache_metrics["writes"] = ctx.cache_metrics.get("writes", 0) + 1
        except Exception as cache_error:
            result.warnings.append(f"Collector cache save failed: {cache_error}")
        timeline.record("collector_cache_save", t, details={"path": cache_file, "cache_metrics": ctx.cache_metrics})

        t = time.perf_counter()
        preflight = run_preflight(ctx.existing_data, ctx.network_config, config)
        result.warnings.extend(preflight["warnings"])
        result.errors.extend(preflight["errors"])
        timeline.record("preflight", t, status="failed" if result.errors else "ok", details={"warnings": len(preflight["warnings"]), "errors": len(preflight["errors"])})

        policy_decision = evaluate_apply_guards(config, policy_decision, preflight, result)
        python_policy_dict = policy_decision.to_dict()
        t = time.perf_counter()
        rust_sync_engine_shadow = rust_build_sync_engine_shadow_preview(
            config,
            mode=mode,
            paths=paths,
            state=state_before,
            current_csv_text=current_csv_text,
            proposed_csv_text=proposed_csv_text,
            current_network_text=current_network_text,
            proposed_network_text=proposed_network_text,
            files_changed=result.files_changed,
            csv_changed=result.csv_changed,
            network_changed=result.network_changed,
            preflight=preflight,
            collector_trust=result.diff.get("collector_trust", []),
            cleanup=cleanup_stats,
            rust_circuit_shadow=result.diff.get("rust_circuit_shadow", {}),
            policy_decision=python_policy_dict,
            diff_summary={
                "csv_changed": result.csv_changed,
                "network_changed": result.network_changed,
                "client_change_summary": result.diff.get("client_change_summary", {}),
            },
            client_change_summary=result.diff.get("client_change_summary", {}),
        )
        result.diff["rust_sync_engine_shadow_preview"] = rust_sync_engine_shadow
        rust_shadow_result = rust_sync_engine_shadow.get("result", {}) if isinstance(rust_sync_engine_shadow, dict) else {}
        rust_diff = rust_shadow_result.get("rust_core_diff", {}) if isinstance(rust_shadow_result.get("rust_core_diff"), dict) else {}
        rust_validation = rust_shadow_result.get("rust_core_validation", {}) if isinstance(rust_shadow_result.get("rust_core_validation"), dict) else {}
        rust_policy_shadow = rust_shadow_result.get("rust_policy_shadow", {}) if isinstance(rust_shadow_result.get("rust_policy_shadow"), dict) else {}
        rust_sync_plan = rust_shadow_result.get("rust_sync_plan", {}) if isinstance(rust_shadow_result.get("rust_sync_plan"), dict) else {}
        rust_authority_gate = rust_shadow_result.get("rust_authority_gate", {}) if isinstance(rust_shadow_result.get("rust_authority_gate"), dict) else {}
        rust_apply_manifest = rust_shadow_result.get("rust_apply_manifest", {}) if isinstance(rust_shadow_result.get("rust_apply_manifest"), dict) else {}
        result.diff["rust_core_diff"] = rust_diff
        result.diff["rust_core_validation"] = rust_validation
        result.diff["rust_policy_shadow"] = rust_policy_shadow
        result.diff["rust_sync_plan"] = rust_sync_plan
        result.diff["rust_authority_gate"] = rust_authority_gate
        result.diff["rust_apply_manifest"] = rust_apply_manifest
        rust_errors, rust_warnings = diagnostics_to_messages(rust_validation)
        if rust_validation.get("available") and not rust_validation.get("ok") and config.get("rust_core", {}).get("enforce_validation", False):
            result.errors.extend(rust_errors)
        elif rust_errors:
            result.warnings.extend(rust_errors)
        if rust_validation.get("available"):
            result.warnings.extend(rust_warnings)
        policy_decision_shadow_result = rust_policy_shadow.get("result", {}) if isinstance(rust_policy_shadow, dict) else {}
        policy_parity = policy_decision_shadow_result.get("parity", {}) if isinstance(policy_decision_shadow_result, dict) else {}
        if policy_parity and policy_parity.get("available") and policy_parity.get("matches_verdict") is False:
            result.warnings.append("Rust policy authority reported a parity mismatch; Python mutation fallback is disabled in stable releases.")
        sync_plan_result = rust_sync_plan.get("result", {}) if isinstance(rust_sync_plan, dict) else {}
        if rust_authority_gate.get("should_block"):
            result.errors.append(rust_authority_gate.get("message") or "Rust sync-plan authority gate blocked this cycle.")
        elif sync_plan_result.get("verdict") == "blocked_by_shadow_plan":
            result.warnings.append("Rust sync-plan authority found blockers; production mutation is fail-closed.")
        timeline.record(
            "rust_sync_engine_shadow_preview",
            t,
            status="ok" if rust_sync_engine_shadow.get("ok") else ("unavailable" if not rust_sync_engine_shadow.get("available") else "check"),
            details={
                "available": bool(rust_sync_engine_shadow.get("available")),
                "ok": bool(rust_sync_engine_shadow.get("ok")),
                "verdict": sync_plan_result.get("verdict"),
                "risk_level": sync_plan_result.get("risk_level"),
                "authoritative": bool(rust_authority_gate.get("authoritative", False)),
                "authority_gate": rust_authority_gate.get("reason"),
                "manifest_status": (rust_apply_manifest.get("result") or {}).get("status") if isinstance(rust_apply_manifest.get("result"), dict) else None,
            },
        )
        apply_manifest_result = rust_apply_manifest.get("result", {}) if isinstance(rust_apply_manifest, dict) else {}
        if rust_apply_manifest.get("available") and not rust_apply_manifest.get("ok"):
            manifest_errors, manifest_warnings = diagnostics_to_messages(rust_apply_manifest)
            result.warnings.extend([f"Rust apply manifest: {msg}" for msg in manifest_errors + manifest_warnings])
        t_apply_transaction = time.perf_counter()
        rust_apply_transaction = rust_execute_apply_transaction(
            config,
            mode=mode,
            paths=paths,
            current_csv_text=current_csv_text,
            proposed_csv_text=proposed_csv_text,
            current_network_text=current_network_text,
            proposed_network_text=proposed_network_text,
            files_changed=result.files_changed,
            csv_changed=result.csv_changed,
            network_changed=result.network_changed,
            policy_decision=python_policy_dict,
            rust_sync_plan=rust_sync_plan,
            rust_authority_gate=rust_authority_gate,
            state=state_before,
            execute=False,
        )
        result.diff["rust_apply_transaction"] = rust_apply_transaction
        tx_result = rust_apply_transaction.get("result", {}) if isinstance(rust_apply_transaction, dict) else {}
        if tx_result.get("executed"):
            result.warnings.append("Rust apply transaction executed file writes. Python apply path should be disabled before enabling this in production.")
        timeline.record(
            "rust_apply_transaction",
            t_apply_transaction,
            status="ok" if rust_apply_transaction.get("ok") else ("unavailable" if not rust_apply_transaction.get("available") else "check"),
            details={
                "available": bool(rust_apply_transaction.get("available")),
                "ok": bool(rust_apply_transaction.get("ok")),
                "status": tx_result.get("status"),
                "executed": bool(tx_result.get("executed")),
                "write_count": tx_result.get("write_count"),
            },
        )
        t_transaction_journal = time.perf_counter()
        rust_transaction_journal = rust_build_transaction_journal(
            config,
            mode=mode,
            paths=paths,
            rust_apply_manifest=rust_apply_manifest,
            rust_apply_transaction=rust_apply_transaction,
            rust_sync_plan=rust_sync_plan,
            rust_authority_gate=rust_authority_gate,
            policy_decision=python_policy_dict,
        )
        result.diff["rust_transaction_journal"] = rust_transaction_journal
        journal_result = rust_transaction_journal.get("result", {}) if isinstance(rust_transaction_journal, dict) else {}
        timeline.record(
            "rust_transaction_journal",
            t_transaction_journal,
            status="ok" if rust_transaction_journal.get("ok") else ("unavailable" if not rust_transaction_journal.get("available") else "check"),
            details={
                "available": bool(rust_transaction_journal.get("available")),
                "ok": bool(rust_transaction_journal.get("ok")),
                "journal_id": journal_result.get("journal_id"),
                "append_required": bool(journal_result.get("append_required")),
                "rollback_available": bool(journal_result.get("rollback_available")),
            },
        )
        t_transaction_journal_append = time.perf_counter()
        rust_transaction_journal_append = rust_append_transaction_journal(
            config,
            mode=mode,
            paths=paths,
            rust_apply_manifest=rust_apply_manifest,
            rust_apply_transaction=rust_apply_transaction,
            rust_sync_plan=rust_sync_plan,
            rust_authority_gate=rust_authority_gate,
            policy_decision=python_policy_dict,
            rust_transaction_journal=rust_transaction_journal,
        )
        result.diff["rust_transaction_journal_append"] = rust_transaction_journal_append
        journal_append_result = rust_transaction_journal_append.get("result", {}) if isinstance(rust_transaction_journal_append, dict) else {}
        timeline.record(
            "rust_transaction_journal_append",
            t_transaction_journal_append,
            status="ok" if rust_transaction_journal_append.get("ok") else ("unavailable" if not rust_transaction_journal_append.get("available") else "check"),
            details={
                "available": bool(rust_transaction_journal_append.get("available")),
                "ok": bool(rust_transaction_journal_append.get("ok")),
                "status": journal_append_result.get("status"),
                "append_executed": bool(journal_append_result.get("append_executed")),
                "journal_id": journal_append_result.get("journal_id"),
            },
        )
        t_rollback_manifest = time.perf_counter()
        rust_rollback_manifest = rust_build_rollback_manifest(
            config,
            rust_apply_manifest=rust_apply_manifest,
            rust_apply_transaction=rust_apply_transaction,
            rust_transaction_journal=rust_transaction_journal,
        )
        result.diff["rust_rollback_manifest"] = rust_rollback_manifest
        rollback_result = rust_rollback_manifest.get("result", {}) if isinstance(rust_rollback_manifest, dict) else {}
        timeline.record(
            "rust_rollback_manifest",
            t_rollback_manifest,
            status="ok" if rust_rollback_manifest.get("ok") else ("unavailable" if not rust_rollback_manifest.get("available") else "check"),
            details={
                "available": bool(rust_rollback_manifest.get("available")),
                "ok": bool(rust_rollback_manifest.get("ok")),
                "rollback_id": rollback_result.get("rollback_id"),
                "status": rollback_result.get("status"),
                "operation_count": rollback_result.get("operation_count"),
            },
        )
        result.diff["policy_decision"] = python_policy_dict
        policy_state["last_policy_decision"] = python_policy_dict
        for w in policy_decision.warnings:
            msg = w.get("message") or w.get("title")
            if msg:
                result.warnings.append(f"Policy: {msg}")
        if policy_decision.blocked_reasons:
            for b in policy_decision.blocked_reasons:
                result.errors.append(f"Policy blocked: {b.get('message') or b.get('title')}")
        save_policy_state(config, policy_state)
        try:
            lifecycle_summary = update_lifecycle_state(
                config,
                policy_state,
                current_rows,
                ctx.existing_data,
                cleanup_candidates,
                policy_decision.to_dict(),
                cleanup_sources,
                active_counts_by_source,
                mode=mode,
            )
            result.diff["lifecycle_summary"] = lifecycle_summary
            result.diff["returned_clients"] = policy_state.get("returned_clients", [])
        except Exception as lifecycle_error:
            result.warnings.append(f"Smart lifecycle update failed: {lifecycle_error}")
            result.diff["lifecycle_summary"] = {"error": str(lifecycle_error)}
        save_policy_state(config, policy_state)
        try:
            result.diff["smart_insights"] = compute_smart_insights(
                config,
                result,
                policy_decision.to_dict(),
                state_before=state_before,
                preflight=preflight,
                policy_state=policy_state,
            )
        except Exception as insights_error:
            result.warnings.append(f"Smart insights failed: {insights_error}")
            result.diff["smart_insights"] = {"summary": "Smart insights failed", "error": str(insights_error), "recommendations": []}
        timeline.record("policy_evaluation", t, status="blocked" if not policy_decision.write_allowed else policy_decision.verdict, details={"verdict": policy_decision.verdict, "risk_level": policy_decision.risk_level, "risk_score": policy_decision.risk_score, "lifecycle": result.diff.get("lifecycle_summary", {})})

        if mode == "dry_run":
            result.finish("dry_run_complete")
            result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
            log_event(config, "info", f"Dry-run complete: csv_changed={result.csv_changed} network_changed={result.network_changed} policy={policy_decision.verdict}")
            write_audit(config, "dry_run_complete", details={"csv_changed": result.csv_changed, "network_changed": result.network_changed, "status": result.status, "policy_decision": policy_decision.to_dict(), "timings": result.timings})
            update_state(state_path, sync_running=False, scheduler_state="idle", last_dry_run=result.to_dict(), last_error=None)
            return result

        if not policy_decision.write_allowed:
            result.finish("policy_blocked")
            result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
            log_event(config, "error", f"Policy blocked sync: {policy_decision.verdict} risk={policy_decision.risk_level}")
            write_audit(config, "policy_blocked", details={"policy_decision": policy_decision.to_dict(), "timings": result.timings})
            update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="policy_blocked")
            return result

        if result.errors:
            result.finish("preflight_failed")
            result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
            log_event(config, "error", f"Preflight failed: {result.errors}")
            update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="preflight_failed")
            return result

        should_run_lq, apply_reason = _libreqos_should_apply(config, state_before, result, mode)
        should_run_lq, apply_reason, auto_apply_policy_decision = evaluate_auto_apply_policy(config, policy_decision, should_run_lq, apply_reason)
        result.diff["libreqos_apply_decision"] = apply_reason
        result.diff["auto_apply_policy_decision"] = auto_apply_policy_decision
        result.diff["policy_decision"] = policy_decision.to_dict()
        policy_state["last_policy_decision"] = policy_decision.to_dict()

        rc = config.get("rust_core", {}) or {}
        rust_file_write_authority = bool(rc.get("execute_apply_manifest") and rc.get("allow_rust_file_writes"))
        rust_apply_authority = bool(rc.get("allow_rust_libreqos_apply") and should_run_lq)
        rust_full_authority_required = bool(
            rc.get("full_rust_backend_authority")
            or rc.get("fail_closed_without_rust_authority")
            or str(rc.get("transaction_authority") or "") in {"rust_full_authoritative", "rust_apply_authoritative"}
        )
        python_mutation_fallback_allowed = False if rust_full_authority_required else bool(rc.get("python_mutation_fallback", False))
        files_were_written = False
        rust_libreqos_already_applied = False

        if rust_full_authority_required:
            result.diff["rust_full_authority_lock"] = {
                "enabled": True,
                "rust_file_write_authority": rust_file_write_authority,
                "rust_apply_authority": rust_apply_authority,
                "python_mutation_fallback_allowed": python_mutation_fallback_allowed,
                "transaction_authority": rc.get("transaction_authority"),
            }
            if result.files_changed and not rust_file_write_authority:
                result.errors.append("Rust full authority lock: file changes require execute_apply_manifest=true and allow_rust_file_writes=true.")
                result.finish("rust_full_authority_missing_file_write_flags")
                result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
                log_event(config, "error", "Rust full authority lock blocked Python file-write fallback")
                write_audit(config, "rust_full_authority_missing_file_write_flags", details={"rust_core": result.diff.get("rust_full_authority_lock"), "timings": result.timings})
                _rust_authority_mark_quarantine(config, "rust_full_authority_missing_file_write_flags", result, {"rust_core": result.diff.get("rust_full_authority_lock")})
                update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="rust_full_authority_missing_file_write_flags")
                return result
            if should_run_lq and not rust_apply_authority:
                result.errors.append("Rust full authority lock: LibreQoS apply requires allow_rust_libreqos_apply=true.")
                result.finish("rust_full_authority_missing_apply_flag")
                result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
                log_event(config, "error", "Rust full authority lock blocked Python LibreQoS apply fallback")
                write_audit(config, "rust_full_authority_missing_apply_flag", details={"rust_core": result.diff.get("rust_full_authority_lock"), "timings": result.timings})
                _rust_authority_mark_quarantine(config, "rust_full_authority_missing_apply_flag", result, {"rust_core": result.diff.get("rust_full_authority_lock")})
                update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="rust_full_authority_missing_apply_flag")
                return result

        if rust_full_authority_required and not _rust_authority_supervisor_preflight(config, result):
            result.finish("rust_authority_preflight_required_failed")
            result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
            log_event(config, "error", "Rust authority supervisor preflight failed; production mutation blocked")
            write_audit(config, "rust_authority_preflight_required_failed", details={"supervisor": result.diff.get("rust_authority_supervisor"), "timings": result.timings})
            _rust_authority_mark_quarantine(config, "rust_authority_preflight_required_failed", result, {"supervisor": result.diff.get("rust_authority_supervisor")})
            update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="rust_authority_preflight_required_failed")
            return result

        if rust_full_authority_required and not _rust_authority_watchdog(config, result):
            result.finish("rust_authority_watchdog_required_failed")
            result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
            log_event(config, "error", "Rust authority watchdog failed; production mutation blocked")
            write_audit(config, "rust_authority_watchdog_required_failed", details={"watchdog": result.diff.get("rust_authority_watchdog"), "timings": result.timings})
            _rust_authority_mark_quarantine(config, "rust_authority_watchdog_required_failed", result, {"watchdog": result.diff.get("rust_authority_watchdog")})
            update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="rust_authority_watchdog_required_failed")
            return result

        if rust_full_authority_required and not _rust_authority_live_stable_gate(config, state_before, result):
            result.finish("rust_live_stable_gate_failed")
            result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
            log_event(config, "error", "Rust live-stable gate failed; production mutation blocked")
            write_audit(config, "rust_live_stable_gate_failed", details={"gate": result.diff.get("rust_live_stable_gate"), "timings": result.timings})
            update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="rust_live_stable_gate_failed")
            return result

        if rust_full_authority_required and not _rust_set_and_forget_gate(config, result):
            result.finish("rust_set_and_forget_gate_failed")
            result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
            log_event(config, "error", "Rust set-and-forget readiness gate failed; production mutation blocked")
            write_audit(config, "rust_set_and_forget_gate_failed", details={"gate": result.diff.get("rust_set_and_forget_gate"), "timings": result.timings})
            _rust_authority_mark_quarantine(config, "rust_set_and_forget_gate_failed", result, {"gate": result.diff.get("rust_set_and_forget_gate")})
            update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="rust_set_and_forget_gate_failed")
            return result

        if result.files_changed:
            t = time.perf_counter()
            if not _drift_check(config, state_before, current_csv_text, current_network_text, result):
                timeline.record("drift_check", t, status="blocked")
                result.finish("file_drift_blocked")
                result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
                update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="file_drift_blocked")
                return result
            timeline.record("drift_check", t)

        if (result.files_changed and rust_file_write_authority) or rust_apply_authority:
            t = time.perf_counter()
            rust_authoritative_tx = rust_execute_apply_transaction(
                config,
                mode=mode,
                paths=paths,
                current_csv_text=current_csv_text,
                proposed_csv_text=proposed_csv_text,
                current_network_text=current_network_text,
                proposed_network_text=proposed_network_text,
                files_changed=result.files_changed,
                csv_changed=result.csv_changed,
                network_changed=result.network_changed,
                policy_decision=policy_decision.to_dict(),
                rust_sync_plan=rust_sync_plan,
                rust_authority_gate=rust_authority_gate,
                state=state_before,
                execute=True,
                allow_libreqos_apply=rust_apply_authority,
            )
            result.diff["rust_authoritative_apply_transaction"] = rust_authoritative_tx
            rust_tx_result = rust_authoritative_tx.get("result", {}) if isinstance(rust_authoritative_tx, dict) else {}
            rust_tx_errors, rust_tx_warnings = diagnostics_to_messages(rust_authoritative_tx)
            result.warnings.extend([f"Rust authoritative apply transaction: {msg}" for msg in rust_tx_warnings])
            timeline.record(
                "rust_authoritative_apply_transaction",
                t,
                status="ok" if rust_authoritative_tx.get("ok") else "failed",
                details={
                    "available": bool(rust_authoritative_tx.get("available")),
                    "ok": bool(rust_authoritative_tx.get("ok")),
                    "status": rust_tx_result.get("status"),
                    "authoritative": bool(rust_tx_result.get("authoritative")),
                    "file_writes_executed": bool(rust_tx_result.get("file_writes_executed")),
                    "libreqos_apply_executed": bool(rust_tx_result.get("libreqos_apply_executed")),
                    "write_count": rust_tx_result.get("write_count"),
                },
            )
            if (not rust_authoritative_tx.get("ok")) or rust_tx_errors or rust_tx_result.get("status") == "failed":
                result.errors.extend([f"Rust authoritative apply transaction: {msg}" for msg in rust_tx_errors] or ["Rust authoritative apply transaction failed"])
                result.finish("rust_authoritative_apply_failed")
                result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
                log_event(config, "error", f"Rust authoritative apply failed: {result.errors[-3:]}")
                write_audit(config, "rust_authoritative_apply_failed", details={"transaction": rust_authoritative_tx, "timings": result.timings})
                _rust_authority_mark_quarantine(config, "rust_authoritative_apply_failed", result, {"transaction": rust_authoritative_tx})
                update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="rust_authoritative_apply_failed")
                return result

            if rc.get("append_transaction_journal") and rc.get("allow_transaction_journal_writes"):
                t_journal = time.perf_counter()
                rust_authoritative_journal = rust_build_transaction_journal(
                    config,
                    mode=mode,
                    paths=paths,
                    rust_apply_manifest=rust_apply_manifest,
                    rust_apply_transaction=rust_authoritative_tx,
                    rust_sync_plan=rust_sync_plan,
                    rust_authority_gate=rust_authority_gate,
                    policy_decision=policy_decision.to_dict(),
                )
                rust_authoritative_journal_append = rust_append_transaction_journal(
                    config,
                    mode=mode,
                    paths=paths,
                    rust_apply_manifest=rust_apply_manifest,
                    rust_apply_transaction=rust_authoritative_tx,
                    rust_sync_plan=rust_sync_plan,
                    rust_authority_gate=rust_authority_gate,
                    policy_decision=policy_decision.to_dict(),
                    rust_transaction_journal=rust_authoritative_journal,
                )
                result.diff["rust_authoritative_transaction_journal"] = rust_authoritative_journal
                result.diff["rust_authoritative_transaction_journal_append"] = rust_authoritative_journal_append
                timeline.record(
                    "rust_authoritative_transaction_journal_append",
                    t_journal,
                    status="ok" if rust_authoritative_journal_append.get("ok") else "failed",
                    details={
                        "ok": bool(rust_authoritative_journal_append.get("ok")),
                        "append_executed": bool((rust_authoritative_journal_append.get("result") or {}).get("append_executed")),
                        "journal_id": (rust_authoritative_journal_append.get("result") or {}).get("journal_id"),
                    },
                )
                journal_errors, journal_warnings = diagnostics_to_messages(rust_authoritative_journal_append)
                result.warnings.extend([f"Rust authoritative journal: {msg}" for msg in journal_warnings])
                if (not rust_authoritative_journal_append.get("ok")) or journal_errors:
                    result.errors.extend([f"Rust authoritative journal: {msg}" for msg in journal_errors] or ["Rust authoritative journal append failed"])
                    result.finish("rust_authoritative_journal_failed")
                    result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
                    log_event(config, "error", f"Rust authoritative journal failed: {result.errors[-3:]}")
                    _rust_authority_mark_quarantine(config, "rust_authoritative_journal_failed", result, {"journal_append": rust_authoritative_journal_append})
                    update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="rust_authoritative_journal_failed")
                    return result

            if rust_tx_result.get("file_writes_executed"):
                files_were_written = True
                result.diff["files_written_by"] = "rust"
                result.diff["rust_write_results"] = rust_tx_result.get("write_results", [])
                write_audit(config, "files_written", details={"executor": "rust", "csv_changed": result.csv_changed, "network_changed": result.network_changed, "client_change_summary": result.diff.get("client_change_summary"), "client_changes": result.diff.get("client_changes", []), "write_results": rust_tx_result.get("write_results", []), "timings": result.timings})
                if result.diff.get("client_change_summary", {}).get("counts", {}).get("total", 0):
                    write_audit(config, "client_changes", details={"summary": result.diff.get("client_change_summary"), "changes": result.diff.get("client_changes", []), "timings": result.timings})
                update_state(state_path, pending_libreqos_apply=True, last_file_write_success=True)

            if rust_tx_result.get("libreqos_apply_executed"):
                lq = rust_tx_result.get("libreqos_apply_result") or {}
                rust_libreqos_already_applied = True
                result.libreqos_triggered = True
                result.libreqos_exit_code = lq.get("exit_code")
                result.libreqos_stdout = lq.get("stdout", "")
                result.libreqos_stderr = lq.get("stderr", "")
                result.diff["libreqos_command"] = lq.get("command")
                result.diff["libreqos_run_id"] = lq.get("run_id")
                result.diff["libreqos_duration_ms"] = lq.get("duration_ms")
                result.diff["libreqos_apply_reason"] = apply_reason
                result.diff["libreqos_working_dir"] = lq.get("working_dir")
                result.diff["libreqos_executor"] = "rust"
                _mark_libreqos_state(state_path, result, bool(lq.get("ok")), apply_reason, lq.get("run_id"))
                write_audit(config, "libreqos_apply", details={"executor": "rust", "ok": lq.get("ok"), "exit_code": lq.get("exit_code"), "run_id": lq.get("run_id"), "reason": apply_reason})
                if not lq.get("ok"):
                    result.errors.append("Rust LibreQoS update failed")
                    result.finish("libreqos_failed")
                    result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
                    log_event(config, "error", f"Rust LibreQoS failed: reason={apply_reason} exit={result.libreqos_exit_code} stderr={result.libreqos_stderr[:500]}")
                    _rust_authority_mark_quarantine(config, "libreqos_failed", result, {"executor": "rust", "exit_code": result.libreqos_exit_code})
                    update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="libreqos_failed")
                    return result

        elif result.files_changed:
            if rust_full_authority_required and not python_mutation_fallback_allowed:
                result.errors.append("Rust full authority lock: Rust transaction did not execute file writes; Python fallback is disabled.")
                result.finish("rust_full_authority_file_write_not_executed")
                result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
                log_event(config, "error", "Rust full authority lock refused Python file-write fallback after Rust transaction path")
                write_audit(config, "rust_full_authority_file_write_not_executed", details={"rust_core": result.diff.get("rust_full_authority_lock"), "timings": result.timings})
                _rust_authority_mark_quarantine(config, "rust_full_authority_file_write_not_executed", result, {"rust_core": result.diff.get("rust_full_authority_lock")})
                update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="rust_full_authority_file_write_not_executed")
                return result
            if config.get("app", {}).get("backup_before_apply", True):
                t = time.perf_counter()
                result.diff["backup_path"] = create_backup(config, reason=mode)
                timeline.record("backup", t, details={"path": result.diff.get("backup_path")})
                write_audit(config, "backup_created", details={"path": result.diff.get("backup_path"), "reason": mode})

            t = time.perf_counter()
            atomic_write_text(csv_path, proposed_csv_text)
            timeline.record("csv_write", t, details={"path": csv_path, "changed": result.csv_changed})

            t = time.perf_counter()
            atomic_write_text(network_path, proposed_network_text)
            timeline.record("network_write", t, details={"path": network_path, "changed": result.network_changed})
            write_audit(config, "files_written", details={"executor": "python", "csv_changed": result.csv_changed, "network_changed": result.network_changed, "client_change_summary": result.diff.get("client_change_summary"), "client_changes": result.diff.get("client_changes", []), "timings": result.timings})
            if result.diff.get("client_change_summary", {}).get("counts", {}).get("total", 0):
                write_audit(config, "client_changes", details={"summary": result.diff.get("client_change_summary"), "changes": result.diff.get("client_changes", []), "timings": result.timings})
            files_were_written = True
            update_state(state_path, pending_libreqos_apply=True, last_file_write_success=True)

        if should_run_lq and not rust_libreqos_already_applied:
            if rust_full_authority_required and not python_mutation_fallback_allowed:
                result.errors.append("Rust full authority lock: Rust did not execute LibreQoS apply; Python apply fallback is disabled.")
                result.finish("rust_full_authority_libreqos_apply_not_executed")
                result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
                log_event(config, "error", "Rust full authority lock refused Python LibreQoS apply fallback")
                write_audit(config, "rust_full_authority_libreqos_apply_not_executed", details={"rust_core": result.diff.get("rust_full_authority_lock"), "timings": result.timings})
                _rust_authority_mark_quarantine(config, "rust_full_authority_libreqos_apply_not_executed", result, {"rust_core": result.diff.get("rust_full_authority_lock")})
                update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="rust_full_authority_libreqos_apply_not_executed")
                return result
            lq = _run_libreqos_apply(config, state_path, result, timeline, apply_reason)
            write_audit(config, "libreqos_apply", details={"executor": "python", "ok": lq.get("ok"), "exit_code": lq.get("exit_code"), "run_id": lq.get("run_id"), "reason": apply_reason})
            if not lq["ok"]:
                result.errors.append("LibreQoS update failed")
                result.finish("libreqos_failed")
                result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
                log_event(config, "error", f"LibreQoS failed: reason={apply_reason} exit={result.libreqos_exit_code} stderr={result.libreqos_stderr[:500]}")
                update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error="libreqos_failed")
                return result
        elif files_were_written and not rust_libreqos_already_applied:
            update_state(state_path, pending_libreqos_apply=True, last_libreqos_apply_reason=apply_reason)

        result.finish("success")
        result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
        try:
            update_successful_source_counts(policy_state, cleanup_sources, active_counts_by_source, {k: int(v.get("active_count", 0) or 0) for k, v in (ctx.node_math or {}).items() if isinstance(v, dict)})
            save_policy_state(config, policy_state)
        except Exception as policy_state_error:
            result.warnings.append(f"Policy state save failed: {policy_state_error}")
        log_event(config, "info", f"Sync success: files_changed={result.files_changed} libreqos_triggered={result.libreqos_triggered} duration_ms={result.timings.get('cycle_total')}")
        write_audit(config, "sync_finished", details={"status": result.status, "files_changed": result.files_changed, "libreqos_triggered": result.libreqos_triggered, "libreqos_exit_code": result.libreqos_exit_code, "client_change_summary": result.diff.get("client_change_summary"), "policy_decision": result.diff.get("policy_decision"), "lifecycle_summary": result.diff.get("lifecycle_summary"), "timings": result.timings})
        _rust_authority_record_last_good_snapshot(config, result)
        update_state(
            state_path,
            sync_running=False,
            scheduler_state="idle",
            last_run=result.to_dict(),
            last_error=None,
            last_file_hashes={"csv": result.file_hashes["proposed_csv"], "network": result.file_hashes["proposed_network"]},
        )
        return result
    except Exception as e:
        result.errors.append(str(e))
        result.finish("failed")
        result.timings["cycle_total"] = round((time.perf_counter() - cycle_start) * 1000, 3)
        try:
            log_event(config if "config" in locals() else {}, "error", f"Sync failed: {e}")
            write_audit(config if "config" in locals() else {}, "sync_failed", details={"error": str(e), "mode": mode, "timings": result.timings})
        except Exception:
            pass
        try:
            update_state(state_path, sync_running=False, scheduler_state="error", last_run=result.to_dict(), last_error=str(e))
        except Exception:
            pass
        return result
    finally:
        if not result.duration_seconds:
            result.duration_seconds = round((time.perf_counter() - cycle_start), 3)


def run_cycle(mode="apply", config_path=None):
    """Run one sync cycle with an inter-process lock."""
    cfg = load_config(config_path)
    state_path = cfg["paths"].get("runtime_state", "state/runtime_state.json")
    lock_path = cfg["paths"].get("lock_file") or str(Path(state_path).with_name("lqosync.lock"))
    try:
        with InterProcessLock(lock_path):
            return _run_cycle_unlocked(mode=mode, config_path=config_path)
    except LockBusy as e:
        result = SyncResult(mode=mode)
        result.warnings.append(str(e))
        result.finish("already_running")
        try:
            update_state(state_path, sync_running=True, scheduler_state="running", last_error=str(e))
        except Exception:
            pass
        return result
