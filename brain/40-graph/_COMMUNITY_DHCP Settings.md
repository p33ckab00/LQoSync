---
type: community
cohesion: 0.18
members: 13
---

# DHCP Settings

**Cohesion:** 0.18 - loosely connected
**Members:** 13 nodes

## Members
- [[dot-default()_13]] - code - src/config.rs
- [[dot-default()_14]] - code - src/config.rs
- [[dot-factors()]] - code - src/config.rs
- [[dot-factors()_1]] - code - src/config.rs
- [[dot-per_client()]] - code - src/config.rs
- [[dot-settings_rate()]] - code - src/config.rs
- [[BandwidthDefaults]] - code - src/config.rs
- [[DhcpCfg]] - code - src/config.rs
- [[DhcpMode]] - code - src/config.rs
- [[DhcpServer]] - code - src/config.rs
- [[HotspotCfg]] - code - src/config.rs
- [[PerPlan]] - code - src/config.rs
- [[PerSite]] - code - src/config.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/DHCP_Settings
SORT file.name ASC
```

## Connections to other communities
- 5 edges to [[_COMMUNITY_Config Loading]]
- 4 edges to [[_COMMUNITY_Router Circuit Collection]]
- 2 edges to [[_COMMUNITY_DHCP Speed Sources]]
- 2 edges to [[_COMMUNITY_Speed Parsing]]
- 1 edge to [[_COMMUNITY_Topology and Node IDs]]

## Top bridge nodes
- [[DhcpMode]] - degree 6, connects to 3 communities
- [[BandwidthDefaults]] - degree 7, connects to 2 communities
- [[DhcpServer]] - degree 7, connects to 2 communities
- [[HotspotCfg]] - degree 5, connects to 2 communities
- [[DhcpCfg]] - degree 3, connects to 2 communities