# LQoSync

**LQoSync is a local appliance-style web app for MikroTik → LibreQoS synchronization.**

This project is **not Django** and **not a SaaS platform**. The WebUI remains the existing Python Flask interface. The backend authority is now the Rust daemon.

## Canonical architecture

```text
MikroTik API sources
  - PPPoE
  - DHCP / IPoE
  - Hotspot
  - static/manual mappings
        ↓
lqosync-core.service
  - one Rust authority daemon
  - scheduler authority
  - validation
  - sync plan
  - transaction journal
  - ShapedDevices.csv writer
  - network.json writer
  - LibreQoS.py --updateonly executor
  - rollback / quarantine / watchdog
        ↓
LibreQoS external middlebox

lqosync web service
  - Python Flask WebUI shell only
  - dashboard, config, dry-run, operations, docs
  - calls the Rust daemon through /run/lqosync-core.sock
```

## Runtime boundary

Rust owns production backend authority. Flask is retained only because it is already the working operator interface.

```text
Rust owns:
- scheduler
- run authorization
- collector-output validation
- sync-plan enforcement
- file write authority
- transaction journal
- LibreQoS apply authority
- recovery bundle
- quarantine
- set-and-forget readiness gates

Python Flask owns:
- WebUI pages
- sessions/login/admin shell
- forms/buttons/API wrappers
- displaying Rust results
- compatibility transport shell where still required
```

## Current stable package

```text
v8.1.0 Rust Scheduler Authority
VERSION=2.151.0
```

The old Python scheduler loop is retired by default:

```json
{
  "scheduler": {
    "engine": "rust",
    "allow_python_scheduler": false
  }
}
```

The Flask UI still exposes the same buttons, but those actions are delegated to Rust scheduler authority.

## Install

For live/local appliance install:

```bash
sudo bash install-rust-stable-safe.sh
```

Manual controlled install:

```bash
sudo bash install-production-safe.sh
sudo bash scripts/build-rust-core.sh
sudo bash scripts/install-rust-core.sh
sudo bash scripts/install-rust-core-daemon.sh
sudo bash scripts/promote-rust-full-authoritative-safe.sh
```

Verify:

```bash
bash scripts/verify-rust-scheduler-authority.sh
python3 scripts/release_check.py
python3 scripts/regression_check.py
python3 scripts/stable_release_check.py
```

## Do not migrate to Django

Django is not part of this appliance path. The correct direction is:

```text
Rust authority daemon + existing Flask WebUI shell
```

## Documentation

Start here:

- `docs/PROJECT_CANONICAL_ARCHITECTURE.md`
- `docs/FLASK_UI_SHELL.md`
- `docs/RUST_CORE_V810_RUST_SCHEDULER_AUTHORITY.md`
- `docs/INSTALLATION_MATRIX.md`
- `docs/FULL_RUST_STABLE_OPERATIONS.md`

## v8.2.0 Full Rust daemon boundary

LQoSync is now documented as a local appliance-style app with this boundary:

```text
Rust authority daemon = backend authority
Python Flask = WebUI shell only
```

The legacy Python scheduler loop has been removed. Flask still exposes the same dashboard and action buttons, but scheduler status, heartbeat, and run authorization are delegated to `lqosync-core`.

This project is not being converted to Django and is not a SaaS platform.

Canonical workflow diagram:

```text
docs/lqosync_workflow_architecture.svg
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

## Dashboard Backend Wiring

The Dashboard includes a read-only backend wiring audit and `/api/dashboard/modules` endpoint so operators can confirm every visible module is connected to its backend provider.

## One-line install/update/check/verify

Fresh install:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- install
```

Update:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- update
```

Check:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- check
```

Verify:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- verify
```
