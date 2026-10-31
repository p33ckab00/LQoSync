---
type: community
members: 11
---

# Speed Rules 3

**Members:** 11 nodes

## Members
- [[ADR-0009 DHCP discovery and exclusions_1]] - document - brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md
- [[Case-insensitive  wildcard name matching]] - rationale - brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md
- [[Circuit rate clamp formula (max_floor_mbps, safe_min_mbps)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Circuit rules for each source]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[DHCP server discovery (like PPP)]] - rationale - brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md
- [[DhcpCfgserver(name) defaults]] - code - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Exclusions (exclude  routers.exclude dhcp_servers, ppp_profiles, ips)]] - rationale - brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md
- [[Name templates (nodes, circuits)]] - rationale - docs/CONFIGURATION.md
- [[Speed Rules]] - concept - docs/ARCHITECTURE.md
- [[Task 2 Rate format and speed rules (SpeedParser)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Task 4 Circuit collection (collect_router)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Speed_Rules_3
SORT file.name ASC
```

## Connections to other communities
- 8 edges to [[_COMMUNITY_Community 36]]
- 2 edges to [[_COMMUNITY_Topology 3]]
- 2 edges to [[_COMMUNITY_Community 16]]
- 1 edge to [[_COMMUNITY_MikroTik Session 5]]
- 1 edge to [[_COMMUNITY_Status and Diff 2]]

## Top bridge nodes
- [[ADR-0009 DHCP discovery and exclusions_1]] - degree 9, connects to 3 communities
- [[Circuit rules for each source]] - degree 9, connects to 2 communities
- [[Task 2 Rate format and speed rules (SpeedParser)]] - degree 4, connects to 1 community
- [[Exclusions (exclude  routers.exclude dhcp_servers, ppp_profiles, ips)]] - degree 4, connects to 1 community
- [[Task 4 Circuit collection (collect_router)]] - degree 3, connects to 1 community