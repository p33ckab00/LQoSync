"""Policy conflict and client identity helpers for LQoSync.

This module is intentionally read-only. It reviews operator-configured policies
and returns explanations that can be shown in Policy Center, Config Center,
Dry Run, and reports. It does not change config.json or runtime state.
"""
from __future__ import annotations

from typing import Any

from engine.policy_schema import get_by_path, policy_diff_from_preset, POLICY_SCHEMA

DANGEROUS_CLEANUP = {"cleanup_immediate", "cleanup_next_run"}
PERMISSIVE_CLEANUP = {"cleanup_immediate", "cleanup_next_run", "warn_only"}
PROTECTED_ZERO_ACTIONS = {"block_cleanup", "block_apply", "require_confirm_next_run", "require_confirm_immediate", "preserve_rows"}
SOURCE_LABELS = {"pppoe": "PPPoE", "dhcp": "DHCP", "hotspot": "Hotspot", "static": "Static/manual"}


def _conflict(severity: str, title: str, what: str, why: str, recommendation: str, paths: list[str] | None = None) -> dict[str, Any]:
    severity_rank = {"critical": 4, "high": 3, "medium": 2, "low": 1}.get(severity, 1)
    return {
        "severity": severity,
        "severity_rank": severity_rank,
        "title": title,
        "what": what,
        "why": why,
        "recommendation": recommendation,
        "paths": paths or [],
    }


