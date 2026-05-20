# Rust Core v5.9 Rust Backend Service Runtime Handoff Contract

`rust/lqosync-core = 5.9.0`  
`LQoSync VERSION = 2.129.0-rc1`

## Summary

v5.9 adds a non-mutating Rust backend service/API runtime handoff contract.

This phase sits after v5.8 apply/journal/rollback authority handoff and prepares Rust to eventually own the outer backend service layer while keeping the existing WebUI/UX unchanged.

```text
apply/journal/rollback authority handoff
+ API route parity
+ WebUI static asset compatibility
+ Rust API shadow response parity
+ Rust service supervision/socket/healthcheck shadow verification
+ Python backend fallback requirement
+ manual confirmation
+ side-effect checks
↓
Rust backend service runtime handoff contract
```

## New operation

```text
build-rust-backend-service-runtime-handoff-contract
```

## New endpoint

```text
GET  /api/rust-core/rust-backend-service-runtime-handoff-contract
POST /api/rust-core/rust-backend-service-runtime-handoff-contract
```

## Required confirmation

```text
CONFIRM_RUST_BACKEND_SERVICE_RUNTIME_HANDOFF_CONTRACT
```

## Important production note

v5.9 still does not remove Python and does not switch WebUI/API traffic to Rust.

This is the service-runtime contract phase before a later full-backend production readiness / cutover phase.

## Safety behavior

```text
No Python backend removal
No Flask route disable
No API traffic switch to Rust
No Rust service runtime authority switch
No cleanup authority transfer
No generated file writes
No LibreQoS apply authority
WebUI/UX remains unchanged
Python backend fallback remains mandatory
```

## Config defaults

```json
"rust_core": {
  "rust_backend_service_runtime_handoff_contract_pilot": false,
  "allow_rust_backend_service_runtime_handoff_contract": false,
  "rust_backend_service_runtime_handoff_mode": "contract_only",
  "rust_backend_service_runtime_handoff_require_apply_journal_rollback_authority": true,
  "rust_backend_service_runtime_handoff_require_python_fallback": true,
  "rust_backend_service_runtime_handoff_require_manual_confirmation": true,
  "rust_backend_service_runtime_handoff_require_route_parity": true,
  "rust_backend_service_runtime_handoff_require_static_assets": true,
  "rust_backend_service_runtime_handoff_require_service_supervision": true,
  "rust_backend_service_runtime_handoff_require_api_shadow": true,
  "rust_backend_service_runtime_handoff_require_no_side_effects": true,
  "rust_backend_service_runtime_handoff_max_shadow_age_seconds": 900
}
```

## Expected validation

```bash
bash scripts/repair-script-permissions.sh
bash scripts/build-rust-core.sh
sudo bash scripts/install-rust-core.sh
sudo bash scripts/install-rust-core-daemon.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
```

Expected new operation:

```text
build-rust-backend-service-runtime-handoff-contract
```
