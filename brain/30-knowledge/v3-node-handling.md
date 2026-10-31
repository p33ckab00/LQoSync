---
type: knowledge
created: 2026-10-31
tags: [libreqos-v3, topology, evidence]
status: draft
related: ["[[ADR-0005-topology-follows-v3]]", "[[libreqos-v3-ingress-facts]]", "[[v3-offline-simulation-method]]"]
---

# How LibreQoS v3 handles nodes

Evidence from 2026-10-31: the v3 3.0.7 binding `write_compiled_topology_from_legacy_artifacts`, the `lqosd` strings of 2.2 and 3.0.7, and the 2.2 `LibreQoS.py`.
The test used synthetic pairs from `updatecsv.py` for all six network modes. All socket connections were blocked. No file was written outside the scratch directory.

## Offline compile results

| Input | Result |
|-------|--------|
| All six modes × `compile_mode` `flat`, `ap_only`, `ap_site`, `full` | Accepted. The compiled output was the same for each `compile_mode`. |
| Node type `plan`, `client`, or an unknown type in the pair | Accepted. The type is kept as `node_kind`. |
| A node with `"id": "lqosync3:node:r1"` | Accepted. v3 uses the given ID. |
| A node without `id` | v3 makes `libreqos:legacy-network-json:node:<hash>` from the name. |
| Two nodes with the same name at different levels | Rejected: `duplicate node id ...`. |
| A circuit with max 0.15 Mbps | Accepted offline. `lqosd` rejects it at run time (next table). |
| `flat_promotion` | 7 `client` nodes, but all 7 circuits have an empty parent. |

## Rules in `lqosd` (2.2 and 3.0.7 have the same strings)

- `Download max must be >= 0.2 Mbps`, `Upload max must be >= 0.2 Mbps`.
- `Download min must be >= 0.1 Mbps`, `Upload min must be >= 0.1 Mbps`.
- `network.json contains duplicate node names ... Node names must be globally unique.`
- `Parent node '...' does not exist`. `Parent node '...' is invalid for a flat network`.
- `N circuit(s) are unresolved in runtime topology and will be shaped under generated parent nodes.`
- IDs in `libreqos_network.json` fields such as `subscriber_id` must match `[A-Za-z0-9._-]{1,64}` with no leading or trailing `.` or `-`.

## CSV columns

- 2.2 example: 14 columns, with `Parent Node ID` after `Parent Node`.
- 3.0.7 example: 15 columns, with `Parent Node ID` and `Anchor Node ID` after `Parent Node`.
- 2.2 `LibreQoS.py` reads the columns by header name. It binds a circuit to a node by `Parent Node ID` first, then by name.

## Not known yet

- The effect of `compile_mode` on a manual tree at run time. The offline compiler does not show it.
- The run-time result of an empty Parent Node in v3. The v3 integration code says that an integration must not send an empty Parent Node ID, because the bridge boot gate then holds.
