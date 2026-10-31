---
type: community
cohesion: 0.33
members: 6
---

# Circuit Sources

**Cohesion:** 0.33 - loosely connected
**Members:** 6 nodes

## Members
- [[Dhcp]] - code - src/model.rs
- [[Entry]] - code - src/cycle.rs
- [[Hotspot_1]] - code - src/model.rs
- [[Ppp_1]] - code - src/model.rs
- [[Source]] - code - src/model.rs
- [[Unknown]] - code - src/model.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Circuit_Sources
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Cycle Assembly]]
- 2 edges to [[_COMMUNITY_Router Circuit Collection]]
- 1 edge to [[_COMMUNITY_CSV Writer and Columns]]
- 1 edge to [[_COMMUNITY_State File and CSV Read]]
- 1 edge to [[_COMMUNITY_Model Types]]

## Top bridge nodes
- [[Source]] - degree 11, connects to 4 communities
- [[Entry]] - degree 3, connects to 2 communities