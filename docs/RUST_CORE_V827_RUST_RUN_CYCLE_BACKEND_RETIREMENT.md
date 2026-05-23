# v8.2.7 Rust Run-Cycle Backend Retirement

LQoSync now routes production backend authority through Rust for scheduled/manual cycles, dry-run preview, generated-file writes, and LibreQoS force apply while preserving the existing Flask WebUI shell.

## Retired from Python backend authority

- `engine/run_cycle.py`
- `scripts/run_cycle_once.py`
- Python PPPoE/DHCP/Hotspot collector transformation modules
- Python duplicate/preflight validators
- Python LibreQoS runner

These files are removed from the active package because Rust `run-rust-cycle-authority`, Rust collector bundle generation, Rust validation, Rust apply manifests, Rust apply transactions, and Rust live-read gates now own that backend path.

## Still intentionally Python

- Flask pages, forms, sessions, users, and docs
- Config/user/backup/diagnostic support imported by `app.py`
- Read-only RouterOS connection-test helpers for the operator UI
- Rust protocol wrapper functions in `engine/rust_core.py`

These remaining Python files are the WebUI shell and local operator support. They are not production run-cycle authority.

## Operator verification

```bash
bash scripts/verify-full-rust-daemon-boundary.sh
bash scripts/verify-rust-scheduler-authority.sh
bash scripts/verify-one-line-operations.sh
bash scripts/verify-rust-stable-release-cleanup.sh
```

## One-line operations

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

Adopt runtime user, ownership, ACLs, and managed-file permissions:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- adopt
```
