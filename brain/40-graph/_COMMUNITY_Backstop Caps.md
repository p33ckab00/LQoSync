---
type: community
cohesion: 0.67
members: 3
---

# Backstop Caps

**Cohesion:** 0.67 - moderately connected
**Members:** 3 nodes

## Members
- [[dot-limit()]] - code - src/config.rs
- [[BackstopCap]] - code - src/config.rs
- [[BackstopCaps]] - code - src/config.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Backstop_Caps
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Config Loading]]
- 1 edge to [[_COMMUNITY_Config Root and Checks]]
- 1 edge to [[_COMMUNITY_Topology and Node Templates]]
- 1 edge to [[_COMMUNITY_Speed and DHCP Sources]]

## Top bridge nodes
- [[BackstopCap]] - degree 4, connects to 2 communities
- [[BackstopCaps]] - degree 3, connects to 2 communities
- [[dot-limit()]] - degree 2, connects to 1 community