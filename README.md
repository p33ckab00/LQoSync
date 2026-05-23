# LQoSync

**LQoSync is a local appliance-style web app for MikroTik → LibreQoS synchronization.**

This project is **not Django** and **not a SaaS platform**. The WebUI remains the existing Python Flask interface. The backend authority now runs through the Rust daemon: scheduler/manual cycles, generated-file writes, LibreQoS apply, dry-run preview, run-cycle orchestration, and the legacy PPPoE/DHCP/Hotspot transformation stack have been moved out of Python.

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
  - Singularity policy guardrails
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

Rust is the backend authority target. Flask is retained only because it is already the working operator interface. Until the live RouterOS adapter lands in Rust, the legacy Python collector path remains the compatibility bridge and must not be deleted blindly.

```text
Rust owns:
- scheduler
- run authorization
- Singularity policy validation surface
- collector-output validation
- RouterOS read-result validation
- RouterOS shadow collector bundle generation
- gated read-only RouterOS live adapter pilot
- live-read shadow parity bridge
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
- dry-run compatibility wrappers that now forward into Rust preview operations
- read-only RouterOS connection test helpers and operator diagnostics
- config/user/backup UI support that does not own production run-cycle mutation
```

## Current stable package

```text
v8.2.7 Rust Run-Cycle Backend Retirement
VERSION=2.152.7
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

The Flask UI still exposes the same buttons, but those actions are delegated to Rust scheduler authority. The scheduler and manual run command defaults now enter Rust first through `scripts/rust-run-cycle-authority.sh`, which invokes the Rust core's `run-rust-cycle-authority` operation for scheduled/manual cycles.

Rust now also exposes `build-python-legacy-retirement-inventory` so backend-only Python remnants can be classified separately from the Flask WebUI shell before any guarded cleanup. The Python run-cycle module, Python run-cycle bridge script, Python collector transformation modules, Python duplicate/preflight validators, and Python LibreQoS runner have been retired from the active package.

Flask dry-run preview is now Rust-backed by default. The WebUI/API forwards preview mode into the Rust core's `build-rust-native-dry-run-preview` operation, so plan/live-read/shadow-bundle orchestration for that path no longer lives in Python. That preview now also builds shadow `network.json` topology in Rust through `build-rust-network-json-shadow` and compares both generated backend artifacts.

Manual and scheduled cycles now enter `run-rust-cycle-authority` directly. The WebUI force-apply action also uses Rust `execute-apply-transaction` for LibreQoS execution instead of the retired Python runner.

## Python backend deletion readiness

Do not erase the remaining Python UI shell yet if any of these are still true:

- `rust_core.native_run_cycle_authority_python_fallback` is still allowed anywhere in runtime config
- scheduled/manual runs are not entering `run-rust-cycle-authority` first
- Flask still imports a Python file for UI/config/user/backup/diagnostic behavior that has no Rust/WebUI replacement yet

This branch now runs manual and scheduled cycles through Rust first, and the `scripts/run_cycle_once.py` bridge plus the legacy Python run-cycle/collector/apply modules have been removed. Guarded deletion now focuses on keeping the Flask WebUI shell working while Rust remains the only production backend mutation authority.

## Singularity Policy

Policy presets are being collapsed into one supported operator mode:

```json
{
  "policies": {
    "mode": "singularity"
  }
}
```

Singularity keeps the operator surface simple while preserving safety: normal inactive dynamic rows clean up quickly after successful scans, disabled dynamic sources require confirmation, source failures preserve rows, enabled sources returning zero rows block cleanup, static/manual rows are preserved, and mass-removal guards block cleanup instead of presenting multiple preset personalities.

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
- `docs/SINGULARITY_RUST_BACKEND_CUTOVER.md`
- `docs/RUST_CORE_V810_RUST_SCHEDULER_AUTHORITY.md`
- `docs/INSTALLATION_MATRIX.md`
- `docs/FULL_RUST_STABLE_OPERATIONS.md`

## v8.2.0 Full Rust daemon boundary

LQoSync is documented as a local appliance-style app with this target boundary:

```text
Rust authority daemon = backend authority
Python Flask = WebUI shell only
```

The legacy Python scheduler loop and Python run-cycle backend have been removed. Flask still exposes the same dashboard and action buttons, but scheduler status, heartbeat, run authorization, run-cycle authority, collector-bundle transformation, generated-file writes, and LibreQoS force apply are delegated to Rust.

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

## One-line install/update/uninstall/adopt/check/verify

Fresh install:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- install
```

Update:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- update
```

Uninstall:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- uninstall
```

Adopt user, ownership, and ACLs:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- adopt
```

The standalone adopt path installs/checks ACL tooling, saves the original permission map under `/root/lqosync_permission_snapshots`, creates the `lqosync` runtime user if missing, applies managed-file ownership/ACLs, and verifies temporary-file creation in `/opt/libreqos/src`. Uninstall restores from that original map first.

Check:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- check
```

Verify:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- verify
```
