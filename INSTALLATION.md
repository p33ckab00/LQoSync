# LQoSync Installation Guide

LQoSync is installed as a local appliance web app.

Canonical paths:

```text
Application: /opt/LQoSync
LibreQoS src: /opt/libreqos/src
Rust socket: /run/lqosync-core.sock
Config: /opt/libreqos/src/config.json
```

## Recommended live install

```bash
cd /opt/LQoSync
sudo bash install-rust-stable-safe.sh
```

This preserves existing LibreQoS files, builds/installs Rust, installs the Rust daemon, promotes full Rust authority, and leaves the Flask WebUI as operator shell.

## ZIP install/update

```bash
mkdir -p /tmp/lqosync-v810
unzip LQoSync_runtime_canonical_FULL_rust_core_v810_rust_scheduler_authority_stable.zip -d /tmp/lqosync-v810
cd /tmp/lqosync-v810
sudo bash install-from-zip.sh
```

For update:

```bash
sudo bash update-from-zip.sh
```

## GitHub update

```bash
cd /opt/LQoSync
git fetch origin lqosync-in-rust
git reset --hard origin/lqosync-in-rust
sudo bash install-rust-stable-safe.sh
```

## Docker

Docker is supported for lab/controlled use. Bare metal/systemd is preferred for live LibreQoS machines.

```bash
sudo bash install-docker.sh
```

## Verify

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

## v8.2.5 one-line GitHub operations

Preferred fresh install from GitHub branch `lqosync-in-rust`:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- install
```

Preferred update:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- update
```

Check current installation:

```bash
sudo /opt/LQoSync/lqosyncctl.sh check
```

Verify current installation:

```bash
sudo /opt/LQoSync/lqosyncctl.sh verify
```
