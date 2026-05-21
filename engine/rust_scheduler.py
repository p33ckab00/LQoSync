"""Rust scheduler authority wrapper for Flask WebUI shell.

Python no longer owns the production scheduler loop when scheduler.engine=rust.
The Flask app uses this module only to ask the Rust authority daemon/binary for
scheduler status, heartbeat, and run-once execution.
"""
from __future__ import annotations

import json
import threading
from pathlib import Path
from typing import Any

from engine.config_loader import load_config
from engine.state import update_state, load_state
from engine.rust_core import call_rust_core


class RustAuthorityScheduler:
    """Flask-compatible scheduler facade backed by lqosync-core.

    It intentionally exposes the small interface expected by app.py:
      - start()
      - stop()
      - is_running()
      - run_now_background(mode)

    In Rust scheduler mode, start() does not spawn a Python scheduler loop. Manual
    runs are delegated to Rust via scheduler-run-once, which then executes the
    configured run-cycle command under Rust scheduler authority.
    """

    def __init__(self, config_path: str | None = None):
        self.config_path = config_path or "/opt/libreqos/src/config.json"
        self.thread: threading.Thread | None = None
        self._lock = threading.Lock()

    def _cfg(self) -> dict[str, Any]:
        return load_config(self.config_path)

    def _state_path(self, cfg: dict[str, Any] | None = None) -> str:
        cfg = cfg or self._cfg()
        return cfg.get("paths", {}).get("runtime_state", "state/runtime_state.json")

    def _payload(self, **extra: Any) -> dict[str, Any]:
        payload = {"config_path": self.config_path}
        payload.update(extra)
        return payload

    def start(self):
        cfg = self._cfg()
        state_path = self._state_path(cfg)
        # Do not start the legacy Python scheduler loop. Rust daemon or Rust
        # scheduler-run-once is the production scheduler authority.
        update_state(
            state_path,
            scheduler_engine="rust",
            scheduler_state="rust_authority",
            scheduler_enabled=bool(cfg.get("scheduler", {}).get("enabled", False)),
            sync_running=False,
            last_error=None,
        )
        call_rust_core("scheduler-heartbeat", self._payload(mode="webui-startup"), config=cfg)

    def stop(self):
        cfg = self._cfg()
        update_state(self._state_path(cfg), scheduler_state="rust_authority_stopped", sync_running=False)

    def is_running(self) -> bool:
        try:
            state = load_state(self._state_path())
            return bool(state.get("sync_running"))
        except Exception:
            return bool(self._lock.locked())

    def status(self) -> dict[str, Any]:
        cfg = self._cfg()
        return call_rust_core("scheduler-status", self._payload(), config=cfg)

    def run_now_background(self, mode: str = "manual") -> bool:
        if self._lock.locked():
            return False
        cfg = self._cfg()
        state_path = self._state_path(cfg)
        update_state(state_path, scheduler_state="queued", sync_running=True, scheduler_engine="rust", last_error=None)

        def target():
            with self._lock:
                cfg2 = self._cfg()
                state_path2 = self._state_path(cfg2)
                update_state(state_path2, scheduler_state="running", sync_running=True, scheduler_engine="rust", last_error=None)
                resp = call_rust_core(
                    "scheduler-run-once",
                    self._payload(mode=mode, execute=True),
                    config=cfg2,
                    timeout=max(30, int(cfg2.get("rust_core", {}).get("scheduler_run_once_timeout_seconds", 1800))),
                )
                ok = bool(resp.get("ok")) and not resp.get("errors")
                update_state(
                    state_path2,
                    scheduler_state="idle" if ok else "error",
                    sync_running=False,
                    last_error=None if ok else "rust_scheduler_run_once_failed",
                    rust_scheduler_last_response=resp,
                )

        self.thread = threading.Thread(target=target, daemon=True, name=f"lqosync-rust-scheduler-{mode}")
        self.thread.start()
        return True


def scheduler_engine_is_rust(config_path: str | None = None) -> bool:
    cfg = load_config(config_path or "/opt/libreqos/src/config.json")
    sched = cfg.get("scheduler", {}) or {}
    return str(sched.get("engine") or "rust").lower() == "rust" and not bool(sched.get("allow_python_scheduler", False))
