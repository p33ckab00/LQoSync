---
type: community
cohesion: 0.22
members: 9
---

# Network Modes

**Cohesion:** 0.22 - loosely connected
**Members:** 9 nodes

## Members
- [[FlatGlobalRoot]] - code - src/config.rs
- [[FlatPromotion]] - code - src/config.rs
- [[FlatRouterRoot]] - code - src/config.rs
- [[GroupedRouterRoot]] - code - src/config.rs
- [[M]] - code - src/validate.rs
- [[Network]] - code - src/config.rs
- [[NetworkMode]] - code - src/config.rs
- [[NoParent]] - code - src/config.rs
- [[RootPromotion]] - code - src/config.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Network_Modes
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Validation]]
- 2 edges to [[_COMMUNITY_Config Loading]]
- 2 edges to [[_COMMUNITY_Topology and Node Templates]]
- 1 edge to [[_COMMUNITY_Config Root and Checks]]
- 1 edge to [[_COMMUNITY_Cycle Assembly]]

## Top bridge nodes
- [[NetworkMode]] - degree 14, connects to 4 communities
- [[Network]] - degree 3, connects to 2 communities
- [[M]] - degree 2, connects to 1 community