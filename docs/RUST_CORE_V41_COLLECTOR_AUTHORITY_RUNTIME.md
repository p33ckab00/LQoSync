# Rust Core v4.1 Collector Authority Runtime Contract

Version: `lqosync-core 4.1.0` / `LQoSync 2.111.0-rc1`

This phase adds `build-collector-authority-runtime-contract`. It consumes the v4.0 collector authority activation plan and produces a redacted, non-mutating runtime contract for a future Rust collector authority pilot. The contract now carries the activation plan's dry-run/live-read shadow provenance so operators can see why Rust is or is not ready without granting Rust production authority.

## Safety

- Python collectors remain production-authoritative.
- Rust does not switch collector authority.
- Rust cannot drive cleanup or apply from this contract.
- Rust cannot write generated files from this contract.
- Python collector fallback remains mandatory.
- No live RouterOS reads are executed by this operation.
- Live-read evidence is consumed only from supplied shadow history or activation-plan output.

## New operation

```text
build-collector-authority-runtime-contract
```

The operation returns `collector_authority_runtime_contract_ready` only when:

1. the v4.0 activation plan is ready,
2. runtime contract gates are explicitly enabled,
3. Python fallback remains enabled, and
4. Rust-shadow state is still fresh enough for a pilot contract.

Even when ready, the result remains `runtime_contract_only=true` and `production_collector_authority_switched=false`.

Runtime evidence fields include:

- `rust_shadow_ready`
- `dry_run_shadow_ready`
- `live_read_shadow_ready`
- `runtime_evidence_source`
- `shadow_history_count`
- `shadow_history_successful_count`
- `configured_successful_shadow_cycles`
- `successful_shadow_cycles`
- `required_shadow_cycles`
- `shadow_cycles_ok`
- `parity_verdict`
- `live_read_shadow_parity_verdict`
- `python_row_count`
- `rust_row_count`
- `live_read_shadow_row_count`

## Config defaults

```json
{
  "rust_core": {
    "collector_authority_runtime_pilot": false,
    "allow_collector_authority_runtime_contract": false,
    "collector_authority_runtime_mode": "contract_only",
    "collector_authority_runtime_require_activation_plan": true,
    "collector_authority_runtime_require_python_fallback": true,
    "collector_authority_runtime_max_shadow_age_seconds": 900,
    "collector_authority_shadow_age_seconds": 0
  }
}
```

## API

```text
GET  /api/rust-core/collector-authority-runtime-contract
POST /api/rust-core/collector-authority-runtime-contract
```

This is a bridge toward a later controlled handoff, not a production Rust collector switch. The Python fallback mirrors the same evidence fields when the Rust binary is unavailable, but it still reports `full_rust_backend=false`.
