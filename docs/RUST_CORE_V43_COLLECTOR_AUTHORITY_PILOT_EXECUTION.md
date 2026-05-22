# Rust Core v4.3 Collector Authority Pilot Execution Contract

Adds `build-collector-authority-pilot-execution-contract`, a non-mutating bridge after the v4.2 switch rehearsal.

## Purpose

This phase proves that LQoSync can describe a future Rust collector authority pilot execution without actually switching production collector authority away from Python. The contract now requires the v4.2 switch rehearsal to have selected Rust shadow rows for diagnostics-only observation before the pilot can become observation-ready.

## Safety

The operation does not open RouterOS sockets, does not perform live reads, does not transfer cleanup authority, does not write generated files, and does not apply LibreQoS. Python collector fallback remains mandatory.

The readiness contract carries these row-authority fields forward:

- `production_row_authority="python_collector"`
- `cleanup_row_authority="python_collector"`
- `diagnostic_row_authority="rust_shadow_diagnostics"` only when the switch rehearsal selected Rust rows for diagnostics
- `diagnostic_selection_ready`
- `rust_rows_may_feed_pilot_observation`

If diagnostic selection is required and not ready, the result remains `collector_authority_pilot_execution_waiting_for_gates`.

## Confirmation

The readiness contract requires the manual token:

```text
CONFIRM_COLLECTOR_AUTHORITY_PILOT_EXECUTION
```

Even with the token, this release only returns `collector_authority_pilot_execution_contract_ready`; it does not execute production authority transfer.

## Config

```json
{
  "rust_core": {
    "collector_authority_pilot_execution_require_diagnostic_selection": true
  }
}
```

This default keeps pilot observation fail-closed until the switch rehearsal proves Rust rows are diagnostics-only and Python remains production/cleanup authority.

## Operation

```text
build-collector-authority-pilot-execution-contract
```

## API

```text
GET  /api/rust-core/collector-authority-pilot-execution-contract
POST /api/rust-core/collector-authority-pilot-execution-contract
```
