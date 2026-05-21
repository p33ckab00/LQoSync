# Rust Core v3.9 — run_cycle Rust-Shadow Integration Report

Rust Core v3.9 adds `build-run-cycle-rust-shadow-report`, a non-mutating bridge that lets the Python `run_cycle` attach Rust-shadow collector dry-run data beside the authoritative Python collector result. On the `lqosync-in-rust` branch, this report also accepts `build-routeros-live-read-shadow-parity` evidence so live-read parity can be carried into repeated run-cycle history.

## Purpose

This phase prepares the production orchestrator for the future Rust collector authority pilot without switching authority yet. It answers:

- Is a Rust-shadow collector bundle available for this cycle?
- Is live-read shadow parity evidence available for this cycle?
- How many authoritative Python rows and Rust-shadow rows are present?
- Is parity available?
- Can Rust output drive cleanup or apply? In v3.9, always no.

## Safety model

Python remains authoritative. Run-cycle shadow reporting does not initiate live RouterOS reads by default, does not drive cleanup, does not write generated files, and does not run LibreQoS apply.

The report always returns:

```text
python_run_cycle_authoritative = true
rust_can_drive_cleanup = false
rust_can_drive_apply = false
rust_can_write_generated_files = false
full_rust_backend = false
```

## Operation

```json
{
  "op": "build-run-cycle-rust-shadow-report",
  "version": "1",
  "payload": {
    "python_rows": [],
    "live_read_shadow_parity": {},
    "collector_parity": {"parity_score": 100, "verdict": "parity_pass"}
  }
}
```

## Config flags

```json
"rust_core": {
  "run_cycle_rust_shadow_report_enabled": false,
  "run_cycle_rust_shadow_report_pilot": false,
  "run_cycle_rust_shadow_include_rows": false
}
```

The default is disabled. When enabled, the report is still diagnostic-only.

## Live-read shadow fields

When live-read shadow evidence is supplied, the report includes:

```text
live_read_shadow_ready
live_read_shadow_status
live_read_shadow_row_count
live_read_shadow_parity_verdict
live_read_shadow_error_count
```

`run_cycle_rust_shadow_ready` may be reached from either the existing dry-run
collector bundle or a passing live-read shadow parity result. In both cases,
Python run_cycle remains authoritative.

## New API

```text
GET  /api/rust-core/run-cycle-rust-shadow-report
POST /api/rust-core/run-cycle-rust-shadow-report
```

## Next phase

The next safe phase is recording repeated live-read shadow parity cycles and feeding that history into collector authority activation. Rust-shadow rows must pass across multiple cycles before any source becomes Rust-authoritative.
