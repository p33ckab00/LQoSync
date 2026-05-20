# Rust Core v4.2 Collector Authority Switch Rehearsal

Version: `lqosync-core 4.2.0` / `LQoSync 2.112.0-rc1`

This phase adds `build-collector-authority-switch-rehearsal`. It consumes the v4.1 collector authority runtime contract and builds a non-mutating switch rehearsal for a future Rust collector authority pilot.

## Safety

- Python collectors remain production-authoritative.
- Rust does not switch collector authority.
- Rust cannot drive cleanup or apply from this rehearsal.
- Rust cannot write generated files from this rehearsal.
- Python collector fallback remains mandatory.
- No live RouterOS reads are executed by this operation.

## New operation

```text
build-collector-authority-switch-rehearsal
```

The operation returns `collector_authority_switch_rehearsal_ready` only when:

1. the v4.1 runtime contract is ready,
2. switch rehearsal gates are explicitly enabled,
3. Python fallback remains enabled, and
4. the manual confirmation token is present.

Even when ready, the result remains `switch_rehearsal_only=true`, `collector_authority_switch_executed=false`, and `production_collector_authority_switched=false`.

## Manual confirmation

```text
CONFIRM_COLLECTOR_AUTHORITY_REHEARSAL
```

The confirmation only permits the rehearsal contract to become ready. It does not switch authority.

## Config defaults

```json
{
  "rust_core": {
    "collector_authority_switch_rehearsal_pilot": false,
    "allow_collector_authority_switch_rehearsal": false,
    "collector_authority_switch_mode": "rehearsal_only",
    "collector_authority_switch_require_runtime_contract": true,
    "collector_authority_switch_require_python_fallback": true,
    "collector_authority_switch_require_manual_confirmation": true
  }
}
```

## API

```text
GET  /api/rust-core/collector-authority-switch-rehearsal
POST /api/rust-core/collector-authority-switch-rehearsal
```

This API is diagnostic only in v4.2.
