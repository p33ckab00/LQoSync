---
type: community
cohesion: 1.00
members: 2
---

# Runtime Mode

**Cohesion:** 1.00 - tightly connected
**Members:** 2 nodes

## Members
- [[runtime.mode (shadow  apply)]] - concept - docs/CONFIGURATION.md
- [[shadow mode]] - rationale - README.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Runtime_Mode
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Operations and Cutover]]
- 1 edge to [[_COMMUNITY_Output Target Selection]]
- 1 edge to [[_COMMUNITY_Configuration and Secrets]]
- 1 edge to [[_COMMUNITY_Installation]]

## Top bridge nodes
- [[shadow mode]] - degree 4, connects to 3 communities
- [[runtime.mode (shadow  apply)]] - degree 2, connects to 1 community