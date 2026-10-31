---
type: community
cohesion: 0.44
members: 9
---

# Output Target Selection

**Cohesion:** 0.44 - moderately connected
**Members:** 9 nodes

## Members
- [[LQoSync UpdateCSV README]] - document - README.md
- [[LibreQoS upgrade 2.2 to 3.x]] - concept - docs/OPERATIONS.md
- [[Target detector]] - concept - docs/ARCHITECTURE.md
- [[Writers (one per target)]] - concept - docs/ARCHITECTURE.md
- [[topology source in lqos.conf]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[target = auto]] - concept - README.md
- [[v2_pair target]] - concept - README.md
- [[v3_network target]] - concept - README.md
- [[v3_pair target]] - concept - README.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Output_Target_Selection
SORT file.name ASC
```

## Connections to other communities
- 7 edges to [[_COMMUNITY_Sync Pipeline Internals]]
- 3 edges to [[_COMMUNITY_Operations and Cutover]]
- 3 edges to [[_COMMUNITY_Brain Vault Notes]]
- 2 edges to [[_COMMUNITY_LibreQoS v3 Runtime Evidence]]
- 1 edge to [[_COMMUNITY_Configuration and Secrets]]
- 1 edge to [[_COMMUNITY_Runtime Mode]]

## Top bridge nodes
- [[LQoSync UpdateCSV README]] - degree 8, connects to 4 communities
- [[LibreQoS upgrade 2.2 to 3.x]] - degree 6, connects to 3 communities
- [[Target detector]] - degree 5, connects to 3 communities
- [[Writers (one per target)]] - degree 7, connects to 2 communities
- [[topology source in lqos.conf]] - degree 5, connects to 2 communities