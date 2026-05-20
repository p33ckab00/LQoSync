# Rust Core v7.5.8 Full Rust Authority Lockdown

This release closes the remaining no-breakage gap from v7.5.7.

v7.5.7 added Rust-owned apply transactions, but existing installs could still remain in a compatibility posture unless the promotion script was run. v7.5.8 makes full Rust authority explicit, fail-closed, and visible in runtime output.

## What changes

When full authority is enabled, Python may still provide the Flask WebUI and scheduler shell, but it is no longer allowed to perform production mutations as a silent fallback.

Rust owns:

- validation enforcement;
- sync-plan blocker enforcement;
- authority readiness checks;
- apply manifest execution;
- atomic `ShapedDevices.csv` writes;
- atomic `network.json` writes;
- transaction journal append;
- external `LibreQoS.py --updateonly` execution.

Python remains:

- WebUI/API compatibility shell;
- scheduler service entrypoint;
- RouterOS transport compatibility path until live Rust RouterOS transport is separately certified.

Collector output is now treated as `rust_validate_all`: even when Python transport gathers RouterOS data, the collected output must pass Rust authority validation before the mutation path can continue.

## New authority lock keys

```json
{
  "full_rust_backend_authority": true,
  "python_mutation_fallback": false,
  "fail_closed_without_rust_authority": true,
  "require_rust_authoritative_transaction": true,
  "collector_output_authority": "rust_validate_all",
  "collector_authority_mode": "rust_validated_python_transport"
}
```

## Runtime fail-closed behavior

If full Rust authority is enabled and Rust does not execute the required mutation, the cycle fails closed:

- `rust_full_authority_missing_file_write_flags`
- `rust_full_authority_missing_apply_flag`
- `rust_full_authority_file_write_not_executed`
- `rust_full_authority_libreqos_apply_not_executed`

This prevents accidental Python writes or Python `LibreQoS.py` apply when the operator has declared Rust authoritative.

## Install / promote

Fresh production-safe install:

```bash
sudo bash install-rust-full-authoritative-safe.sh
```

Existing install promotion:

```bash
cd /opt/LQoSync
sudo bash scripts/promote-rust-full-authoritative-safe.sh
```

The promotion script backs up `/opt/libreqos/src/config.json`, requires `lqosync-core self-test`, and applies the full authority lock keys.

## Verify package wiring

```bash
bash scripts/verify-rust-full-authority-lockdown.sh
```

## Boundary

This is a full Rust authority lock for the production mutation path. It is not a removal of Python WebUI/scheduler code. Python fallback remains available only when full Rust authority lock is disabled.
