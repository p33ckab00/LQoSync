---
type: spec
created: 2026-11-19
status: approved
related: ["[[topology-manager-status]]", "[[2026-11-19-topology-live-acceptance]]"]
---

# Manager node locations design (Phase 3 blocker fix)

## Intent

The production enrollment dry run failed with `legacy managed policy parity conflict` and `ChangedNetwork` (2026-11-19).
All node rates were equal. The difference was the node location: the legacy writer (`topology::add_locations`) puts the `latitude` and `longitude` of the router (from `[[routers]]` in `config.toml`) on each node that it makes for that router. The manager export never writes a location.
So enrollment can never pass parity on a router with a location, and after activation the LibreQoS Site Map would lose the nodes.

## Owner decision (2026-11-19)

Approach A: the manager writes the location of the router on each node that comes from that router, as the legacy writer does. Nodes without a router (WAN without a source, intermediate, fallback group) get no location.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. Manager writes the router location (chosen) | Export adds `latitude` and `longitude` to each node whose source belongs to a configured router with a location | Exact parity; Site Map keeps the nodes |
| B. Parity ignores locations | Like the Comment column | Less code, but the Site Map loses the nodes after activation |

## Design

- Source of the location: `config.toml` `[[routers]] latitude` and `longitude`, matched by router name. The registry schema does not change (an older build must still load the registry). Designer call.
- Node to router: a node with `source_id` whose source origin is `Subscribers { scope.router_id }` or `Wan { router_id }`; the catalog router with that ID gives the router name. A node without a source has no location. Designer call, matches the legacy rule "each node that the tool made for a router".
- The export writes the two fields with the same JSON number values as the legacy writer, so `parity::compare_snapshots` (`numeric_network`) sees equal values.
- A router without a location, or with only one of the two values, gives no fields (config validation already refuses one value alone).
- The live-test rule model, admission, and the compiler path stay unchanged; check that admission and the controlled compiler accept nodes with a location.

## Verification

- Export test: a group node of a router with a location gets the two fields; a node of a router without a location, and a node without a source, get none.
- Parity test: a legacy pair written with `add_locations` and the manager pair of the same policy give no `ChangedNetwork`.
- Admission or compiler test (existing harness): a pair with node locations is admitted.
- Full suite, format, Clippy with warnings denied, and whitespace checks.
