import threading
from datetime import datetime, timezone, timedelta

from engine.config_loader import load_config
from engine.run_cycle import run_cycle
from engine.state import update_state, load_state
from engine.logging_utils import log_event
from engine.rust_scheduler import RustAuthorityScheduler


class LQoSyncScheduler:
    def __init__(self, config_path=None):
        self.config_path = config_path
        self.thread = None
        self.stop_event = threading.Event()
        self.lock = threading.Lock()
        self.current_job = None
        self.rust = RustAuthorityScheduler(config_path)

    def _use_rust_scheduler(self, cfg=None):
        cfg = cfg or load_config(self.config_path)
        sched = cfg.get("scheduler", {}) or {}
        return str(sched.get("engine") or "rust").lower() == "rust" and not bool(sched.get("allow_python_scheduler", False))

    def start(self):
        cfg = load_config(self.config_path)
        if self._use_rust_scheduler(cfg):
            # v8.1.0: Python no longer starts the production scheduler loop.
            # Flask keeps the same WebUI object, but Rust owns scheduler authority.
            return self.rust.start()
        if self.thread and self.thread.is_alive():
            return
        self.stop_event.clear()
        self.thread = threading.Thread(target=self._loop, daemon=True, name="lqosync-python-legacy-scheduler")
        self.thread.start()

    def stop(self):
        cfg = load_config(self.config_path)
        if self._use_rust_scheduler(cfg):
            return self.rust.stop()
        self.stop_event.set()

    def is_running(self):
        cfg = load_config(self.config_path)
        if self._use_rust_scheduler(cfg):
            return self.rust.is_running()
        return self.lock.locked()

    def _set_next_run(self, cfg, seconds):
        state_path = cfg["paths"].get("runtime_state", "state/runtime_state.json")
        next_run_at = (datetime.now(timezone.utc) + timedelta(seconds=int(seconds))).isoformat()
        update_state(state_path, next_run_at=next_run_at)

    def _loop(self):
        while not self.stop_event.is_set():
            cfg = load_config(self.config_path)
            if self._use_rust_scheduler(cfg):
                self.rust.start()
                self.stop_event.wait(5)
                continue
            state_path = cfg["paths"].get("runtime_state", "state/runtime_state.json")
            if not cfg.get("scheduler", {}).get("enabled", False):
                update_state(state_path, scheduler_enabled=False, scheduler_state="disabled", sync_running=False, next_run_at=None)
                self.stop_event.wait(5)
                continue

            update_state(state_path, scheduler_enabled=True, scheduler_engine="python_legacy")
            if self.lock.acquire(blocking=False):
                try:
                    res = run_cycle(mode="scheduled", config_path=self.config_path)
                    interval = cfg["scheduler"].get("active_interval_seconds", 30) if res.files_changed else cfg["scheduler"].get("idle_interval_seconds", 120)
                    update_state(state_path, scheduler_state="idle")
                except Exception as exc:
                    log_event(cfg, "error", f"Legacy Python scheduler loop error: {exc}")
                    interval = cfg["scheduler"].get("error_retry_interval_seconds", 30)
                    update_state(state_path, scheduler_state="error", last_error=str(exc))
                finally:
                    self.lock.release()
            else:
                log_event(cfg, "warning", "Scheduled run skipped: sync already running")
                interval = cfg["scheduler"].get("active_interval_seconds", 30)

            self._set_next_run(cfg, interval)
            self.stop_event.wait(int(interval))

    def run_now_background(self, mode="manual"):
        cfg = load_config(self.config_path)
        if self._use_rust_scheduler(cfg):
            return self.rust.run_now_background(mode)
        if self.lock.locked():
            return False

        try:
            state_path0 = cfg["paths"].get("runtime_state", "state/runtime_state.json")
            update_state(state_path0, scheduler_state="queued", sync_running=True, last_error=None, scheduler_engine="python_legacy")
        except Exception:
            pass

        def target():
            cfg2 = load_config(self.config_path)
            state_path = cfg2["paths"].get("runtime_state", "state/runtime_state.json")
            if not self.lock.acquire(blocking=False):
                update_state(state_path, last_error="sync already running")
                return
            try:
                update_state(state_path, scheduler_state="running", sync_running=True, last_error=None, scheduler_engine="python_legacy")
                run_cycle(mode=mode, config_path=self.config_path)
            finally:
                self.lock.release()

        t = threading.Thread(target=target, daemon=True, name=f"lqosync-python-legacy-{mode}")
        t.start()
        return True
