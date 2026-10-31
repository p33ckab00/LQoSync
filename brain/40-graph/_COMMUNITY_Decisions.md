---
type: community
members: 6
---

# Decisions

**Members:** 6 nodes

## Members
- [[ADR-0001 One model, many writers]] - rationale - brain/20-decisions/ADR-0001-one-model-many-writers.md
- [[ADR-0003 Router failed vs offline_1]] - document - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[Architecture (LQoSync3)]] - document - docs/ARCHITECTURE.md
- [[Router state model (onlinefailedunreachableoffline)]] - rationale - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[offline_action (default remove)]] - concept - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[offline_after (default 300 s)]] - concept - brain/20-decisions/ADR-0003-router-failed-vs-offline.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Decisions
SORT file.name ASC
```

## Connections to other communities
- 5 edges to [[_COMMUNITY_Speed Rules 2]]
- 3 edges to [[_COMMUNITY_Topology 3]]
- 2 edges to [[_COMMUNITY_Topology 2]]
- 2 edges to [[_COMMUNITY_Topology 4]]
- 2 edges to [[_COMMUNITY_Topology 5]]
- 1 edge to [[_COMMUNITY_Session Notes]]
- 1 edge to [[_COMMUNITY_Router State 3]]

## Top bridge nodes
- [[ADR-0003 Router failed vs offline_1]] - degree 10, connects to 4 communities
- [[ADR-0001 One model, many writers]] - degree 7, connects to 4 communities
- [[Architecture (LQoSync3)]] - degree 6, connects to 4 communities
- [[Router state model (onlinefailedunreachableoffline)]] - degree 5, connects to 1 community