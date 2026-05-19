"""Optional Python wrapper for the LQoSync Rust safety core.

The Rust core is introduced as an optional sidecar. If the binary is missing or
returns malformed output, Python keeps the existing sync path and records a
non-blocking unavailable/fallback result. This allows the `lqosync-in-rust`
branch to harden deterministic validation without breaking current installs.
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import time
from pathlib import Path
from typing import Any

PROTOCOL_VERSION = "1"
DEFAULT_SOCKET = "/run/lqosync-core.sock"


def _project_root() -> Path:
    return Path(__file__).resolve().parent.parent


def rust_core_config(config: dict | None = None) -> dict:
    cfg = (config or {}).get("rust_core", {}) if isinstance(config, dict) else {}
    return {
        "enabled": bool(cfg.get("enabled", True)),
        "binary_path": str(cfg.get("binary_path") or os.getenv("LQOSYNC_CORE_BIN") or "").strip(),
        "timeout_seconds": int(cfg.get("timeout_seconds", os.getenv("LQOSYNC_CORE_TIMEOUT", 10)) or 10),
        "enforce_validation": bool(cfg.get("enforce_validation", False)),
        "prefer_daemon": bool(cfg.get("prefer_daemon", False)),
        "unix_socket": str(cfg.get("unix_socket") or os.getenv("LQOSYNC_CORE_SOCKET") or DEFAULT_SOCKET),
    }


def find_rust_core_binary(config: dict | None = None) -> str | None:
    rc = rust_core_config(config)
    if not rc["enabled"]:
        return None
    candidates = []
    if rc["binary_path"]:
        candidates.append(Path(rc["binary_path"]))
    candidates.extend([
        _project_root() / "rust" / "lqosync-core" / "target" / "release" / "lqosync-core",
        _project_root() / "rust" / "lqosync-core" / "target" / "debug" / "lqosync-core",
    ])
    which = shutil.which("lqosync-core")
    if which:
        candidates.append(Path(which))
    candidates.append(Path("/usr/local/bin/lqosync-core"))

    for candidate in candidates:
        try:
            if candidate.exists() and os.access(candidate, os.X_OK):
                return str(candidate)
        except Exception:
            continue
    return None


def rust_core_status(config: dict | None = None) -> dict[str, Any]:
    rc = rust_core_config(config)
    binary = find_rust_core_binary(config)
    return {
        "enabled": rc["enabled"],
        "available": bool(binary),
        "binary": binary,
        "timeout_seconds": rc["timeout_seconds"],
        "enforce_validation": rc["enforce_validation"],
        "prefer_daemon": rc["prefer_daemon"],
        "unix_socket": rc["unix_socket"],
        "mode": "subprocess" if binary else "python_fallback",
    }


def call_rust_core(op: str, payload: dict[str, Any] | None = None, *, config: dict | None = None, request_id: str | None = None, timeout: int | None = None) -> dict[str, Any]:
    """Call the Rust core through the stable JSON envelope.

    The same envelope is intended for the future Unix socket daemon. v0.1 uses
    subprocess because it is easy to deploy and debug.
    """
    started = time.perf_counter()
    binary = find_rust_core_binary(config)
    rc = rust_core_config(config)
    if not binary:
        return {
            "version": PROTOCOL_VERSION,
            "op": op,
            "request_id": request_id,
            "available": False,
            "ok": True,
            "skipped": True,
            "result": {},
            "errors": [],
            "warnings": [{
                "code": "rust_core_unavailable",
                "severity": "info",
                "message": "Rust core binary is not installed/built; Python validator fallback is active.",
            }],
            "meta": {
                "engine": "python-wrapper",
                "mode": "python_fallback",
                "duration_ms": round((time.perf_counter() - started) * 1000, 3),
            },
        }

    request_payload = {
        "version": PROTOCOL_VERSION,
        "op": op,
        "request_id": request_id,
        "payload": payload or {},
    }
    try:
        proc = subprocess.run(
            [binary],
            input=json.dumps(request_payload, ensure_ascii=False),
            text=True,
            capture_output=True,
            timeout=timeout or rc["timeout_seconds"],
        )
    except subprocess.TimeoutExpired:
        return _wrapper_error(op, request_id, "rust_core_timeout", f"Rust core timed out after {timeout or rc['timeout_seconds']} seconds", started, available=True)
    except Exception as exc:
        return _wrapper_error(op, request_id, "rust_core_call_failed", str(exc), started, available=True)

    try:
        response = json.loads(proc.stdout or "{}")
    except Exception as exc:
        return _wrapper_error(
            op,
            request_id,
            "rust_core_invalid_response",
            f"Rust core returned invalid JSON: {exc}; stderr={proc.stderr[:500]}",
            started,
            available=True,
        )

    response.setdefault("available", True)
    response.setdefault("meta", {})
    response["meta"].setdefault("wrapper_duration_ms", round((time.perf_counter() - started) * 1000, 3))
    response["meta"].setdefault("exit_code", proc.returncode)
    if proc.stderr:
        response["meta"].setdefault("stderr", proc.stderr[:1000])
    return response


def validate_runtime_outputs(config: dict, *, csv_text: str | None = None, network_text: str | None = None, csv_path: str | None = None, network_path: str | None = None) -> dict[str, Any]:
    paths = (config or {}).get("paths", {})
    payload = {
        "config": config or {},
        "csv_text": csv_text,
        "network_text": network_text,
        "shaped_devices_csv_path": csv_path or paths.get("shaped_devices_csv"),
        "network_json_path": network_path or paths.get("network_json"),
    }
    # Avoid sending null text values because the Rust side treats present text as
    # authoritative. If text is None, let the path fallback be used.
    payload = {k: v for k, v in payload.items() if v is not None}
    return call_rust_core("validate-files", payload, config=config)


def validate_collector_output(config: dict, envelope: dict[str, Any]) -> dict[str, Any]:
    return call_rust_core("validate-collector-output", envelope, config=config)


def diagnostics_to_messages(response: dict[str, Any], *, include_warnings: bool = True) -> tuple[list[str], list[str]]:
    errors = []
    warnings = []
    for item in response.get("errors") or []:
        msg = item.get("message") or item.get("code") or "Rust core error"
        errors.append(f"Rust core: {msg}")
    if include_warnings:
        for item in response.get("warnings") or []:
            msg = item.get("message") or item.get("code") or "Rust core warning"
            severity = item.get("severity", "warning")
            if severity == "info":
                warnings.append(f"Rust core: {msg}")
            else:
                warnings.append(f"Rust core: {msg}")
    return errors, warnings


def _wrapper_error(op: str, request_id: str | None, code: str, message: str, started: float, *, available: bool) -> dict[str, Any]:
    return {
        "version": PROTOCOL_VERSION,
        "op": op,
        "request_id": request_id,
        "available": available,
        "ok": False,
        "result": {},
        "errors": [{"code": code, "severity": "error", "message": message}],
        "warnings": [],
        "meta": {
            "engine": "python-wrapper",
            "mode": "subprocess",
            "duration_ms": round((time.perf_counter() - started) * 1000, 3),
        },
    }
