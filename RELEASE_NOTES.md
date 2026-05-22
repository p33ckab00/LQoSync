# Release Notes

## 2.152.6 - v8.2.6 Python Legacy Retirement Inventory

- Added Rust operation `build-python-legacy-retirement-inventory` to classify Flask WebUI shell files separately from legacy backend cleanup candidates.
- Added `/api/rust-core/python-legacy-retirement-inventory` as a WebUI-shell API wrapper with no Python fallback authority.
- Added config gates, documentation, docs manifest entry, and stable cleanup verification coverage.
- Kept cleanup non-mutating by design: `delete_allowed=false`, `side_effects_allowed=false`, and archive planning only reports ready after audit-sentinel, rollback, WebUI, and operator gates pass.

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

## 2.152.4 - v8.2.4 Dashboard Backend Wiring Audit

- Added `engine/dashboard_modules.py`, `/api/dashboard/modules`, and a Dashboard Backend Wiring card.
- Added read-only verification for Dashboard modules and their backend providers.
- Added `scripts/verify-dashboard-backend-wiring.sh` and documentation for operator-facing module wiring checks.
- No collector, apply, scheduler, or WebUI redesign behavior changed.

## 2.152.5 - v8.2.5 One-Line Operations + Dry-Run Hardening

- Added `lqosyncctl.sh`, a one-line operator control script for GitHub fresh install, update, check, verify, start, stop, restart, and repair.
- The one-line script handles root execution, `/opt/LQoSync` Git safe.directory, rustup stable Cargo installation/update, live-file backups, GitHub branch update from `lqosync-in-rust`, stable install, and verification.
- Hardened `/sync/dry-run` and `/api/sync/dry-run` so runtime exceptions render actionable diagnostics instead of raw Internal Server Error pages.
- Added `docs/ONE_LINE_OPERATIONS.md` and `docs/RUST_CORE_V825_ONE_LINE_OPERATIONS_AND_DRY_RUN_HARDENING.md`.
- Added `scripts/verify-one-line-operations.sh`.
