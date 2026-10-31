---
type: community
cohesion: 0.32
members: 8
---

# Pure Core Design

**Cohesion:** 0.32 - loosely connected
**Members:** 8 nodes

## Members
- [[Plan 1 global constraints]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Plan 1 review focus (edge cases)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[RouterResult (OnlineFailedUnreachableOffline)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[RouterSnapshot (six RouterOS tables)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[State seed from live ShapedDevices.csv]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Task 3 Model, state file, and seed]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[assemble() pure core function]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[state.json format (version 1)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Pure_Core_Design
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Cycle and Node Rules]]
- 2 edges to [[_COMMUNITY_Output Files and State Store]]
- 1 edge to [[_COMMUNITY_Decisions and Session Notes]]
- 1 edge to [[_COMMUNITY_Collector and Router State]]

## Top bridge nodes
- [[assemble() pure core function]] - degree 5, connects to 2 communities
- [[RouterResult (OnlineFailedUnreachableOffline)]] - degree 4, connects to 2 communities
- [[state.json format (version 1)]] - degree 4, connects to 1 community
- [[State seed from live ShapedDevices.csv]] - degree 3, connects to 1 community
- [[Plan 1 review focus (edge cases)]] - degree 2, connects to 1 community