---
type: community
cohesion: 0.11
members: 32
---

# DHCP Discovery and Exclusions

**Cohesion:** 0.11 - loosely connected
**Members:** 32 nodes

## Members
- [[etclqosync3lqosync3.toml]] - concept - docs/CONFIGURATION.md
- [[ADR-0009 DHCP discovery and exclusions]] - document - brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md
- [[Case-insensitive  wildcard name matching]] - rationale - brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md
- [[Circuit rate clamp formula (max_floor_mbps, safe_min_mbps)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Circuit rules for each source]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Comment speed format]] - concept - docs/CONFIGURATION.md
- [[Configfrom_toml_str (defaults merge)]] - code - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Configuration reference]] - document - docs/CONFIGURATION.md
- [[DHCP server discovery (like PPP)]] - rationale - brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md
- [[DHCP speed_sources (per_site  per_plan)]] - concept - docs/CONFIGURATION.md
- [[DhcpCfgserver(name) defaults]] - code - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Differences from updatecsv.py]] - concept - docs/CONFIGURATION.md
- [[Exclusions (exclude  routers.exclude dhcp_servers, ppp_profiles, ips)]] - rationale - brain/20-decisions/ADR-0009-dhcp-discovery-and-exclusions.md
- [[Exclusions config (exclude, routers.exclude)]] - concept - docs/CONFIGURATION.md
- [[LibreQoS v3 checks (no_parent, compile_mode)]] - concept - docs/CONFIGURATION.md
- [[Map from config.json (import-json)]] - concept - docs/CONFIGURATION.md
- [[Name templates (nodes, circuits)]] - concept - docs/CONFIGURATION.md
- [[Output format (round3, fmt_rate, 1415 CSV columns)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Removal guard (safety)]] - rationale - docs/CONFIGURATION.md
- [[RouterOS listen change detection (future)]] - concept - docs/ARCHITECTURE.md
- [[Speed Rules]] - concept - docs/ARCHITECTURE.md
- [[Subscriber presence (subscriber_grace)]] - concept - docs/CONFIGURATION.md
- [[Task 1 Crate scaffold and config]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Task 2 Rate format and speed rules (SpeedParser)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Task 4 Circuit collection (collect_router)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[router_defaults (merged into each routers)]] - concept - docs/CONFIGURATION.md
- [[routers.dhcp (discover, server_mode, require_bound, routers.dhcp.servers)]] - concept - docs/CONFIGURATION.md
- [[configdefaults.toml (embedded defaults)]] - rationale - docs/CONFIGURATION.md
- [[lqosync3 check]] - code - docs/CONFIGURATION.md
- [[lqosync3 diff]] - code - docs/CONFIGURATION.md
- [[rate_limit_order  unitless_unit]] - rationale - docs/CONFIGURATION.md
- [[speed.pppoe_sources]] - concept - docs/CONFIGURATION.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/DHCP_Discovery_and_Exclusions
SORT file.name ASC
```

## Connections to other communities
- 13 edges to [[_COMMUNITY_Plan 1 Session and Review]]
- 8 edges to [[_COMMUNITY_Early Sessions and ADRs]]
- 6 edges to [[_COMMUNITY_Router State Decisions]]
- 5 edges to [[_COMMUNITY_Scenario Test Harness]]
- 5 edges to [[_COMMUNITY_Architecture Parts]]

## Top bridge nodes
- [[Configuration reference]] - degree 31, connects to 5 communities
- [[ADR-0009 DHCP discovery and exclusions]] - degree 7, connects to 3 communities
- [[Differences from updatecsv.py]] - degree 6, connects to 3 communities
- [[Output format (round3, fmt_rate, 1415 CSV columns)]] - degree 4, connects to 3 communities
- [[Removal guard (safety)]] - degree 5, connects to 2 communities