"""Policy and path completeness audit for LQoSync.

Read-only helper used by release/regression checks to ensure policy schema
paths, policy defaults, config.json.example, and required runtime paths stay in
sync as the project grows.
"""
from __future__ import annotations

import json
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

from engine.config_schema import migrate_config_schema, validate_schema
from engine.policy_defaults import smart_policy_defaults
from engine.policy_schema import POLICY_SCHEMA, get_by_path


@dataclass
class AuditItem:
    key: str
    title: str
    status: str
    detail: str
    fix: str = ""

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


def _summary(items: list[AuditItem]) -> dict[str, int]:
    return {
        "ok": sum(1 for i in items if i.status == "ok"),
        "warn": sum(1 for i in items if i.status == "warn"),
        "fail": sum(1 for i in items if i.status == "fail"),
    }


REQUIRED_RUNTIME_PATHS = [
    "paths.shaped_devices_csv",
    "paths.network_json",
    "paths.backup_dir",
    "paths.runtime_state",
    "paths.policy_state",
    "paths.audit_log",
    "paths.collector_cache",
    "paths.libreqos_apply_log_dir",
    "paths.lock_file",
    "paths.log_file",
    "libreqos.cmd",
    "libreqos.working_dir",
]


REQUIRED_POLICY_PATHS = [item["path"] for item in POLICY_SCHEMA]


def _load_config_example(root: Path) -> dict[str, Any]:
    return json.loads((root / "config.json.example").read_text(encoding="utf-8"))


def audit_policy_and_paths(root: str | Path | None = None) -> dict[str, Any]:
    root = Path(root or Path(__file__).resolve().parents[1])
    items: list[AuditItem] = []

    try:
        raw = _load_config_example(root)
        cfg, notes = migrate_config_schema(raw)
        schema = validate_schema(cfg)
    except Exception as exc:
        item = AuditItem("config.example", "Load/migrate config.json.example", "fail", str(exc), "Fix config.json.example and schema migration.")
        return {"verdict": "fail", "items": [item.to_dict()], "summary": _summary([item])}

    # 1. Required runtime paths exist in config example after migration.
    missing_runtime = [path for path in REQUIRED_RUNTIME_PATHS if get_by_path(cfg, path, None) in (None, "")]
    if missing_runtime:
        items.append(AuditItem("paths.required", "Required runtime paths", "fail", ", ".join(missing_runtime), "Add these paths to config.json.example and config schema defaults."))
    else:
        items.append(AuditItem("paths.required", "Required runtime paths", "ok", f"{len(REQUIRED_RUNTIME_PATHS)} required runtime paths are present."))

    # 2. Policy schema paths are present in config example after migration.
    missing_config_policy = [path for path in REQUIRED_POLICY_PATHS if get_by_path(cfg, path, None) is None]
    if missing_config_policy:
        items.append(AuditItem("policy.config_paths", "Config policy paths", "fail", "; ".join(missing_config_policy[:20]), "Add missing policies to policy defaults and schema migration."))
    else:
        items.append(AuditItem("policy.config_paths", "Config policy paths", "ok", f"{len(REQUIRED_POLICY_PATHS)} policy schema paths are present after config migration."))

    # 3. Policy defaults also include every schema path.
    defaults_wrapped = {"policies": smart_policy_defaults()}
    missing_default_policy = [path for path in REQUIRED_POLICY_PATHS if get_by_path(defaults_wrapped, path, None) is None]
    if missing_default_policy:
        items.append(AuditItem("policy.default_paths", "Policy defaults cover schema", "fail", "; ".join(missing_default_policy[:20]), "Update engine/policy_defaults.py so POLICY_SCHEMA and defaults match."))
    else:
        items.append(AuditItem("policy.default_paths", "Policy defaults cover schema", "ok", f"{len(REQUIRED_POLICY_PATHS)} policy schema paths are covered by smart_policy_defaults()."))

    # 4. Schema validation should not produce missing-policy warnings.
    warnings = schema.get("warnings") or []
    missing_policy_warnings = [w for w in warnings if str(w).startswith("Missing policy setting:")]
    if missing_policy_warnings:
        items.append(AuditItem("policy.schema_warnings", "Missing policy warnings", "fail", "; ".join(missing_policy_warnings[:20]), "Fix policy defaults/schema migration before publishing."))
    else:
        items.append(AuditItem("policy.schema_warnings", "Missing policy warnings", "ok", "No missing-policy schema warnings after migration."))

    # 5. Report schema errors separately.
    errors = schema.get("errors") or []
    if errors:
        items.append(AuditItem("config.schema_errors", "Schema errors", "fail", "; ".join(errors[:20]), "Fix config schema errors before publishing."))
    else:
        items.append(AuditItem("config.schema_errors", "Schema errors", "ok", "No schema errors after migration."))

    summary = _summary(items)
    verdict = "pass" if summary["fail"] == 0 else "fail"
    if verdict == "pass" and summary["warn"]:
        verdict = "pass_with_warnings"
    return {"verdict": verdict, "items": [i.to_dict() for i in items], "summary": summary, "migration_notes": notes, "schema": schema}
