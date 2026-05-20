# Rust Core v3.6 — Collector Authority Decision Manifest

LQoSync `2.106.0-rc1` adds `lqosync-core` `3.6.0`.

## Operation

```text
build-collector-authority-manifest
```

This operation builds an auditable, non-mutating decision manifest for future Rust collector authority. It consumes the source-level `evaluate-rust-collector-authority-pilot` gate and returns per-source decisions such as `python_authoritative_shadow` or `rust_pilot_ready`.

## Safety

This phase does not perform live RouterOS reads, does not switch collector authority, does not mark cleanup safe, and does not write LibreQoS files. Python collectors remain authoritative.

## API

```text
GET  /api/rust-core/collector-authority-manifest
POST /api/rust-core/collector-authority-manifest
```

Example:

```bash
curl "http://YOUR-LQOSYNC/api/rust-core/collector-authority-manifest?sources=pppoe,dhcp&parity_score=100&parity_verdict=parity_pass"
```

## Expected statuses

```text
collector_authority_manifest_shadow_only
collector_authority_manifest_partial
collector_authority_manifest_ready
blocked
```

## Why this matters

Before Rust can own any collector source, operators need a stable manifest showing exactly which source would remain Python-authoritative and which source is eligible for a Rust authority pilot. This is the bridge before collector authority dry-run integration.
