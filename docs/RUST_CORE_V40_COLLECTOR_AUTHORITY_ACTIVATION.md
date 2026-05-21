# Rust Core v4.0 Collector Authority Activation Plan

LQoSync `2.110.0-rc1` / `lqosync-core 4.0.0` adds the first **collector authority activation plan** after the v3.9 Python `run_cycle` Rust-shadow integration.

## Operation

```text
build-collector-authority-activation-plan
```

This operation evaluates whether the system is ready to enter a future Rust collector authority pilot.

It combines:

```text
run_cycle Rust-shadow report
+ collector authority dry-run bundle
+ parity result
+ successful shadow-cycle count or supplied live-read shadow history
+ explicit activation gates
+ Python fallback requirement
```

## Safety model

This release is still non-mutating and does **not** switch production authority.

```text
Python collectors remain authoritative
Python run_cycle remains authoritative
Rust cannot drive cleanup
Rust cannot drive LibreQoS apply
Rust cannot write generated files from collector output
Rust does not perform live RouterOS reads
```

The operation always reports:

```json
{
  "full_rust_backend": false,
  "production_collector_authority_switched": false,
  "collector_authority_switch_supported": false,
  "python_collector_fallback_required": true,
  "rust_can_drive_cleanup": false,
  "rust_can_drive_apply": false,
  "rust_can_write_generated_files": false
}
```

## New config flags

```json
{
  "rust_core": {
    "collector_authority_activation_pilot": false,
    "allow_collector_authority_activation": false,
    "collector_authority_activation_mode": "shadow_only",
    "collector_authority_require_python_fallback": true,
    "collector_authority_require_run_cycle_shadow": true,
    "collector_authority_min_shadow_cycles": 3,
    "collector_authority_successful_shadow_cycles": 0
  }
}
```

`successful_shadow_cycles` may be supplied directly, read from config, or
derived from shadow history arrays:

```text
run_cycle_shadow_history
live_read_shadow_history
shadow_history
successful_shadow_history
```

An entry counts as successful when it is a ready run-cycle shadow report with
`rust_shadow_ready=true` or `live_read_shadow_ready=true` and parity passed.
This lets repeated live-read shadow parity evidence satisfy the activation
cycle gate without transferring production collector authority.

## Status values

```text
collector_authority_activation_shadow_only
collector_authority_activation_waiting_for_gates_or_cycles
collector_authority_activation_ready_for_pilot
blocked
```

## API endpoint

```text
GET  /api/rust-core/collector-authority-activation-plan
POST /api/rust-core/collector-authority-activation-plan
```

Example:

```bash
curl "http://YOUR-LQOSYNC/api/rust-core/collector-authority-activation-plan?sources=pppoe&parity_score=100&parity_verdict=parity_pass&successful_shadow_cycles=3"
```

## Why this phase exists

The previous phase proved that Python `run_cycle` can carry a Rust-shadow report. v4.0 turns that diagnostic report and repeated shadow history into an explicit activation plan so operators can see whether Rust collector authority is eligible for a pilot before any live switch is allowed.

## Not full Rust backend yet

This phase is still part of the bridge to a full Rust backend. Remaining stages include:

```text
v4.1 Collector authority pilot runtime decision
v4.2 Rust collector source authority pilot
v4.3 Rust circuit builder authority
v4.4 Rust sync engine authority
v5.0 Full Rust backend core production
```
