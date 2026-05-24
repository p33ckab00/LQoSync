"""Dashboard module wiring audit helpers.

This module is read-only. It gives operators and release checks a single source
of truth for which Dashboard cards are backed by which backend source.

These helpers do not run collectors, write files, or mutate LibreQoS state.
"""
from __future__ import annotations

from pathlib import Path
from typing import Any


def _last_run(state: dict[str, Any] | None) -> dict[str, Any]:
    if not isinstance(state, dict):
        return {}
    return state.get("last_run") or state.get("last_dry_run") or {}


def _diff(state: dict[str, Any] | None) -> dict[str, Any]:
    last = _last_run(state)
    d = last.get("diff") if isinstance(last, dict) else {}
    return d if isinstance(d, dict) else {}


def _service_active(services: dict[str, Any] | None, name: str) -> str:
    if not isinstance(services, dict):
        return "unknown"
    info = services.get(name) or {}
    if isinstance(info, dict):
        return str(info.get("active") or "unknown")
    return "unknown"


def _path_exists(path: str | None) -> bool:
    if not path:
        return False
    try:
        return Path(path).exists()
    except Exception:
        return False


def _module(module_id: str, label: str, backend: str, status: str, detail: str, target: str = "#") -> dict[str, Any]:
    return {
        "id": module_id,
        "label": label,
        "backend": backend,
        "status": status,
        "detail": detail,
        "target": target,
    }


