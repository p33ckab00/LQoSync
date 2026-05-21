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