def evaluate_policy_conflicts(cfg: dict) -> dict[str, Any]:
    """Return policy conflicts and risky combinations from config.json."""
    conflicts: list[dict[str, Any]] = []
    policies = cfg.get("policies") or {}
    app = cfg.get("app") or {}

    for source, label in SOURCE_LABELS.items():
        base = f"policies.cleanup_sources.{source}"
        normal = get_by_path(cfg, f"{base}.normal_inactive_action")
        zero = get_by_path(cfg, f"{base}.zero_result_action")
        failed = get_by_path(cfg, f"{base}.collector_failed_action")
        disabled = get_by_path(cfg, f"{base}.source_disabled_action")
        mass = get_by_path(cfg, f"{base}.mass_removal_action")
        respect = get_by_path(cfg, f"{base}.respect_percentage_guards")

        if normal == "cleanup_immediate" and zero in {"cleanup_immediate", "cleanup_next_run"}:
            conflicts.append(_conflict(
                "critical",
                f"{label}: zero-result cleanup is too aggressive",
                f"{label} normal inactive cleanup is immediate and zero-result cleanup is also {zero}.",
                "If a source returns zero rows due to API/VLAN/query trouble, LQoSync may remove the entire source too quickly.",
                "Use block_cleanup or require_confirm_next_run for zero_result_action, especially in production.",
                [f"{base}.normal_inactive_action", f"{base}.zero_result_action"],
            ))
        elif normal == "cleanup_immediate" and zero == "warn_only" and source != "hotspot":
            conflicts.append(_conflict(
                "high",
                f"{label}: zero-result only warns while normal cleanup is immediate",
                f"{label} removes normal inactive rows immediately, but zero-result only warns.",
                "This can be acceptable for very dynamic/session-only sources, but it weakens protection when a source suddenly returns zero.",
                "Use block_cleanup for subscriber sources, or document why warn_only is intentional.",
                [f"{base}.normal_inactive_action", f"{base}.zero_result_action"],
            ))

        if failed in DANGEROUS_CLEANUP:
            conflicts.append(_conflict(
                "critical",
                f"{label}: collector failure can delete rows",
                f"collector_failed_action is {failed}.",
                "A collector/API failure is not proof that subscribers are gone. Deleting on failure can remove valid rows from ShapedDevices.csv.",
                "Set collector_failed_action to preserve_rows.",
                [f"{base}.collector_failed_action"],
            ))

        if disabled == "cleanup_immediate":
            conflicts.append(_conflict(
                "high",
                f"{label}: source-disabled cleanup is immediate",
                "Disabling this source in config can remove existing rows in the same cycle.",
                "Source-disabled cleanup may be intentional, but it is often high impact.",
                "Use require_confirm_next_run for production so the operator confirms source-wide cleanup.",
                [f"{base}.source_disabled_action"],
            ))

        if respect is False and mass in PERMISSIVE_CLEANUP:
            conflicts.append(_conflict(
                "medium",
                f"{label}: percentage guards bypassed with permissive mass action",
                f"respect_percentage_guards is disabled and mass_removal_action is {mass}.",
                "The source can behave very aggressively during large removals.",
                "This is acceptable for guest/session-like sources, but subscriber sources should respect guards or require confirmation.",
                [f"{base}.respect_percentage_guards", f"{base}.mass_removal_action"],
            ))

    backup_before = app.get("backup_before_apply", cfg.get("backup_before_apply", False))
    require_backup = get_by_path(cfg, "policies.backup_guard.require_backup_before_apply")
    low = get_by_path(cfg, "policies.auto_apply_policy.allow_low_risk")
    med = get_by_path(cfg, "policies.auto_apply_policy.allow_medium_risk")
    high = get_by_path(cfg, "policies.auto_apply_policy.allow_high_risk")
    crit = get_by_path(cfg, "policies.auto_apply_policy.allow_critical_risk")
    if require_backup and (low or med or high or crit) and not backup_before:
        conflicts.append(_conflict(
            "high",
            "Auto-apply can run while backup_before_apply is disabled",
            "Backup Guard requires backups before apply, but app.backup_before_apply is disabled.",
            "Automatic LibreQoS applies should normally create a backup first.",
            "Enable app.backup_before_apply or disable risk-level auto-apply.",
            ["app.backup_before_apply", "policies.backup_guard.require_backup_before_apply", "policies.auto_apply_policy"],
        ))

    if high:
        conflicts.append(_conflict(
            "high",
            "High-risk auto-apply is enabled",
            "policies.auto_apply_policy.allow_high_risk is true.",
            "High-risk changes should normally require operator review because they may involve mass cleanup, collector issues, or validation warnings.",
            "Keep high-risk auto-apply disabled in production.",
            ["policies.auto_apply_policy.allow_high_risk"],
        ))
    if crit:
        conflicts.append(_conflict(
            "critical",
            "Critical-risk auto-apply is enabled",
            "policies.auto_apply_policy.allow_critical_risk is true.",
            "Critical-risk events can include unsafe cleanup, invalid data, or serious validation failures.",
            "Disable critical-risk auto-apply.",
            ["policies.auto_apply_policy.allow_critical_risk"],
        ))
    if med:
        conflicts.append(_conflict(
            "medium",
            "Medium-risk auto-apply is enabled",
            "policies.auto_apply_policy.allow_medium_risk is true.",
            "Medium-risk changes can still be safe, but they deserve visibility in production.",
            "Use this only after testing policies with Dry Run and backups enabled.",
            ["policies.auto_apply_policy.allow_medium_risk"],
        ))

    for path in [
        "policies.apply_guard.block_apply_on_missing_parent",
        "policies.apply_guard.block_apply_on_duplicate_ip",
        "policies.apply_guard.block_apply_on_invalid_speed",
        "policies.apply_guard.block_apply_on_collector_failure",
    ]:
        if get_by_path(cfg, path) is False:
            conflicts.append(_conflict(
                "high",
                "Apply guard is disabled",
                f"{path} is false.",
                "This weakens protection before writing/applying generated files.",
                "Keep apply guards enabled unless you are intentionally testing in a lab.",
                [path],
            ))

    identity_report = client_identity_report(cfg)
    for item in identity_report["sources"]:
        if item["grace_enabled"] and item["stability"] in {"unstable", "mixed"}:
            conflicts.append(_conflict(
                "medium",
                f"{item['label']}: grace enabled on non-stable identity",
                f"Grace is enabled while identity is {item['identity']} ({item['stability']}).",
                "Grace works best when the same subscriber returns with the same identity. Random MAC/IP behavior can create ghost rows.",
                "Disable grace for unstable DHCP/Hotspot MAC-only environments, or use stable usernames/vouchers.",
                [f"policies.stale_lifecycle.sources.{item['source']}.grace_enabled", f"policies.stale_lifecycle.sources.{item['source']}.identity"],
            ))

    conflicts.sort(key=lambda x: x["severity_rank"], reverse=True)
    counts = {"critical": 0, "high": 0, "medium": 0, "low": 0}
    for c in conflicts:
        counts[c["severity"]] = counts.get(c["severity"], 0) + 1
    verdict = "ok"
    if counts["critical"]:
        verdict = "critical_conflicts"
    elif counts["high"]:
        verdict = "high_conflicts"
    elif counts["medium"]:
        verdict = "warnings"
    return {"verdict": verdict, "counts": counts, "conflicts": conflicts, "total": len(conflicts)}


