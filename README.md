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
