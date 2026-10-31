---
type: community
members: 9
---

# Topology 6

**Members:** 9 nodes

## Members
- [[LQoSync3 README]] - document - README.md
- [[Output target v2_pair]] - concept - README.md
- [[Output target v3_network]] - concept - README.md
- [[Output target v3_pair]] - concept - README.md
- [[Target auto-detection]] - rationale - README.md
- [[Write files only on content change]] - rationale - README.md
- [[topology source setting]] - concept - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[libreqos_network.json schema_version 2]] - concept - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[no_parent guard on v3 targets]] - rationale - brain/20-decisions/ADR-0006-v3-guards-and-node-types.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Topology_6
SORT file.name ASC
```

## Connections to other communities
- 6 edges to [[_COMMUNITY_Topology 4]]
- 4 edges to [[_COMMUNITY_Topology 3]]
- 3 edges to [[_COMMUNITY_Topology 5]]
- 2 edges to [[_COMMUNITY_Speed Rules 2]]
- 1 edge to [[_COMMUNITY_Status and Diff 5]]

## Top bridge nodes
- [[LQoSync3 README]] - degree 13, connects to 4 communities
- [[Output target v3_pair]] - degree 5, connects to 2 communities
- [[Output target v3_network]] - degree 5, connects to 1 community
- [[no_parent guard on v3 targets]] - degree 5, connects to 1 community
- [[Output target v2_pair]] - degree 3, connects to 1 community