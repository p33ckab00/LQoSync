# Rust Core v6.4 Rust Backend Production Enablement Contract

`rust/lqosync-core = 6.4.0`  
`LQoSync VERSION = 2.134.0-rc1`

## Summary

v6.4 adds the Rust backend production enablement contract.

This is the bridge after v6.3 Python backend retirement planning. It can mark the system as a full Rust backend production enablement candidate, but it still does not remove Python, disable Flask routes, switch API traffic, or enable Rust production authority.

## New operation

```text
build-rust-backend-production-enablement-contract
```

## New endpoint

```text
GET  /api/rust-core/rust-backend-production-enablement-contract
POST /api/rust-core/rust-backend-production-enablement-contract
```

## Required confirmation token

```text
CONFIRM_RUST_BACKEND_PRODUCTION_ENABLEMENT_CONTRACT
```

## Phase position

```text
v6.0 full Rust backend production-readiness contract
v6.1 full Rust backend cutover plan
v6.2 full Rust backend cutover execution contract
v6.3 Python backend retirement plan
v6.4 Rust backend production enablement contract
```

## Safety behavior

v6.4 remains non-mutating:

```text
No Python backend removal
No Flask route disable
No API traffic switch to Rust
No Rust production service authority enablement
No generated file writes
No LibreQoS apply authority
WebUI/UX remains unchanged
Python backend fallback remains mandatory
```

It can mark:

```text
full_rust_backend_candidate = true
python_backend_retirement_candidate = true
```

But it still keeps:

```text
full_rust_backend = false
full_rust_backend_production_enabled = false
rust_backend_production_enablement_allowed = false
python_backend_removed = false
python_backend_removable = false
python_removal_allowed = false
flask_routes_disabled = false
api_traffic_switched_to_rust = false
rust_service_runtime_authoritative = false
```

## New config defaults

```json
"rust_core": {
  "rust_backend_production_enablement_contract_pilot": false,
  "allow_rust_backend_production_enablement_contract": false,
  "rust_backend_production_enablement_mode": "contract_only",
  "rust_backend_production_enablement_require_python_retirement_plan": true,
  "rust_backend_production_enablement_require_python_fallback": true,
  "rust_backend_production_enablement_require_manual_confirmation": true,
  "rust_backend_production_enablement_require_webui_unchanged": true,
  "rust_backend_production_enablement_require_rollback_path": true,
  "rust_backend_production_enablement_require_operator_ack": true,
  "rust_backend_production_enablement_require_no_side_effects": true,
  "rust_backend_production_enablement_max_shadow_age_seconds": 900
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

Expected new operation:

```text
build-rust-backend-production-enablement-contract
```
