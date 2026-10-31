---
type: community
cohesion: 0.29
members: 7
---

# PPPoE Speed Sources

**Cohesion:** 0.29 - loosely connected
**Members:** 7 nodes

## Members
- [[ActiveComment]] - code - src/config.rs
- [[Default]] - code - src/config.rs
- [[PppSource]] - code - src/config.rs
- [[ProfileComment]] - code - src/config.rs
- [[ProfileName]] - code - src/config.rs
- [[ProfileRateLimit]] - code - src/config.rs
- [[SecretComment]] - code - src/config.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/PPPoE_Speed_Sources
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Speed Parsing]]
- 1 edge to [[_COMMUNITY_Router Circuit Collection]]
- 1 edge to [[_COMMUNITY_Config Loading]]

## Top bridge nodes
- [[PppSource]] - degree 9, connects to 3 communities