# Rust Core v1.9 Collector Bundle Parity Report

Rust Core v1.9 adds a diagnostic parity comparator for the `lqosync-in-rust` branch.

## Operation

```text
compare-collector-bundle-parity
```

The operation compares Python-authoritative ShapedDevices-compatible rows with Rust-shadow collector bundle rows. It is designed to answer one question before future collector authority migration:

> Does Rust produce the same circuits that Python would currently write?

## Inputs

The request accepts:

```json
{
  "python_rows": [],
  "rust_rows": [],
  "rust_bundle": {"result": {"normalized_rows": []}},
  "compare_fields": ["Parent Node", "MAC", "IPv4", "Download Max Mbps", "Upload Max Mbps"],
  "strict": false
}
```

`rust_rows` may be provided directly, or extracted from a previous `build-collector-circuit-bundle` response.

## Result

The response reports:

- `verdict`: `parity_pass`, `parity_warning`, or `parity_failed`
- `parity_score`
- `python_count` and `rust_count`
- missing rows in Rust
- extra rows in Rust
- field mismatch samples

## API

```text
POST /api/rust-core/collector-bundle-parity
```

This endpoint is diagnostic only. It does not write files, alter config, or enable Rust collector authority.

## Safety status

Python remains authoritative. Rust collector bundle parity is a readiness signal for the later collector/circuit migration stage.
