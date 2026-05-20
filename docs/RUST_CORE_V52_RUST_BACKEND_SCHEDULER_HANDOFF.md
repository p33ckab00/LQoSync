# Rust Core v5.2 Rust Backend Scheduler / Run Cycle Handoff Plan

`VERSION = 2.122.0-rc1`  
`lqosync-core = 5.2.0`

## Summary

v5.2 adds the next full-Rust-backend bridge after v5.1 API handoff planning.

It prepares Rust ownership of backend scheduling and `run_cycle` orchestration, while keeping the existing WebUI/UX unchanged and keeping Python authoritative.

## New operation

```text
build-rust-backend-scheduler-handoff-plan
```

## New endpoint

```text
GET  /api/rust-core/rust-backend-scheduler-handoff-plan
POST /api/rust-core/rust-backend-scheduler-handoff-plan
```

## Required confirmation token

```text
CONFIRM_RUST_BACKEND_SCHEDULER_RUN_CYCLE_HANDOFF_PLAN
```

## What it checks

```text
Rust API handoff plan is ready
scheduler manifest is ready
scheduler interval is valid
run_cycle shadow cycles are present
Python backend fallback remains enabled
no Python removal happened
no scheduler/run_cycle authority switch happened
no cleanup/apply/write side effects happened
```

## Safety behavior

v5.2 is still non-mutating:

```text
No Python backend removal
No Python scheduler replacement
No run_cycle traffic switch to Rust
No cleanup authority transfer
No generated file writes
No LibreQoS apply authority
WebUI/UX remains unchanged
Python backend fallback remains mandatory
```

## Config defaults

```json
"rust_core": {
  "rust_backend_scheduler_handoff_plan_pilot": false,
  "allow_rust_backend_scheduler_handoff_plan": false,
  "rust_backend_scheduler_handoff_mode": "plan_only",
  "rust_backend_scheduler_handoff_require_api_handoff": true,
  "rust_backend_scheduler_handoff_require_python_fallback": true,
  "rust_backend_scheduler_handoff_require_manual_confirmation": true,
  "rust_backend_scheduler_handoff_require_run_cycle_shadow": true,
  "rust_backend_scheduler_handoff_require_scheduler_parity": true,
  "rust_backend_scheduler_handoff_require_no_side_effects": true,
  "rust_backend_scheduler_handoff_max_shadow_age_seconds": 900
}
```

## Production note

This does not make LQoSync full Rust backend production yet. It is the scheduler/run_cycle handoff planning stage.

Remaining backend ownership phases include Rust run_cycle orchestrator authority, Rust config/state/audit service ownership, Rust apply/journal/rollback production authority, then final Python backend removal.
