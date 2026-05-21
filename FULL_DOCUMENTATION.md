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


## Operator troubleshooting

If installation or migration fails, start with:

```text
docs/OPERATOR_TROUBLESHOOTING.md
```

It covers missing Cargo, old Cargo with `Cargo.lock` version 4, Git `fetch first` / `non-fast-forward`, rebase conflict recovery, production-safe `enable_only` service behavior, and old Python/main to `lqosync-in-rust` migration.


### v8.2.3 Rust Sync-Plan Gate Import Hardening

If the WebUI reports `name 'rust_sync_plan_authority_gate' is not defined`, apply v8.2.3 and run:

```bash
bash scripts/verify-rust-sync-plan-gate-import-hardening.sh
find /opt/LQoSync -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
sudo systemctl restart lqosync
```

Collector parity warnings are separate and indicate Rust MikroTik collector parity is not complete yet.
