# v7.6.1 Rust Authority Watchdog

This update adds a non-mutating runtime watchdog for promoted full Rust authority deployments.

## Purpose

v7.6.0 added the Rust Authority Supervisor and preflight stamp. v7.6.1 adds a second operational guard that checks the evidence needed for safe Rust-owned production mutation before the Python scheduler shell allows the cycle to proceed.

The watchdog verifies:

- full Rust authority flags are enabled;
- Python mutation fallback is disabled;
- the preflight stamp is present, passing, and fresh;
- the recovery bundle root contains a latest bundle with `MANIFEST.json`;
- the transaction journal parent path exists and is writable;
- transaction journal authority flags are consistent.

## Runtime fail-closed status

When enabled and failing, the run cycle stops with:

```text
rust_authority_watchdog_required_failed
```

This is separate from the preflight failure status:

```text
rust_authority_preflight_required_failed
```

## Config keys

```json
{
  "rust_core": {
    "rust_authority_watchdog_enabled": true,
    "fail_closed_on_authority_watchdog_failure": true,
    "rust_authority_watchdog_require_fresh_preflight": true,
    "rust_authority_watchdog_max_preflight_age_seconds": 900,
    "rust_authority_watchdog_require_recovery_bundle": true,
    "rust_authority_watchdog_require_transaction_journal_path": true
  }
}
```

The package default keeps `rust_authority_watchdog_enabled=false` so upgrades do not break existing installs. The full authority promotion script enables it after creating the recovery bundle and writing a fresh preflight stamp.

## Operator commands

Manual watchdog check:

```bash
cd /opt/LQoSync
CONFIG_PATH=/opt/libreqos/src/config.json bash scripts/rust-authority-watchdog.sh
```

Full promotion flow:

```bash
cd /opt/LQoSync
sudo bash scripts/promote-rust-full-authoritative-safe.sh
```

The promotion script now runs:

```text
rust-full-authority-recovery-bundle.sh
rust-full-authority-preflight.sh --write-stamp
rust-authority-watchdog.sh
```

## No-breakage rule

The watchdog does not replace Python WebUI/scheduler shell behavior. It only blocks mutation when full Rust authority has been explicitly promoted and the required safety evidence is missing or stale.
