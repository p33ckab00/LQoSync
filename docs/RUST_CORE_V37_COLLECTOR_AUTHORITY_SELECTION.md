# Rust Core v3.7 Collector Authority Dry-Run Selection

LQoSync `2.107.0-rc1` / `lqosync-core 3.7.0` adds `build-collector-authority-selection`.

This phase is a bridge between the v3.6 collector authority decision manifest and future run-cycle integration.

## What it does

The Rust core now converts the collector authority manifest into a dry-run selection map:

```text
collector authority manifest
↓
per-source dry-run selection
↓
python_collector or rust_shadow_collector
```

The result is explicit about which sources may be compared through Rust-shadow collection during dry runs.

## What it does not do

This release does **not** switch production authority:

```text
No live RouterOS reads
No collector authority switch
No cleanup authority transfer
No LibreQoS apply authority
No generated file writes
```

Python collectors remain production authoritative.

## New operation

```text
build-collector-authority-selection
```

## New API endpoint

```text
GET  /api/rust-core/collector-authority-selection
POST /api/rust-core/collector-authority-selection
```

Example:

```bash
curl "http://YOUR-LQOSYNC/api/rust-core/collector-authority-selection?sources=pppoe,dhcp&parity_score=100&parity_verdict=parity_pass"
```

## New config defaults

```json
"rust_core": {
  "collector_authority_dry_run_selection_pilot": false,
  "allow_collector_authority_dry_run_selection": false
}
```

Even when enabled, the output is dry-run only. Production authority remains Python.

## Status values

```text
collector_authority_dry_run_selection_python_only
collector_authority_dry_run_selection_ready
collector_authority_dry_run_selection_partial
blocked
```

## Next phase

The next phase can wire this selector into Python `run_cycle.py` as a dry-run comparison signal, without allowing Rust collector output to drive cleanup or apply decisions yet.
