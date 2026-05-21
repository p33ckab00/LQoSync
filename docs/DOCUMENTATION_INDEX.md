# LQoSync Documentation Index

This documentation set is aligned to the current project journey:

```text
Local appliance web app
Rust authority daemon
Python Flask WebUI shell
No Django
No SaaS
```

## Start here

- [Project Canonical Architecture](PROJECT_CANONICAL_ARCHITECTURE.md)
- [Flask UI Shell](FLASK_UI_SHELL.md)
- [Rust Scheduler Authority](RUST_CORE_V810_RUST_SCHEDULER_AUTHORITY.md)
- [Full Rust Stable Operations](FULL_RUST_STABLE_OPERATIONS.md)
- [Installation Matrix](INSTALLATION_MATRIX.md)

## Install / update / uninstall

- [GitHub Install](GITHUB_INSTALL.md)
- [ZIP Install](ZIP_INSTALL.md)
- [Docker Operations](DOCKER_OPERATIONS.md)
- [Bare Metal Install](../BARE_METAL_INSTALL.md)
- [Uninstallation](../UNINSTALLATION.md)


## Operator troubleshooting

- [Operator Troubleshooting Guide](OPERATOR_TROUBLESHOOTING.md)
- [v8.2.1 Operator Error Runbook](RUST_CORE_V821_OPERATOR_TROUBLESHOOTING_DOCS.md)

Use these first when a live install/migration hits Rust/Cargo, Git, rebase, service-start, or old Python/main migration errors.

## Stable release checks

```bash
bash scripts/verify-rust-scheduler-authority.sh
bash scripts/verify-rust-stable-release-cleanup.sh
python3 scripts/release_check.py
python3 scripts/regression_check.py
python3 scripts/stable_release_check.py
```

## Historical notes

Old per-version migration documents may remain in the repository as audit history, but they are no longer the canonical operator path. The canonical path is Rust authority daemon + Flask UI shell.

## v8.2.0 Full Rust Daemon Boundary Cleanup

- [Full Rust Backend Boundary](FULL_RUST_BACKEND_BOUNDARY.md)
- [Rust Core v8.2.0 Full Rust Daemon Boundary Cleanup](RUST_CORE_V820_FULL_RUST_DAEMON_BOUNDARY.md)
- [Workflow Architecture SVG](lqosync_workflow_architecture.svg)

## v8.2.2 Runtime Hotfix

- [Rust Core v8.2.2 Runtime Hotfix](RUST_CORE_V822_RUNTIME_HOTFIX.md)

## v8.2.3 Sync-Plan Gate Import Hardening

- [Rust Sync-Plan Gate Import Hardening](RUST_CORE_V823_SYNC_PLAN_GATE_IMPORT_HARDENING.md)

## v8.2.4 Dashboard Backend Wiring Audit

- [Dashboard Backend Wiring Audit](RUST_CORE_V824_DASHBOARD_BACKEND_WIRING_AUDIT.md)

## v8.2.5 One-Line Operations

- [One-Line Operations Guide](ONE_LINE_OPERATIONS.md)
- [Rust Core v8.2.5 One-Line Operations + Dry-Run Hardening](RUST_CORE_V825_ONE_LINE_OPERATIONS_AND_DRY_RUN_HARDENING.md)
