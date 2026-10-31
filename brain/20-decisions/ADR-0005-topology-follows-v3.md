---
type: decision
created: 2026-10-31
tags: [adr, topology, libreqos-v3]
status: accepted
supersedes:
related: ["[[ADR-0001-one-model-many-writers]]", "[[ADR-0004-toml-policy-python-defaults]]", "[[libreqos-v3-ingress-facts]]", "[[v3-node-handling]]"]
---

# ADR-0005 — The topology output follows the LibreQoS v3 node rules

## Context

The operator wants the nodes and `network.json` to follow the LibreQoS v3 rules.
An offline test with the v3 3.0.7 compiler and a read of the 2.2 and v3 files gave these facts. Refer to [[v3-node-handling]].

- v3 and 2.2 read an optional `id` in each `network.json` node. Without it, v3 makes an ID from the node name.
- v3 and 2.2 bind a circuit to a node by `Parent Node ID` first, then by `Parent Node`.
- The 2.2 example CSV has 14 columns (adds `Parent Node ID`). The v3 example CSV has 15 columns (adds `Parent Node ID` and `Anchor Node ID`).
- v3 and 2.2 `lqosd` reject a circuit with max rate below 0.2 Mbps or min rate below 0.1 Mbps.
- v3 rejects two nodes with the same name.
- In `flat_promotion`, `updatecsv.py` makes one `client` node for each circuit, but it leaves the circuit Parent Node empty. Thus no circuit is in its node.

## Decision

1. Each node that the tool makes gets a stable `id`. Format: `lq3-` and 16 hex characters of FNV-1a 64 of `<kind>|<router>|<key>`. A `client` node of `flat_promotion` uses `lq3-c-<Circuit ID>`.
2. Each circuit row gets `Parent Node ID` = the `id` of its parent node.
3. `v2_pair` writes 14 columns, as the 2.2 example. `v3_pair` writes 15 columns, as the v3 example. `Anchor Node ID` stays empty.
4. The max rate of a circuit is at least `max_floor_mbps` (default 0.2). `safe_min_mbps` must be 0.1 or more.
5. In `flat_promotion`, the Parent Node of each circuit is its own `client` node.
6. The parity test with `updatecsv.py` compares the 13 legacy columns and `network.json` without `id`. The core stays pure Rust. Python is only a test tool.

## Alternatives

1. Keep the `updatecsv.py` output byte for byte. Then v3 makes node IDs from names, and a rename changes the ID. `flat_promotion` stays without effect.
2. Write only `libreqos_network.json` on v3. That is the `v3_network` target. It stays available. The pair targets must also follow the v3 rules.

## Consequences

- A `shadow` compare with the Python service must compare the legacy projection, not the bytes.
- The 2.2 `lqosd` must accept the `id` key and 14 columns on the production shaper. Check this in `shadow` mode before the cutover.
- Open for a v3 test shaper: the effect of `compile_mode` (`flat`, `ap_only`, `ap_site`) on a manual tree, and the runtime result of an empty Parent Node (`no_parent`).

## Status
accepted on 2026-10-31 (operator direction in chat: follow the v3 node rules)
