# Release Notes

## 2.151.0 - v8.1.0 Rust Scheduler Authority Stable

- Moves scheduler authority to the Rust daemon.
- Keeps Python Flask WebUI as the operator shell only.
- Adds Rust scheduler operations: `scheduler-status`, `scheduler-heartbeat`, `scheduler-decision`, and `scheduler-run-once`.
- Updates `lqosync-core.service` to start the Rust daemon with scheduler authority enabled.
- Adds `engine/rust_scheduler.py` and updates `scheduler/runner.py` so Flask no longer starts the legacy Python scheduler loop when `scheduler.engine=rust`.
- Adds `scripts/run_cycle_once.py` as a stable command target for Rust scheduler authority.
- Adds canonical documentation for the new project journey: local appliance, Rust backend authority, Flask UI shell, no Django, no SaaS.
- Cleans main docs so historical hybrid/Python-backend migration notes are no longer the operator path.

## 2.150.0 - v8.0.0 Rust Backend Stable Cleanup

- Retired legacy Python backend mutation authority.
- Locked stable Rust authority defaults.
- Added stable install and cleanup verification scripts.
- Documented Python as WebUI/scheduler compatibility shell only.
