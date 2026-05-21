# Singularity Rust Backend Cutover

This branch keeps the existing Flask WebUI, but the backend direction is Rust.
The migration is intentionally staged so generated LibreQoS files are not put at
risk by deleting Python collector code before the Rust live path is proven.

## Target Boundary

```text
Python Flask:
- WebUI routes, templates, login/session shell
- forms and read-only presentation
- JSON calls into lqosync-core

Rust lqosync-core:
- scheduler authority
- live RouterOS reads
- PPPoE/DHCP/Hotspot collection
- Singularity policy guardrails
- ShapedDevices.csv and network.json generation
- atomic writes, transaction journal, rollback, and LibreQoS apply
```

## Singularity Policy

Singularity is the only supported operator policy mode:

```json
{
  "policies": {
    "mode": "singularity"
  }
}
```

Legacy `conservative`, `balanced`, and `aggressive` names are compatibility
aliases only. They should normalize toward Singularity instead of becoming Rust
backend features.

The policy behavior should remain simple:

```text
- cleanup normal inactive dynamic clients after a successful source scan
- preserve static/manual rows
- preserve rows when a collector fails
- block cleanup when an enabled source returns zero rows unexpectedly
- require confirmation when a dynamic source is disabled, preserving rows until confirmed
- block mass-removal cleanup instead of asking operators to tune many knobs
```

## Migration Order

1. Keep the Python WebUI as the shell.
2. Normalize policy configuration and docs to Singularity.
3. Build Rust RouterOS read plans and validate read-result trust.
4. Build Rust shadow collector bundles from trusted RouterOS read results.
5. Implement Rust live RouterOS API reads behind pilot gates.
6. Move PPPoE, DHCP, and Hotspot row generation into Rust authority.
7. Move run-cycle orchestration into Rust.
8. Switch scheduler commands away from `scripts/run_cycle_once.py`.
9. Delete stale Python backend modules only after Rust parity passes.

## Current Shadow Bundle Bridge

The next backend migration layer is now represented by:

```text
build-routeros-shadow-collector-bundle
```

This Rust operation accepts RouterOS read results, validates them against the
planned MikroTik reads, normalizes PPPoE/DHCP/Hotspot rows through the Rust
collector bundle builder, and can compare those rows against Python output.

It is intentionally non-authoritative:

```text
- no socket connections are opened
- no credentials are emitted
- no cleanup/write/apply authority is transferred
- Python collectors remain production-authoritative
- parity output is diagnostic until live-read shadow cycles pass
```

## Python Backend Retirement Gate

Do not delete these production paths until Rust live parity is demonstrated:

```text
collectors/
builders/
rules/
parsers/
validators/
engine/run_cycle.py
scripts/run_cycle_once.py
```

After cutover, those files can be removed or converted into test fixtures and
migration notes. Until then, they are compatibility bridge code, not dead code.
