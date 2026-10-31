---
type: community
members: 12
---

# Topology 4

**Members:** 12 nodes

## Members
- [[ADR-0006 v3 guards and node types_1]] - document - brain/20-decisions/ADR-0006-v3-guards-and-node-types.md
- [[Group nodes typed site on v3]] - rationale - brain/20-decisions/ADR-0006-v3-guards-and-node-types.md
- [[How LibreQoS v3 handles nodes]] - document - brain/30-knowledge/v3-node-handling.md
- [[Tool must reject duplicate IP and node name]] - concept - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[compile_mode (flatap_onlyap_sitefull)]] - concept - brain/30-knowledge/v3-node-handling.md
- [[compile_mode not full warning]] - concept - brain/20-decisions/ADR-0006-v3-guards-and-node-types.md
- [[flat_promotion parent fix]] - rationale - brain/20-decisions/ADR-0005-topology-follows-v3.md
- [[lqosd runtime rules (rates, unique names, parents)]] - concept - brain/30-knowledge/v3-node-handling.md
- [[max_floor_mbps (0.2)  safe_min_mbps (0.1)]] - concept - brain/20-decisions/ADR-0005-topology-follows-v3.md
- [[v3 bridge boot gate]] - concept - brain/30-knowledge/v3-node-handling.md
- [[v3 name-derived legacy node id]] - concept - brain/30-knowledge/v3-node-handling.md
- [[v3 offline compile results]] - concept - brain/30-knowledge/v3-node-handling.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Topology_4
SORT file.name ASC
```

## Connections to other communities
- 11 edges to [[_COMMUNITY_Community 36]]
- 4 edges to [[_COMMUNITY_Topology 2]]
- 2 edges to [[_COMMUNITY_Session Notes]]
- 1 edge to [[_COMMUNITY_Topology 3]]
- 1 edge to [[_COMMUNITY_Community 16]]
- 1 edge to [[_COMMUNITY_Status and Diff 2]]

## Top bridge nodes
- [[ADR-0006 v3 guards and node types_1]] - degree 12, connects to 6 communities
- [[How LibreQoS v3 handles nodes]] - degree 12, connects to 2 communities
- [[v3 bridge boot gate]] - degree 3, connects to 1 community
- [[v3 name-derived legacy node id]] - degree 3, connects to 1 community
- [[max_floor_mbps (0.2)  safe_min_mbps (0.1)]] - degree 2, connects to 1 community