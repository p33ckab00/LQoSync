---
type: community
cohesion: 0.27
members: 11
---

# Speed Rules Design

**Cohesion:** 0.27 - loosely connected
**Members:** 11 nodes

## Members
- [[Circuit rate formula (clamp, max_floor, min_rate_percentage)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Circuit rules per source (PPPoE, Hotspot, DHCP)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Comment speed format (Name25M5M)]] - concept - docs/CONFIGURATION.md
- [[Differences from updatecsv.py]] - rationale - docs/CONFIGURATION.md
- [[PPPoE speed source chain (speed.pppoe_sources)]] - concept - docs/CONFIGURATION.md
- [[Rate direction and units (rate_limit_order, unitless_unit)]] - concept - docs/CONFIGURATION.md
- [[Removal Guard (safety)]] - rationale - docs/CONFIGURATION.md
- [[Speed Rules]] - concept - docs/ARCHITECTURE.md
- [[Task 2 Rate format and speed rules (SpeedParser)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Task 4 Circuit collection (collect_router)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[updatecsv.py (legacy Python service)]] - concept - docs/CONFIGURATION.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Speed_Rules_Design
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Cycle and Node Rules]]
- 2 edges to [[_COMMUNITY_Config Rules and Modes]]
- 2 edges to [[_COMMUNITY_Output Files and State Store]]

## Top bridge nodes
- [[Task 2 Rate format and speed rules (SpeedParser)]] - degree 6, connects to 1 community
- [[Task 4 Circuit collection (collect_router)]] - degree 4, connects to 1 community
- [[Circuit rules per source (PPPoE, Hotspot, DHCP)]] - degree 4, connects to 1 community
- [[Comment speed format (Name25M5M)]] - degree 3, connects to 1 community
- [[Circuit rate formula (clamp, max_floor, min_rate_percentage)]] - degree 3, connects to 1 community