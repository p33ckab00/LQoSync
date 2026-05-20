# Rust Core v7.6.0 — Rust Authority Supervisor

v7.6.0 adds an operator-supervised production gate around full Rust authority. It does not remove the Python WebUI or scheduler shell. Instead, it prevents the full Rust mutation path from running blindly after promotion.

## What changed

- Added `scripts/rust-full-authority-preflight.sh`.
- Added `scripts/rust-full-authority-recovery-bundle.sh`.
- Added `scripts/verify-rust-authority-supervisor.sh`.
- Promotion to full Rust authority now creates a recovery bundle before config promotion.
- Promotion now writes a fresh Rust authority preflight stamp after the Rust self-test and config patch pass.
- Runtime can fail closed with `rust_authority_preflight_required_failed` when a promoted full-authority config requires a fresh preflight stamp.

## New config keys

```json
{
  "rust_core": {
    "full_rust_authority_supervisor_enabled": true,
    "require_rust_authority_preflight": true,
    "fail_closed_on_authority_preflight_failure": true,
    "rust_authority_preflight_stamp": "/opt/LQoSync/state/rust_authority_preflight.json",
    "rust_authority_preflight_max_age_seconds": 900,
    "require_rust_authority_recovery_bundle": true,
    "rust_authority_recovery_bundle_dir": "/opt/LQoSync/state/rust_authority_recovery",
    "rust_authority_recovery_bundle_before_promotion": true,
    "rust_authority_supervisor_mode": "operator_supervised"
  }
}
```

`require_rust_authority_preflight` is enabled by the promotion script after it writes a passing stamp. This keeps package upgrades safe while ensuring promoted production authority has a recent proof of Rust readiness.

## Operator flow

```bash
cd /opt/LQoSync
bash scripts/build-rust-core.sh
sudo bash scripts/install-rust-core.sh
sudo bash scripts/install-rust-core-daemon.sh
sudo bash scripts/promote-rust-full-authoritative-safe.sh
bash scripts/verify-rust-authority-supervisor.sh
```

The promotion script runs:

```bash
scripts/rust-full-authority-recovery-bundle.sh
scripts/rust-full-authority-preflight.sh --write-stamp
```

## Recovery bundle

The recovery bundle captures:

- `config.json`
- `ShapedDevices.csv`
- `network.json`
- runtime and policy state where configured
- service status snapshots
- Rust self-test output
- file SHA256 manifest

It is stored under:

```text
/opt/LQoSync/state/rust_authority_recovery/<timestamp>
```

## Fail-closed runtime status

If full Rust authority is promoted and the preflight stamp is missing, stale, or failed, the runtime blocks production mutation with:

```text
rust_authority_preflight_required_failed
```

This is intentional. Run:

```bash
sudo CONFIG_PATH=/opt/libreqos/src/config.json \
  LQOSYNC_INSTALL_DIR=/opt/LQoSync \
  bash /opt/LQoSync/scripts/rust-full-authority-preflight.sh --write-stamp
```

Then rerun Dry Run or restart the service after operator review.

## No-breakage boundary

Python still owns:

- Flask WebUI
- scheduler shell
- RouterOS transport compatibility

Rust owns the production mutation authority once full-authority promotion succeeds:

- validation
- sync-plan blocker enforcement
- file writes
- transaction journal
- LibreQoS apply
- authority preflight gate
