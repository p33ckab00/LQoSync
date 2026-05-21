# Release Notes

## 2.152.3 - v8.2.3 Rust Sync-Plan Gate Import Hardening

- Hardened `engine/run_cycle.py` against live NameError failures for `rust_sync_plan_authority_gate` by adding a defensive local import at the call site while preserving the existing top-level import.
- Added `scripts/verify-rust-sync-plan-gate-import-hardening.sh`.
- Added operator documentation for the exact runtime error and clarified that collector parity warnings are separate from the import hotfix.
- No WebUI redesign, scheduler rollback, or Rust authority behavior change.

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

## 2.152.0 - v8.2.0 Full Rust Daemon Boundary Cleanup

- Removed the legacy Python scheduler loop from `scheduler/runner.py`; the Flask scheduler facade now delegates every scheduler action to Rust authority.
- Added the canonical workflow architecture SVG with the Python legacy scheduler loop removed.
- Added `docs/RUST_CORE_V820_FULL_RUST_DAEMON_BOUNDARY.md` and `docs/FULL_RUST_BACKEND_BOUNDARY.md`.
- Updated stable defaults so Python fallback flags are false by default across Rust handoff/authority settings.
- Retained Flask WebUI as-is. This is not a Django or SaaS migration.
- Clarified that remaining Python files are UI/diagnostic/compatibility shell files, not production mutation authority.


## 2.152.1 - v8.2.1 Operator Troubleshooting Documentation

- Added `docs/OPERATOR_TROUBLESHOOTING.md` with real-world fixes for missing Cargo, old Cargo/Cargo.lock v4, Git push rejection, non-fast-forward divergence, rebase conflict recovery, service `enable_only` behavior, and old Python/main to `lqosync-in-rust` migration.
- Added `docs/RUST_CORE_V821_OPERATOR_TROUBLESHOOTING_DOCS.md` as the release runbook.
- Added `scripts/verify-operator-troubleshooting-docs.sh`.
- Updated README, installation, full documentation, package notes, documentation index, and docs manifest so operators know where to look when install/migration errors happen.
- No runtime authority, Flask WebUI, Rust scheduler, or production mutation behavior changed.

## 2.152.2 - v8.2.2 Rust Authority Runtime Hotfix

- Fixed a runtime `NameError` where `engine.run_cycle` called `rust_sync_plan_authority_gate` without importing it from `engine.rust_core`.
- Added operator guidance for Rust shadow collector warnings that indicate Python collector transport is still selected until Rust collector parity is proven.
- No WebUI redesign and no live LibreQoS file mutation behavior change.
