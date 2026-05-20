# Rust Core v5.5 Rust Live Collector Authority Handoff Contract

LQoSync `2.125.0-rc1` / `lqosync-core 5.5.0` adds `build-rust-live-collector-authority-handoff-contract`.

## Phase

```text
Current phase: Rust live collector authority handoff contract
Full Rust backend production: not yet
Python removal: not yet
WebUI/UX: unchanged
```

This phase moves the full-Rust-backend track from config/state authority planning toward live RouterOS collector authority planning. It validates live collector shadow evidence, RouterOS live adapter shadow evidence, collector parity, prerequisite config/state handoff, and Python fallback.

## New operation

```text
build-rust-live-collector-authority-handoff-contract
```

## Required confirmation token

```text
CONFIRM_RUST_LIVE_COLLECTOR_AUTHORITY_HANDOFF_CONTRACT
```

## Safety behavior

v5.5 remains non-mutating:

```text
No Python backend removal
No Python live collector replacement
No Rust live collector authority switch
No RouterOS live writes
No cleanup authority transfer
No generated file writes
No LibreQoS apply authority
WebUI/UX remains unchanged
Python backend fallback remains mandatory
```

## API endpoint

```text
GET  /api/rust-core/rust-live-collector-authority-handoff-contract
POST /api/rust-core/rust-live-collector-authority-handoff-contract
```

## Config defaults

```json
"rust_core": {
  "rust_live_collector_authority_handoff_contract_pilot": false,
  "allow_rust_live_collector_authority_handoff_contract": false,
  "rust_live_collector_authority_handoff_mode": "contract_only",
  "rust_live_collector_authority_handoff_require_config_state_authority": true,
  "rust_live_collector_authority_handoff_require_python_fallback": true,
  "rust_live_collector_authority_handoff_require_manual_confirmation": true,
  "rust_live_collector_authority_handoff_require_live_collector_shadow": true,
  "rust_live_collector_authority_handoff_require_routeros_adapter_shadow": true,
  "rust_live_collector_authority_handoff_require_collector_parity": true,
  "rust_live_collector_authority_handoff_require_no_side_effects": true,
  "rust_live_collector_authority_handoff_max_shadow_age_seconds": 900
}
```

## Server validation

```bash
bash scripts/repair-script-permissions.sh
bash scripts/build-rust-core.sh
sudo bash scripts/install-rust-core.sh
sudo bash scripts/install-rust-core-daemon.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
```

Expected operation:

```text
build-rust-live-collector-authority-handoff-contract
```
