"""Router overview and multi-router UX helpers.

v2.69 adds a read-only router overview so operators can see which MikroTik
routers are configured, which sources are enabled per router, how many rows each
router appears to own, and where to go next for configuration, dry-run, or
operations.
"""
from __future__ import annotations

from typing import Any


def _as_bool(value: Any, default: bool = False) -> bool:
    if value is None:
        return default
    return bool(value)


def _source_enabled(router: dict[str, Any], source: str) -> bool:
    block = router.get(source) or {}
    return bool(block.get("enabled", False))


def _router_error_count(last: dict[str, Any], router_name: str) -> tuple[int, list[str]]:
    errors = []
    for err in last.get("router_errors", []) or []:
        if str(err.get("router") or "") == router_name:
            msg = f"{err.get('source') or 'router'}: {err.get('error') or 'error'}"
            errors.append(msg)
    return len(errors), errors


def _collector_metrics_for_router(last: dict[str, Any], router_name: str) -> dict[str, Any]:
    diff = last.get("diff") or {}
    metrics = diff.get("collector_metrics") or {}
    out = {}
    for key, value in metrics.items():
        if not isinstance(key, str) or not isinstance(value, dict):
            continue
        if key.startswith(f"{router_name}."):
            out[key.split(".", 1)[1]] = value
    return out


def _rows_for_router(rows: dict[str, dict[str, Any]], router_name: str) -> dict[str, int]:
    counts = {"total": 0, "pppoe": 0, "dhcp": 0, "hotspot": 0, "static": 0, "other": 0}
    token = f"-{router_name}".lower()
    router_lower = router_name.lower()
    for row in (rows or {}).values():
        parent = str(row.get("Parent Node") or "").lower()
        comment = str(row.get("Comment") or "").lower()
        # Generated node names normally end with -{router}; comments are less reliable.
        if token not in parent and router_lower not in parent:
            continue
        counts["total"] += 1
        if comment == "ppp":
            counts["pppoe"] += 1
        elif comment == "hs":
            counts["hotspot"] += 1
        elif comment == "static":
            counts["static"] += 1
        elif comment.startswith("dhcp") or comment not in ("", "ppp", "hs", "static"):
            counts["dhcp"] += 1
        else:
            counts["other"] += 1
    return counts


def _status(router: dict[str, Any], row_counts: dict[str, int], metrics: dict[str, Any], error_count: int) -> tuple[str, list[str]]:
    warnings = []
    if not _as_bool(router.get("enabled"), True):
        return "disabled", ["Router is disabled in config."]
    if error_count:
        warnings.append(f"{error_count} collector/router error(s) on last run.")
    enabled_sources = [s for s in ("pppoe", "dhcp", "hotspot") if _source_enabled(router, s)]
    if not enabled_sources:
        warnings.append("No PPPoE, DHCP, or Hotspot source is enabled for this router.")
    if enabled_sources and row_counts.get("total", 0) == 0:
        warnings.append("Sources are enabled but no generated rows currently match this router name.")
    for source in enabled_sources:
        m = metrics.get(source) or {}
        if source == "dhcp" and m.get("servers_enabled", 0) == 0:
            warnings.append("DHCP is enabled but no DHCP server entry is enabled.")
        if source in metrics and int((m.get("generated_rows") or 0)) == 0:
            warnings.append(f"{source.upper()} collector generated zero rows on last run.")
    if error_count:
        return "error", warnings
    if warnings:
        return "warning", warnings
    return "ok", []


def compute_router_overview(config: dict[str, Any], state: dict[str, Any] | None = None, rows: dict[str, dict[str, Any]] | None = None) -> dict[str, Any]:
    """Return a read-only router overview view-model."""
    state = state or {}
    rows = rows or {}
    last = state.get("last_run") or state.get("last_dry_run") or {}
    routers = config.get("routers") or []
    items = []
    totals = {"routers": len(routers), "enabled": 0, "ok": 0, "warning": 0, "error": 0, "disabled": 0, "rows": 0, "sources_enabled": 0}
    for idx, router in enumerate(routers):
        name = str(router.get("name") or f"router-{idx+1}")
        row_counts = _rows_for_router(rows, name)
        metrics = _collector_metrics_for_router(last, name)
        error_count, errors = _router_error_count(last, name)
        status, warnings = _status(router, row_counts, metrics, error_count)
        source_blocks = {
            "pppoe": {
                "enabled": _source_enabled(router, "pppoe"),
                "rows": row_counts.get("pppoe", 0),
                "metrics": metrics.get("pppoe") or {},
            },
            "dhcp": {
                "enabled": _source_enabled(router, "dhcp"),
                "servers_enabled": len([s for s in ((router.get("dhcp") or {}).get("servers") or []) if s.get("enabled", True)]),
                "servers_total": len(((router.get("dhcp") or {}).get("servers") or [])),
                "rows": row_counts.get("dhcp", 0),
                "metrics": metrics.get("dhcp") or {},
            },
            "hotspot": {
                "enabled": _source_enabled(router, "hotspot"),
                "rows": row_counts.get("hotspot", 0),
                "metrics": metrics.get("hotspot") or {},
            },
        }
        sources_enabled = sum(1 for v in source_blocks.values() if v.get("enabled"))
        totals["sources_enabled"] += sources_enabled
        if router.get("enabled", True):
            totals["enabled"] += 1
        totals[status] = totals.get(status, 0) + 1
        totals["rows"] += row_counts.get("total", 0)
        items.append({
            "index": idx,
            "name": name,
            "enabled": bool(router.get("enabled", True)),
            "address": router.get("address") or "",
            "port": router.get("port") or 8728,
            "username": router.get("username") or "",
            "parent_node": router.get("parent_node") or "",
            "root_download_mbps": router.get("root_download_mbps"),
            "root_upload_mbps": router.get("root_upload_mbps"),
            "root_type": router.get("root_type") or "site",
            "root_virtual": bool(router.get("root_virtual", False)),
            "role": "child" if router.get("parent_node") else "root",
            "status": status,
            "warnings": warnings,
            "errors": errors,
            "sources_enabled": sources_enabled,
            "sources": source_blocks,
            "row_counts": row_counts,
            "collector_metrics": metrics,
        })
    recommendations = []
    if not routers:
        recommendations.append("Add at least one MikroTik router in Config Center before enabling scheduler.")
    if totals.get("error"):
        recommendations.append("Open Operations Center or Dry Run to inspect router/source collector errors.")
    if totals.get("warning"):
        recommendations.append("Review routers with warnings before changing cleanup or auto-apply policy.")
    if totals.get("sources_enabled", 0) == 0 and routers:
        recommendations.append("Enable PPPoE, DHCP, or Hotspot source collection for at least one router.")
    return {
        "routers": items,
        "totals": totals,
        "recommendations": recommendations,
        "network_mode": config.get("network_mode") or "router_children",
        "last_run_at": last.get("finished_at") or last.get("started_at") or "",
    }
