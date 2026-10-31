---
type: community
members: 20
---

# Speed Rules

**Members:** 20 nodes

## Members
- [[ADR-0009 DHCP discovery and exclusions]] - document - brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md
- [[Case-insensitive  wildcard name matching]] - rationale - brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md
- [[Circuit rate clamp formula (max_floor_mbps, safe_min_mbps)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Circuit rules for each source]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Cycle assembly rules]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[DHCP server discovery (ipdhcp-server, discover)]] - concept - docs/CONFIGURATION.md
- [[DHCP server discovery (like PPP)]] - rationale - brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md
- [[DhcpCfgserver(name) defaults]] - code - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Exclusions (exclude  routers.exclude dhcp_servers, ppp_profiles, ips)]] - rationale - brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md
- [[Exclusions (exclude, routers.exclude)]] - concept - docs/CONFIGURATION.md
- [[Name templates (nodes, circuits)]] - rationale - docs/CONFIGURATION.md
- [[Output format (round3, fmt_rate, 1415 CSV columns)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Removal guard (safety)]] - rationale - docs/CONFIGURATION.md
- [[Removal guard on (20% or 50 rows held)]] - concept - brain/HANDOFF.md
- [[RouterOS listen change detection (future)]] - concept - docs/ARCHITECTURE.md
- [[Speed Rules]] - concept - docs/ARCHITECTURE.md
- [[Subscriber presence and subscriber_grace]] - concept - docs/CONFIGURATION.md
- [[Task 2 Rate format and speed rules (SpeedParser)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Task 4 Circuit collection (collect_router)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Validation]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Speed_Rules
SORT file.name ASC
```

## Connections to other communities
- 9 edges to [[_COMMUNITY_Topology 4]]
- 6 edges to [[_COMMUNITY_CLI and Checks 2]]
- 3 edges to [[_COMMUNITY_Topology 2]]
- 3 edges to [[_COMMUNITY_Speed Rules 2]]
- 2 edges to [[_COMMUNITY_MikroTik Session]]
- 1 edge to [[_COMMUNITY_Topology 3]]
- 1 edge to [[_COMMUNITY_Router Reads 2]]
- 1 edge to [[_COMMUNITY_Router Reads 3]]

## Top bridge nodes
- [[ADR-0009 DHCP discovery and exclusions]] - degree 11, connects to 4 communities
- [[Output format (round3, fmt_rate, 1415 CSV columns)]] - degree 4, connects to 3 communities
- [[Cycle assembly rules]] - degree 8, connects to 2 communities
- [[Circuit rules for each source]] - degree 9, connects to 1 community
- [[Removal guard (safety)]] - degree 6, connects to 1 community