---
type: community
cohesion: 0.20
members: 10
---

# Runtime Settings

**Cohesion:** 0.20 - loosely connected
**Members:** 10 nodes

## Members
- [[dot-default()_1]] - code - src/config.rs
- [[Apply]] - code - src/config.rs
- [[Debug]] - code - src/config.rs
- [[Error]] - code - src/config.rs
- [[Info]] - code - src/config.rs
- [[LogLevel]] - code - src/config.rs
- [[Runtime]] - code - src/config.rs
- [[RuntimeMode]] - code - src/config.rs
- [[Shadow]] - code - src/config.rs
- [[Warn]] - code - src/config.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Runtime_Settings
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Config Loading]]
- 1 edge to [[_COMMUNITY_Router Circuit Collection]]

## Top bridge nodes
- [[Runtime]] - degree 5, connects to 2 communities
- [[LogLevel]] - degree 6, connects to 1 community
- [[RuntimeMode]] - degree 4, connects to 1 community