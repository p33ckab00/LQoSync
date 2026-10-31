---
type: community
members: 20
---

# Topology 3

**Members:** 20 nodes

## Members
- [[ADR-0006 v3 guards and node types]] - rationale - brain/HANDOFF.md
- [[Cycle assembly rules]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[LibreQoS v3 checks (no_parent, compile_mode)]] - concept - docs/CONFIGURATION.md
- [[LibreQoS v3 topology.compile_mode]] - concept - docs/ARCHITECTURE.md
- [[Node rules]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Node types per target (v2_type, v3_type)]] - concept - docs/CONFIGURATION.md
- [[Plan 1 review focus (edge cases)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Removal guard fingerprint (FNV-1a 64 of removed names)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[RouterOS listen change detection (future)]] - concept - docs/ARCHITECTURE.md
- [[Six Network Modes]] - concept - docs/ARCHITECTURE.md
- [[Stable lq3- node id (FNV-1a 64 of kindrouterkey)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[State seed (seed_state  seed_nodes)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Subscriber presence and subscriber_grace]] - concept - docs/CONFIGURATION.md
- [[Task 3 Model, state file, and seed]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Task 5 Topology (build_nodes, to_network_json)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Task 6 Cycle assembly and validation]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Topology Builder]] - concept - docs/ARCHITECTURE.md
- [[Troubleshooting table]] - concept - docs/OPERATIONS.md
- [[Validation]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[state.json state file]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Topology_3
SORT file.name ASC
```

## Connections to other communities
- 10 edges to [[_COMMUNITY_Community 36]]
- 4 edges to [[_COMMUNITY_Community 16]]
- 2 edges to [[_COMMUNITY_Speed Rules 3]]
- 1 edge to [[_COMMUNITY_Session Notes]]
- 1 edge to [[_COMMUNITY_Community 18]]
- 1 edge to [[_COMMUNITY_Topology 4]]
- 1 edge to [[_COMMUNITY_Status and Diff 2]]

## Top bridge nodes
- [[Cycle assembly rules]] - degree 8, connects to 3 communities
- [[Node rules]] - degree 7, connects to 2 communities
- [[Task 5 Topology (build_nodes, to_network_json)]] - degree 5, connects to 1 community
- [[Removal guard fingerprint (FNV-1a 64 of removed names)]] - degree 4, connects to 1 community
- [[state.json state file]] - degree 4, connects to 1 community