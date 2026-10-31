---
type: community
cohesion: 0.33
members: 6
---

# LibreQoS Targets

**Cohesion:** 0.33 - loosely connected
**Members:** 6 nodes

## Members
- [[Auto]] - code - src/config.rs
- [[Libreqos]] - code - src/config.rs
- [[Target]] - code - src/config.rs
- [[V2Pair]] - code - src/config.rs
- [[V3Network]] - code - src/config.rs
- [[V3Pair]] - code - src/config.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/LibreQoS_Targets
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Config Loading]]
- 1 edge to [[_COMMUNITY_Config Root and Checks]]

## Top bridge nodes
- [[Libreqos]] - degree 3, connects to 2 communities
- [[Target]] - degree 6, connects to 1 community