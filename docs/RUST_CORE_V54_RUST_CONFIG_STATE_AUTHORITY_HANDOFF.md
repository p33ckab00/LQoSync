# Rust Core v5.4 Rust Config/State Authority Handoff Contract

LQoSync `2.124.0-rc1` / `lqosync-core 5.4.0` adds `build-rust-config-state-authority-handoff-contract`.

## Status

This phase is part of the full-Rust-backend track, but it is not final production and does not remove Python.

```text
Current phase: Rust config/state authority handoff contract
Full Rust backend production: not yet
Python removal: not yet
WebUI/UX: unchanged
```

## Purpose

v5.4 prepares Rust to own config/state authority in a later production phase. It checks that config/state shadow verification, atomic writer shadow checks, transaction journal shadow checks, audit shadow checks, and rollback-manifest shadow checks are ready.

## New operation

```text
build-rust-config-state-authority-handoff-contract
```

## Required confirmation token

```text
CONFIRM_RUST_CONFIG_STATE_AUTHORITY_HANDOFF_CONTRACT
```

## Safety behavior

v5.4 remains non-mutating:

```text
No Python backend removal
No Python config/state authority switch
No Rust config/state writes
No audit/journal writes
No cleanup authority transfer
No generated file writes
No LibreQoS apply authority
WebUI/UX remains unchanged
Python backend fallback remains mandatory
```

## New API endpoint

```text
GET  /api/rust-core/rust-config-state-authority-handoff-contract
POST /api/rust-core/rust-config-state-authority-handoff-contract
```

## New config defaults

```json
"rust_core": {
  "rust_config_state_authority_handoff_contract_pilot": false,
  "allow_rust_config_state_authority_handoff_contract": false,
  "rust_config_state_authority_handoff_mode": "contract_only",
  "rust_config_state_authority_handoff_require_run_cycle_orchestrator": true,
  "rust_config_state_authority_handoff_require_python_fallback": true,
  "rust_config_state_authority_handoff_require_manual_confirmation": true,
  "rust_config_state_authority_handoff_require_config_state_shadow": true,
  "rust_config_state_authority_handoff_require_atomic_writer_shadow": true,
  "rust_config_state_authority_handoff_require_transaction_journal_shadow": true,
  "rust_config_state_authority_handoff_require_audit_shadow": true,
  "rust_config_state_authority_handoff_require_rollback_shadow": true,
  "rust_config_state_authority_handoff_require_no_side_effects": true,
  "rust_config_state_authority_handoff_max_shadow_age_seconds": 900
}
```

## Validation

```bash
bash scripts/repair-script-permissions.sh
bash scripts/build-rust-core.sh
sudo bash scripts/install-rust-core.sh
sudo bash scripts/install-rust-core-daemon.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
```

Expected operation:

```text
build-rust-config-state-authority-handoff-contract
```
