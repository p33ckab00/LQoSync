# v8.2.5 One-Line Operations + Dry-Run Hardening

This release adds `lqosyncctl.sh`, a one-line operator control script for fresh install, update, check, verify, start, stop, restart, and repair.

It also hardens `/sync/dry-run` and `/api/sync/dry-run` so route exceptions are shown as actionable diagnostics instead of raw Internal Server Error pages.

## No runtime authority change

This release does not change collector authority, apply authority, scheduler authority, or WebUI design. It only improves operator workflows and error handling.

## One-line examples

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- install
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- update
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- check
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- verify
```

## Verification

```bash
bash scripts/verify-one-line-operations.sh
python3 scripts/release_check.py
python3 scripts/regression_check.py
python3 scripts/stable_release_check.py
```
