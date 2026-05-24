#!/usr/bin/env python3
"""LQoSync environment doctor.
Checks config, file permissions, LibreQoS command path, and runtime directories.
Does not contact MikroTik unless --router-test is passed.
"""
from __future__ import annotations
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from engine.config_loader import load_config, validate_config  # noqa: E402
from engine.release_integrity import compute_release_integrity  # noqa: E402
from engine.rust_core import rust_run_routeros_live_read_adapter_pilot  # noqa: E402


def ok(msg): print(f"[OK] {msg}")
def warn(msg): print(f"[WARN] {msg}")
def fail(msg): print(f"[FAIL] {msg}")


def rust_test_router_connection(cfg, router):
    response = rust_run_routeros_live_read_adapter_pilot(cfg, {
        "config": cfg,
        "router": router,
        "adapter": "live",
        "mode": "live_read",
        "execute": True,
        "path": "/system/identity",
        "fields": ["name"],
    })
    result = response.get("result") if isinstance(response.get("result"), dict) else {}
    read_result = result.get("read_result") if isinstance(result.get("read_result"), dict) else {}
    rows = read_result.get("rows") if isinstance(read_result.get("rows"), list) else []
    if response.get("ok") and result.get("status") == "live_read_adapter_read_complete":
        identity = ""
        if rows and isinstance(rows[0], dict):
            identity = str(rows[0].get("name") or "").strip()
        label = str(router.get("name") or "unknown-router")
        if identity:
            return True, f"{label}: Rust live-read adapter ok (identity={identity})"
        return True, f"{label}: Rust live-read adapter ok"
    messages = [
        str(item.get("message") or item.get("code") or "").strip()
        for item in (response.get("errors") or [])
        if isinstance(item, dict) and str(item.get("message") or item.get("code") or "").strip()
    ]
    if not messages and result.get("status"):
        messages.append(str(result.get("status")))
    label = str(router.get("name") or "unknown-router")
    return False, f"{label}: {messages[0] if messages else 'Rust live-read adapter failed'}"


def main():
    config_path = os.getenv("CONFIG_PATH") or (sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else "/opt/libreqos/src/config.json")
    router_test = "--router-test" in sys.argv
    router_test_failed = False
    release_report = compute_release_integrity(ROOT)
    if release_report["summary"]["fail"]:
        fail(f"release integrity failed: {release_report['summary']['fail']} issue(s)")
    elif release_report["summary"]["warn"]:
        warn(f"release integrity has {release_report['summary']['warn']} warning(s)")
    else:
        ok("release integrity passed")

    cfg = load_config(config_path)
    errors, warnings = validate_config(cfg)
    if errors:
        for e in errors: fail(e)
    else:
        ok("config validation passed")
    for w in warnings:
        warn(w)

    for key in ("shaped_devices_csv", "network_json"):
        p = Path(cfg["paths"][key])
        if p.exists():
            ok(f"{key} exists: {p}")
            if os.access(p, os.R_OK | os.W_OK): ok(f"{key} readable/writable")
            else: fail(f"{key} not readable/writable by current user")
        else:
            warn(f"{key} does not exist yet: {p}")

    for key in ("backup_dir", "log_file", "runtime_state", "lock_file", "audit_log"):
        p = Path(cfg["paths"].get(key, ""))
        if not str(p):
            warn(f"paths.{key} is empty")
            continue
        parent = p if key == "backup_dir" else p.parent
        try:
            parent.mkdir(parents=True, exist_ok=True)
            test = parent / ".lqosync_write_test"
            test.write_text("ok", encoding="utf-8")
            test.unlink()
            ok(f"{key} parent writable: {parent}")
        except Exception as e:
            fail(f"{key} parent not writable: {parent} ({e})")

    lqcmd = Path(cfg.get("libreqos", {}).get("cmd", ""))
    if lqcmd.exists(): ok(f"LibreQoS command exists: {lqcmd}")
    else: warn(f"LibreQoS command not found: {lqcmd}")

    if router_test:
        for r in cfg.get("routers", []):
            if not r.get("enabled", True):
                continue
            ok_result, message = rust_test_router_connection(cfg, r)
            if ok_result:
                ok(message)
            else:
                fail(message)
                router_test_failed = True

    return 1 if errors or router_test_failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