def build_dashboard_module_wiring(
    cfg: dict[str, Any] | None,
    state: dict[str, Any] | None,
    *,
    services: dict[str, Any] | None = None,
    git_status: dict[str, Any] | None = None,
    config_errors: list[Any] | None = None,
    config_warnings: list[Any] | None = None,
    health_report: dict[str, Any] | None = None,
    production_readiness: dict[str, Any] | None = None,
    setup_wizard: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Return Dashboard module-to-backend wiring status.

    This is intentionally defensive because it runs on the Dashboard. Missing
    state should report idle/warn instead of raising template-breaking errors.
    """
    cfg = cfg if isinstance(cfg, dict) else {}
    state = state if isinstance(state, dict) else {}
    services = services if isinstance(services, dict) else {}
    git_status = git_status if isinstance(git_status, dict) else {}
    config_errors = config_errors or []
    config_warnings = config_warnings or []
    health_report = health_report if isinstance(health_report, dict) else {}
    production_readiness = production_readiness if isinstance(production_readiness, dict) else {}
    setup_wizard = setup_wizard if isinstance(setup_wizard, dict) else {}

    last = _last_run(state)
    diff = _diff(state)
    rc = cfg.get("rust_core") or {}
    scheduler_cfg = cfg.get("scheduler") or {}
    paths = cfg.get("paths") or {}

    lqosync_core_state = _service_active(services, "lqosync-core")
    lqosync_web_state = _service_active(services, "lqosync")

    modules: list[dict[str, Any]] = []

    modules.append(_module(
        "operator_health_summary",
        "Operator health summary",
        "get_status + validate_config + runtime_state.json",
        "fail" if config_errors else ("warn" if config_warnings else "ok"),
        f"config_errors={len(config_errors)} config_warnings={len(config_warnings)}",
        "#health-summary",
    ))

    modules.append(_module(
        "production_readiness",
        "Production Readiness",
        "engine.production_readiness.compute_production_readiness + /api/production/readiness",
        "ok" if production_readiness.get("checks") else "warn",
        f"score={production_readiness.get('score', 'n/a')} level={production_readiness.get('level', 'unknown')}",
        "#production-readiness",
    ))

    modules.append(_module(
        "source_health_performance",
        "Source Health & Performance",
        "engine.health_trends.compute_health_report + /api/health/trends",
        "ok" if {"source_health", "performance_trends", "libreqos_apply_health"}.issubset(set(health_report.keys())) else "warn",
        f"sources={len(health_report.get('source_health') or [])} health={health_report.get('health_score', 'n/a')}",
        "#source-health-performance",
    ))

    scheduler_ok = scheduler_cfg.get("engine") == "rust" and scheduler_cfg.get("allow_python_scheduler") is False
    modules.append(_module(
        "rust_scheduler_authority",
        "Rust Scheduler Authority",
        "scheduler.runner facade -> engine.rust_scheduler -> lqosync-core Unix socket",
        "ok" if scheduler_ok and lqosync_core_state == "active" else "warn",
        f"engine={scheduler_cfg.get('engine', 'unknown')} allow_python_scheduler={scheduler_cfg.get('allow_python_scheduler')} lqosync-core={lqosync_core_state}",
        "/api/status",
    ))

    rust_authority_ok = bool(rc.get("enabled")) and bool(rc.get("full_rust_backend_authority")) and bool(rc.get("python_mutation_fallback") is False)
    modules.append(_module(
        "rust_backend_authority",
        "Rust Backend Authority",
        "engine.rust_core wrappers -> lqosync-core daemon",
        "ok" if rust_authority_ok and lqosync_core_state == "active" else "warn",
        f"enabled={rc.get('enabled')} full_authority={rc.get('full_rust_backend_authority')} python_fallback={rc.get('python_mutation_fallback')} service={lqosync_core_state}",
        "/api/status",
    ))

    modules.append(_module(
        "legacy_python_backend_service",
        "Legacy Python backend service",
        "retired gunicorn/flask lqosync service",
        "warn" if lqosync_web_state == "active" else "ok",
        f"lqosync={lqosync_web_state}; Python role={rc.get('python_runtime_role', 'retired')}",
        "/",
    ))

    modules.append(_module(
        "change_summary",
        "What changed last sync / client feed",
        "state.last_run.diff.client_change_summary + client_changes",
        "ok" if diff.get("client_change_summary") or diff.get("client_changes") else "idle",
        f"client_changes={len(diff.get('client_changes') or [])}",
        "#policy-decision",
    ))

    modules.append(_module(
        "policy_decision",
        "Policy Decision",
        "engine.policy_engine result stored in state.last_run.diff.policy_decision",
        "ok" if diff.get("policy_decision") else "idle",
        f"verdict={(diff.get('policy_decision') or {}).get('verdict', 'n/a')}",
        "#policy-decision",
    ))

    modules.append(_module(
        "smart_insights",
        "Smart Insights",
        "engine.insights output stored in state.last_run.diff.smart_insights",
        "ok" if diff.get("smart_insights") else "idle",
        f"recommendations={len(((diff.get('smart_insights') or {}).get('recommendations') or []))}",
        "#smart-insights",
    ))

    modules.append(_module(
        "lifecycle",
        "Smart Lifecycle",
        "engine.lifecycle output stored in state.last_run.diff.lifecycle_summary",
        "ok" if diff.get("lifecycle_summary") else "idle",
        f"tracked={(diff.get('lifecycle_summary') or {}).get('total_tracked_clients', 'n/a')}",
        "#smart-lifecycle",
    ))

    modules.append(_module(
        "generated_files",
        "Generated Files and Drift Policy",
        "paths.shaped_devices_csv + paths.network_json + last_run write/apply metadata",
        "ok" if _path_exists(paths.get("shaped_devices_csv")) and _path_exists(paths.get("network_json")) else "warn",
        f"csv_exists={_path_exists(paths.get('shaped_devices_csv'))} network_exists={_path_exists(paths.get('network_json'))}",
        "/operations?tab=apply",
    ))

    modules.append(_module(
        "services_snapshot",
        "Services Snapshot",
        "monitoring.service_monitor.all_service_status + /api/services/status",
        "ok" if services else "warn",
        f"services={len(services)} lqosync-core={lqosync_core_state} lqosync={lqosync_web_state}",
        "/operations?tab=services",
    ))

    modules.append(_module(
        "git_status",
        "Version / Git Status",
        "_git_status helper",
        "ok" if git_status.get("branch") else "warn",
        f"branch={git_status.get('branch', 'unknown')} relation={git_status.get('relation', 'unknown')}",
        "/updates",
    ))

    modules.append(_module(
        "setup_wizard_banner",
        "Setup Wizard banner",
        "engine.setup_wizard.compute_setup_wizard",
        "ok" if setup_wizard else "warn",
        f"progress={setup_wizard.get('progress', 'n/a')} production_ready={setup_wizard.get('production_ready', 'n/a')}",
        "/setup-wizard",
    ))

    summary = {
        "ok": sum(1 for m in modules if m["status"] == "ok"),
        "warn": sum(1 for m in modules if m["status"] == "warn"),
        "idle": sum(1 for m in modules if m["status"] == "idle"),
        "fail": sum(1 for m in modules if m["status"] == "fail"),
        "total": len(modules),
    }
    overall = "fail" if summary["fail"] else ("warn" if summary["warn"] else "ok")
    return {
        "schema": "lqosync.dashboard_module_wiring.v1",
        "overall": overall,
        "summary": summary,
        "modules": modules,
        "notes": [
            "Dashboard wiring audit is read-only and does not run collectors or mutate LibreQoS files.",
            "Idle means the module is wired but waiting for a first dry-run/sync cycle to populate state.",
        ],
    }