def enhanced_preset_comparison(cfg: dict, preset: str = "singularity", limit: int = 200) -> dict[str, Any]:
    """Return richer current-vs-preset rows for UI tables."""
    diff = policy_diff_from_preset(cfg, preset, limit=limit)
    schema_by_path = {item["path"]: item for item in POLICY_SCHEMA}
    rows = []
    for d in diff:
        meta = schema_by_path.get(d["path"], {})
        rows.append({
            **d,
            "section": meta.get("section", "Policy"),
            "risk": meta.get("risk", d.get("risk", "medium")),
            "recommendation": meta.get("setup_guidance") or meta.get("description") or "Review this difference before saving.",
        })
    by_section: dict[str, int] = {}
    for row in rows:
        by_section[row["section"]] = by_section.get(row["section"], 0) + 1
    return {"preset": preset, "rows": rows, "count": len(rows), "by_section": by_section}


def client_identity_report(cfg: dict) -> dict[str, Any]:
    """Explain source identity behavior for lifecycle/cleanup decisions."""
    items = []
    default_info = {
        "pppoe": ("PPPoE", "username", "stable", "PPPoE usernames are usually stable even when IP changes. Optional short grace is safest here."),
        "dhcp": ("DHCP", "server_mac", "mixed", "DHCP identity is usually DHCP server + MAC. Private/random MAC can appear as a new client."),
        "hotspot": ("Hotspot", "username_or_mac", "mixed", "Hotspot username/voucher is stable; MAC-only Hotspot can be unstable with randomized MAC."),
        "static": ("Static/manual", "manual", "stable", "Manual rows are normally operator-controlled and should usually be preserved by policy."),
    }
    sources = get_by_path(cfg, "policies.stale_lifecycle.sources", {}) or {}
    for source, (label, fallback_identity, fallback_stability, guidance) in default_info.items():
        data = sources.get(source, {}) if isinstance(sources, dict) else {}
        identity = data.get("identity", fallback_identity)
        if source == "pppoe" and identity == "username":
            stability = "stable"
        elif source == "static":
            stability = "stable"
        elif source == "dhcp" and identity in {"server_mac", "mac", "ip", "server_ip"}:
            stability = "mixed"
        elif source == "hotspot" and identity in {"username", "voucher"}:
            stability = "stable"
        elif source == "hotspot" and identity in {"username_or_mac"}:
            stability = "mixed"
        else:
            stability = fallback_stability
        grace_enabled = bool(data.get("grace_enabled", False))
        grace_runs = data.get("grace_runs", 0)
        if stability == "stable":
            recommendation = "Grace can be enabled carefully if short disconnects are common. Keep grace short and use Dry Run."
        elif stability == "mixed":
            recommendation = "Keep grace disabled by default unless users have stable usernames/vouchers. Random MACs can create ghost rows."
        else:
            recommendation = "Avoid grace; use cleanup_immediate or cleanup_next_run based on source behavior."
        items.append({
            "source": source,
            "label": label,
            "identity": identity,
            "stability": stability,
            "grace_enabled": grace_enabled,
            "grace_runs": grace_runs,
            "return_cancels_cleanup": bool(data.get("return_cancels_cleanup", False)),
            "guidance": guidance,
            "recommendation": recommendation,
        })
    return {"sources": items}
