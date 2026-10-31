---
type: community
members: 20
---

# Speed Rules

**Members:** 20 nodes

## Members
- [[ADR-0006 v3 guards and node types]] - rationale - brain/HANDOFF.md
- [[ADR-0009 DHCP discovery and exclusions]] - rationale - brain/Home.md
- [[Comment speed format (Name25M5M)]] - concept - docs/CONFIGURATION.md
- [[Configuration reference]] - document - docs/CONFIGURATION.md
- [[DHCP server discovery]] - concept - docs/CONFIGURATION.md
- [[Defaults layering over configdefaults.toml]] - rationale - docs/CONFIGURATION.md
- [[Exclusions (exclude, routers.exclude)]] - concept - docs/CONFIGURATION.md
- [[Home (vault MOC)]] - document - brain/Home.md
- [[LibreQoS v3 checks (no_parent, compile_mode)]] - concept - docs/CONFIGURATION.md
- [[Loader validation (unknown keys, ranges)]] - concept - docs/CONFIGURATION.md
- [[Map from config.json]] - concept - docs/CONFIGURATION.md
- [[Node types per target (v2_type, v3_type)]] - concept - docs/CONFIGURATION.md
- [[PPPoE speed sources order]] - concept - docs/CONFIGURATION.md
- [[Policy in TOML, formats in code]] - rationale - docs/CONFIGURATION.md
- [[RouterOS listen change detection (future)]] - concept - docs/ARCHITECTURE.md
- [[Subscriber presence and subscriber_grace]] - concept - docs/CONFIGURATION.md
- [[Troubleshooting table]] - concept - docs/OPERATIONS.md
- [[configdefaults.toml embedded defaults]] - concept - brain/HANDOFF.md
- [[lqosync3 import-json (planned)]] - concept - docs/CONFIGURATION.md
- [[rate_limit_order and unitless_unit]] - rationale - docs/CONFIGURATION.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Speed_Rules
SORT file.name ASC
```

## Connections to other communities
- 7 edges to [[_COMMUNITY_Status and Diff 2]]
- 4 edges to [[_COMMUNITY_Telegram Notifier]]
- 4 edges to [[_COMMUNITY_Session Notes]]
- 2 edges to [[_COMMUNITY_Topology 3]]
- 1 edge to [[_COMMUNITY_Topology 2]]
- 1 edge to [[_COMMUNITY_MikroTik Session 3]]
- 1 edge to [[_COMMUNITY_Speed Rules 3]]
- 1 edge to [[_COMMUNITY_Telegram Notifier 3]]
- 1 edge to [[_COMMUNITY_Router State 3]]
- 1 edge to [[_COMMUNITY_Plan 2 Runtime]]

## Top bridge nodes
- [[Configuration reference]] - degree 25, connects to 7 communities
- [[Home (vault MOC)]] - degree 7, connects to 3 communities
- [[ADR-0009 DHCP discovery and exclusions]] - degree 4, connects to 1 community
- [[Node types per target (v2_type, v3_type)]] - degree 3, connects to 1 community
- [[Subscriber presence and subscriber_grace]] - degree 3, connects to 1 community