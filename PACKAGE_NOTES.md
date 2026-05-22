# Package Notes

Package: v8.2.6 Python Legacy Retirement Inventory
Version: 2.152.6

This package aligns the codebase and documentation to the current project journey:

```text
Rust authority daemon + Flask WebUI shell
```

It is not Django and not SaaS.

## Key boundary

- Rust owns scheduler authority and production mutation.
- Flask owns the existing WebUI only.
- The old Python scheduler loop is disabled by default when `scheduler.engine=rust`.

## Required verification

```bash
bash scripts/verify-rust-scheduler-authority.sh
bash scripts/verify-rust-stable-release-cleanup.sh
python3 scripts/release_check.py
python3 scripts/regression_check.py
python3 scripts/stable_release_check.py
```

## v8.2.0 Full Rust Daemon Boundary Cleanup

This package removes the stale Python scheduler loop and aligns the workflow diagram/documentation to the stable journey: Rust authority daemon + Flask WebUI shell. Python remains only where required for UI, diagnostics, and compatibility transport. Production mutation fallback is disabled by default.


## Operator troubleshooting

If installation or migration fails, start with:

```text
docs/OPERATOR_TROUBLESHOOTING.md
```

It covers missing Cargo, old Cargo with `Cargo.lock` version 4, Git `fetch first` / `non-fast-forward`, rebase conflict recovery, production-safe `enable_only` service behavior, and old Python/main to `lqosync-in-rust` migration.

## v8.2.2 runtime hotfix

This package fixes the missing `rust_sync_plan_authority_gate` import seen during live WebUI/run-cycle testing. If the UI shows Rust-shadow collector warnings, treat them as authority-readiness warnings: scheduler/apply authority may be Rust-owned, but collector transport/parity still needs validation before claiming complete Rust collector authority.

## v8.2.3 Rust Sync-Plan Gate Import Hardening

This package hardens the Rust sync-plan authority gate import used by `engine/run_cycle.py` so live WebUI cycles do not fail with `name 'rust_sync_plan_authority_gate' is not defined`. It adds a verification script and operator documentation.

## v8.2.4 Dashboard Backend Wiring Audit

- Added `engine/dashboard_modules.py`, `/api/dashboard/modules`, and a Dashboard Backend Wiring card.
- Added read-only verification for Dashboard modules and their backend providers.
- Added `scripts/verify-dashboard-backend-wiring.sh` and documentation for operator-facing module wiring checks.
- No collector, apply, scheduler, or WebUI redesign behavior changed.

## v8.2.5 One-Line Operations + Dry-Run Hardening

This package adds `lqosyncctl.sh` for one-line fresh install, update, check, verify, start, stop, restart, and repair from the `lqosync-in-rust` GitHub branch. It also catches dry-run route exceptions and displays operator diagnostics instead of raw Internal Server Error pages.

## v8.2.6 Python Legacy Retirement Inventory

This package adds Rust operation `build-python-legacy-retirement-inventory` and `/api/rust-core/python-legacy-retirement-inventory`. It preserves Flask WebUI shell files, marks backend-only Python paths as guarded archive candidates, and keeps `delete_allowed=false` until rollback-aware cleanup is explicitly run outside Rust core.
