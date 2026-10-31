---
type: community
cohesion: 0.21
members: 12
---

# Output Files and State Store

**Cohesion:** 0.21 - loosely connected
**Members:** 12 nodes

## Members
- [[Name templates (nodes, circuits)]] - concept - docs/CONFIGURATION.md
- [[Node type defaults per target]] - concept - docs/CONFIGURATION.md
- [[Output format (round3, fmt_rate, CSV sort, JSON indent)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Proposed Crate Layout]] - concept - docs/ARCHITECTURE.md
- [[ShapedDevices.csv]] - concept - docs/ARCHITECTURE.md
- [[State Store (state.json)]] - concept - docs/ARCHITECTURE.md
- [[Target v2_pair]] - concept - docs/ARCHITECTURE.md
- [[Task 7 Pair writer (render_csv, render_network_json)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Validator]] - concept - docs/ARCHITECTURE.md
- [[Writers (one per target)]] - concept - docs/ARCHITECTURE.md
- [[network.json]] - concept - docs/ARCHITECTURE.md
- [[v2 vs v3 writer differences]] - concept - docs/ARCHITECTURE.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Output_Files_and_State_Store
SORT file.name ASC
```

## Connections to other communities
- 5 edges to [[_COMMUNITY_Targets and Applier]]
- 2 edges to [[_COMMUNITY_Config Rules and Modes]]
- 2 edges to [[_COMMUNITY_Pure Core Design]]
- 2 edges to [[_COMMUNITY_Cycle and Node Rules]]
- 2 edges to [[_COMMUNITY_Speed Rules Design]]
- 1 edge to [[_COMMUNITY_Collector and Router State]]

## Top bridge nodes
- [[Validator]] - degree 5, connects to 3 communities
- [[ShapedDevices.csv]] - degree 6, connects to 2 communities
- [[Target v2_pair]] - degree 5, connects to 2 communities
- [[Node type defaults per target]] - degree 3, connects to 2 communities
- [[Writers (one per target)]] - degree 7, connects to 1 community