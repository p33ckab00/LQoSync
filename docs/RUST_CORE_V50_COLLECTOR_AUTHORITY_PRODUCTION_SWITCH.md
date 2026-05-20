# Rust Core v5.0 Collector Authority Production Switch Contract

`rust/lqosync-core = 5.0.0`  
`LQoSync VERSION = 2.120.0-rc1`

## Summary

v5.0 adds the first production collector-authority switch contract.

This is **not Python removal yet**. It is the contract stage after the v4.9 production freeze gate.

```text
production freeze gate
+ production switch contract gates
+ manual confirmation
+ maintenance window
+ operator acknowledgment
+ rollback path
+ Python fallback requirement
+ side-effect checks
→ production switch contract
```

## New Rust operation

```text
build-collector-authority-production-switch-contract
```

## New API endpoint

```text
GET  /api/rust-core/collector-authority-production-switch-contract
POST /api/rust-core/collector-authority-production-switch-contract
```

## Manual confirmation token

```text
CONFIRM_COLLECTOR_AUTHORITY_PRODUCTION_SWITCH_CONTRACT
```

If Rust must build the v4.9 prerequisite internally, callers may pass:

```json
{
  "collector_authority_production_freeze_confirmation": "CONFIRM_COLLECTOR_AUTHORITY_PRODUCTION_FREEZE_GATE"
}
```

## Safety behavior

v5.0 is still non-mutating:

```text
No Rust collector production switch execution
No Python backend removal
No cleanup authority transfer
No generated file writes
No LibreQoS apply authority
Python collector fallback remains mandatory
```

The result explicitly keeps:

```text
full_rust_backend = false
production_collector_authority_switched = false
collector_authority_production_switch_executed = false
python_backend_removable = false
python_backend_required = true
rust_can_drive_cleanup = false
rust_can_drive_apply = false
rust_can_write_generated_files = false
```

## Why Python is not removed yet

Removing Python requires Rust ownership of more than collector authority:

```text
Rust API/service engine
Rust scheduler
Rust run_cycle orchestrator
Rust RouterOS live collectors
Rust circuit builder authority
Rust sync engine authority
Rust apply/journal/rollback authority
Rust config/state/audit writes
```

The WebUI/UX can remain visually as-is, but the backend route/API layer must eventually be replaced by Rust if Flask/Python is removed.

## New config defaults

```json
"rust_core": {
  "collector_authority_production_switch_contract_pilot": false,
  "allow_collector_authority_production_switch_contract": false,
  "collector_authority_production_switch_mode": "contract_only",
  "collector_authority_production_switch_require_freeze_gate": true,
  "collector_authority_production_switch_require_python_fallback": true,
  "collector_authority_production_switch_require_manual_confirmation": true,
  "collector_authority_production_switch_require_no_cleanup_apply": true,
  "collector_authority_production_switch_require_rollback_path": true,
  "collector_authority_production_switch_require_maintenance_window": true,
  "collector_authority_production_switch_require_operator_ack": true,
  "collector_authority_production_switch_max_shadow_age_seconds": 900
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
build-collector-authority-production-switch-contract
```
