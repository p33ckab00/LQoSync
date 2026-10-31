---
type: decision
created: 2026-10-31
tags: [adr, architecture]
status: accepted
supersedes:
related: ["[[ADR-0002-v3-target-selection]]", "[[2026-10-31-project-setup-and-v3-simulation]]"]
---

# ADR-0001 — One circuit model with one writer for each LibreQoS target

## Context

`updatecsv.py` writes `ShapedDevices.csv` and `network.json` for LibreQoS 2.x.
The operator plans a migration from LibreQoS 2.2 to LibreQoS 3.x.
LibreQoS 3.x reads the old pair, and it also reads a new file, `libreqos_network.json`.
The new tool must operate on 2.2 now and on 3.x after the upgrade.
The operator wants the new tool in Rust, with the settings in TOML, and with `mikrotik-rs` for the RouterOS API.

## Decision

Make one Rust crate with one internal circuit model.
Add one writer for each target: `v2_pair`, `v3_pair`, and `v3_network`.
The collector, the speed rules, the topology builder, and the validator are the same for all targets.
Only the writer and the reload step change with the target.

## Alternatives

1. Write only the pair for all versions. This is the simplest option, and it keeps the device comments. It does not use the v3 strict reject gate, the stable node IDs, or the VLAN and MPLS match modes.
2. Write only `libreqos_network.json`. LibreQoS 2.2 cannot read it. A converter for 2.2 is necessary, and that is the same work as this decision.

## Consequences

- The upgrade to v3 needs no code change. A TOML value or the auto detector selects the new writer.
- The schema of `libreqos_network.json` is beta and has no public documentation. A schema change in LibreQoS can break `v3_network`. `v3_pair` stays available as a fallback.
- The tool must do checks that the v3 compiler does not do: duplicate IP and duplicate node name.
- Performance comes from parallel router reads, open API sessions, and no write or reload when the output hash does not change.

## Status
accepted on 2026-10-31 (operator selected option A in chat)
