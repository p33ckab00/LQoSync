---
type: community
cohesion: 0.52
members: 7
---

# Cycle and Node Rules

**Cohesion:** 0.52 - moderately connected
**Members:** 7 nodes

## Members
- [[Cycle assembly rules (8 ordered rules)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Guard hold fingerprint (FNV-1a 64 of removed names)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Node rules (group nodes, roots, client)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Stable node id (lq3- + FNV-1a 64)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Task 5 Topology (build_nodes, to_network_json)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Task 6 Cycle assembly and validation]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Validation stop rules]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Cycle_and_Node_Rules
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Speed Rules Design]]
- 3 edges to [[_COMMUNITY_Pure Core Design]]
- 2 edges to [[_COMMUNITY_Config Rules and Modes]]
- 2 edges to [[_COMMUNITY_Output Files and State Store]]
- 1 edge to [[_COMMUNITY_Collector and Router State]]

## Top bridge nodes
- [[Cycle assembly rules (8 ordered rules)]] - degree 7, connects to 3 communities
- [[Task 5 Topology (build_nodes, to_network_json)]] - degree 5, connects to 2 communities
- [[Node rules (group nodes, roots, client)]] - degree 5, connects to 2 communities
- [[Task 6 Cycle assembly and validation]] - degree 6, connects to 1 community
- [[Guard hold fingerprint (FNV-1a 64 of removed names)]] - degree 4, connects to 1 community