---
type: community
cohesion: 0.18
members: 20
---

# Scenario Test Harness

**Cohesion:** 0.18 - loosely connected
**Members:** 20 nodes

## Members
- [[1415 CSV columns (v2_pairv3_pair)]] - concept - brain/20-decisions/ADR-0005-topology-follows-v3.md
- [[ADR-0005 Topology follows v3]] - document - brain/20-decisions/ADR-0005-topology-follows-v3.md
- [[ADR-0006 v3 guards and node types]] - document - brain/20-decisions/ADR-0006-v3-guards-and-node-types.md
- [[Group nodes typed site on v3]] - rationale - brain/20-decisions/ADR-0006-v3-guards-and-node-types.md
- [[How LibreQoS v3 handles nodes]] - document - brain/30-knowledge/v3-node-handling.md
- [[Node rules]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Node types (v2_type  v3_type)]] - concept - docs/CONFIGURATION.md
- [[Parent Node ID column]] - concept - brain/20-decisions/ADR-0005-topology-follows-v3.md
- [[Stable lq3- node id (FNV-1a 64 of kindrouterkey)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Stable node id (lq3- FNV-1a 64)]] - rationale - brain/20-decisions/ADR-0005-topology-follows-v3.md
- [[Task 5 Topology (build_nodes, to_network_json)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
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
TABLE source_file, type FROM #community/Scenario_Test_Harness
SORT file.name ASC
```

## Connections to other communities
- 14 edges to [[_COMMUNITY_Early Sessions and ADRs]]
- 12 edges to [[_COMMUNITY_Plan 1 Session and Review]]
- 5 edges to [[_COMMUNITY_DHCP Discovery and Exclusions]]
- 3 edges to [[_COMMUNITY_Router State Decisions]]
- 1 edge to [[_COMMUNITY_Architecture Parts]]
- 1 edge to [[_COMMUNITY_v3 Ingress Evidence]]

## Top bridge nodes
- [[ADR-0005 Topology follows v3]] - degree 19, connects to 4 communities
- [[ADR-0006 v3 guards and node types]] - degree 13, connects to 3 communities
- [[How LibreQoS v3 handles nodes]] - degree 12, connects to 3 communities
- [[Task 5 Topology (build_nodes, to_network_json)]] - degree 5, connects to 3 communities
- [[1415 CSV columns (v2_pairv3_pair)]] - degree 5, connects to 2 communities