"""Atomic file helpers for LQoSync.

Python remains the default writer so existing installs do not require Rust.
When LQOSYNC_RUST_ATOMIC_WRITES=1 and lqosync-core is available, these helpers
can delegate write operations to the Rust atomic state engine. The fallback path
still uses tmp-write + fsync + rename + parent-directory fsync.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any


def _rust_atomic_enabled() -> bool:
    return str(os.getenv("LQOSYNC_RUST_ATOMIC_WRITES") or "").strip().lower() in {"1", "true", "yes", "on"}


def _maybe_rust_call(op: str, payload: dict[str, Any]) -> bool:
    if not _rust_atomic_enabled():
        return False
    try:
        from engine.rust_core import call_rust_core
        response = call_rust_core(op, payload, config=None)
        return bool(response.get("available") and response.get("ok") and response.get("result", {}).get("wrote"))
    except Exception:
        return False


def _fsync_parent(path: Path) -> None:
    try:
        parent = path.parent
        fd = os.open(str(parent), os.O_RDONLY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    except Exception:
        # Some filesystems/platforms do not allow directory fsync. The file fsync
        # and os.replace still provide the core atomic write behavior.
        pass


def atomic_write_text(path, content: str, *, file_kind: str = "text", create_backup: bool = False) -> None:
    target = Path(path)
    if _maybe_rust_call("write-text-file", {"path": str(target), "content": content, "file_kind": file_kind, "create_backup": create_backup}):
        return
    target.parent.mkdir(parents=True, exist_ok=True)
    tmp = target.with_name(f".{target.name}.tmp.{os.getpid()}")
    with tmp.open("w", encoding="utf-8", newline="") as f:
        f.write(content)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, target)
    _fsync_parent(target)


def atomic_write_json(path, data: Any, *, file_kind: str = "json_state", sort_keys: bool = False, create_backup: bool = False) -> None:
    text = json.dumps(data, indent=2, ensure_ascii=False, sort_keys=sort_keys) + "\n"
    target = Path(path)
    if _maybe_rust_call("write-json-state", {"path": str(target), "state": data, "state_type": file_kind, "create_backup": create_backup}):
        return
    atomic_write_text(target, text, file_kind=file_kind, create_backup=create_backup)


def append_jsonl(path, event: dict[str, Any]) -> None:
    target = Path(path)
    if _maybe_rust_call("append-audit-jsonl", {"path": str(target), "event": event}):
        return
    target.parent.mkdir(parents=True, exist_ok=True)
    line = json.dumps(event, ensure_ascii=False, sort_keys=True) + "\n"
    with target.open("a", encoding="utf-8") as f:
        f.write(line)
        f.flush()
        os.fsync(f.fileno())
    _fsync_parent(target)
