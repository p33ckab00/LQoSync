# LQoSync Full Documentation

LQoSync is a local appliance-style MikroTik to LibreQoS synchronization web app.

Current stable architecture:

```text
Rust authority daemon + Flask WebUI shell
```

This is not a SaaS platform and not a Django migration.

## Core documents

- `README.md`
- `INSTALLATION.md`
- `BARE_METAL_INSTALL.md`
- `GIT_INSTALL.md`
- `DOCKER_INSTALL.md`
- `UNINSTALLATION.md`
- `docs/PROJECT_CANONICAL_ARCHITECTURE.md`
- `docs/FLASK_UI_SHELL.md`
- `docs/RUST_CORE_V810_RUST_SCHEDULER_AUTHORITY.md`
- `docs/FULL_RUST_STABLE_OPERATIONS.md`
- `docs/INSTALLATION_MATRIX.md`

## Stable verification

```bash
bash scripts/verify-rust-scheduler-authority.sh
python3 scripts/release_check.py
python3 scripts/regression_check.py
python3 scripts/stable_release_check.py
```
