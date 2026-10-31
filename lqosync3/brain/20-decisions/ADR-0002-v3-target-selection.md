---
type: decision
created: 2026-10-31
tags: [adr, libreqos-v3]
status: proposed
supersedes:
related: ["[[ADR-0001-one-model-many-writers]]", "[[libreqos-v3-ingress-facts]]"]
---

# ADR-0002 — Automatic target selection from the LibreQoS version and the v3 topology source

## Context

LibreQoS 3.x selects its topology input with `[topology] source` in `/etc/libreqos/lqos.conf`.
If the source is `shaped_devices`, `lqosd` reads the pair. If the source is `libreqos_network`, `lqosd` reads `libreqos_network.json`. If the source is a built-in integration, LibreQoS owns the topology.
If the tool writes the wrong format, `lqosd` ignores the file and the subscribers do not get updates.

## Decision

With `target = "auto"`, the tool selects the target at the start of each cycle:

1. LibreQoS 2.0 to 2.2: `v2_pair`.
2. LibreQoS 3.x with `source = "libreqos_network"`: `v3_network`.
3. LibreQoS 3.x with `source = "shaped_devices"`: `v3_pair`.
4. Any other case: stop the cycle and report an error.

The tool does not write `topology_import.json`. That file is internal to LibreQoS.

## Alternatives

1. A fixed target in TOML only. This is simple, but an upgrade without a config change writes the wrong format.
2. Write all formats in each cycle. This writes files that no reader uses and can confuse the operator.

## Consequences

- The tool reads `/etc/libreqos/lqos.conf` on v3. A parse error stops the cycle.
- A fixed target in TOML stays possible. It overrides the detector.
- `network.mode = "no_parent"` cannot use `v3_network`, because v3 requires a parent for each circuit.

## Status
proposed. Accept it after a test on a v3 test shaper.
