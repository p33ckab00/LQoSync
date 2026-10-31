---
type: community
cohesion: 0.24
members: 10
---

# Model Types

**Cohesion:** 0.24 - loosely connected
**Members:** 10 nodes

## Members
- [[dot-default()]] - code - src/model.rs
- [[GuardHold]] - code - src/model.rs
- [[RouterMemo]] - code - src/model.rs
- [[State]] - code - src/model.rs
- [[V2_COLUMNS]] - code - src/model.rs
- [[V3_COLUMNS]] - code - src/model.rs
- [[btreemap]] - concept
- [[deserialize]] - concept
- [[model.rs]] - code - src/model.rs
- [[serialize]] - concept

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Model_Types
SORT file.name ASC
```

## Connections to other communities
- 7 edges to [[_COMMUNITY_Cycle Assembly]]
- 4 edges to [[_COMMUNITY_v3 Node Rules]]
- 3 edges to [[_COMMUNITY_Topology and Node Templates]]
- 3 edges to [[_COMMUNITY_CSV Writer and Columns]]
- 3 edges to [[_COMMUNITY_State File and CSV Read]]
- 3 edges to [[_COMMUNITY_Router Circuit Collection]]
- 1 edge to [[_COMMUNITY_Circuit Sources]]
- 1 edge to [[_COMMUNITY_Config Loading]]

## Top bridge nodes
- [[model.rs]] - degree 15, connects to 5 communities
- [[btreemap]] - degree 6, connects to 5 communities
- [[State]] - degree 13, connects to 4 communities
- [[deserialize]] - degree 3, connects to 2 communities
- [[GuardHold]] - degree 3, connects to 1 community