"""Flask scheduler facade for the Rust authority daemon.

v8.2.0 removes the legacy Python scheduler loop from the production code path. Python no longer starts the production scheduler loop.
The Flask WebUI keeps the historical ``LQoSyncScheduler`` object so templates,
routes, and manual buttons do not change, but every scheduler action is now
delegated to ``lqosync-core`` through ``RustAuthorityScheduler``.
"""
from __future__ import annotations

from engine.config_loader import load_config
from engine.rust_scheduler import RustAuthorityScheduler


class LQoSyncScheduler:
    """Compatibility facade used by app.py.

    This class intentionally does not start a Python background loop. Python is
    the WebUI shell only; Rust owns scheduler status, heartbeat, run decisions,
    run-once execution, locks, and production authority checks.
    """

    def __init__(self, config_path=None):
        self.config_path = config_path
        self.rust = RustAuthorityScheduler(config_path)

    def _assert_rust_scheduler(self):
        cfg = load_config(self.config_path)
        sched = cfg.get("scheduler", {}) or {}
        if str(sched.get("engine") or "rust").lower() != "rust":
            raise RuntimeError("scheduler.engine must be 'rust'; Python scheduler authority has been removed")
        if bool(sched.get("allow_python_scheduler", False)):
            raise RuntimeError("scheduler.allow_python_scheduler must be false; Python scheduler authority has been removed")
        return cfg

    def start(self):
        self._assert_rust_scheduler()
        return self.rust.start()

    def stop(self):
        self._assert_rust_scheduler()
        return self.rust.stop()

    def is_running(self):
        self._assert_rust_scheduler()
        return self.rust.is_running()

    def status(self):
        self._assert_rust_scheduler()
        return self.rust.status()

    def run_now_background(self, mode="manual"):
        self._assert_rust_scheduler()
        return self.rust.run_now_background(mode)
