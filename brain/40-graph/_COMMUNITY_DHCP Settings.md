---
type: community
cohesion: 0.21
members: 13
---

# DHCP Settings

**Cohesion:** 0.21 - loosely connected
**Members:** 13 nodes

## Members
- [[dot-factors()]] - code - src/config.rs
- [[dot-factors()_1]] - code - src/config.rs
- [[dot-per_client()]] - code - src/config.rs
- [[dot-server()]] - code - src/config.rs
- [[dot-settings_rate()]] - code - src/config.rs
- [[BandwidthDefaults]] - code - src/config.rs
- [[DhcpCfg]] - code - src/config.rs
- [[DhcpMode]] - code - src/config.rs
- [[DhcpServer]] - code - src/config.rs
- [[HotspotCfg]] - code - src/config.rs
- [[PerPlan]] - code - src/config.rs
- [[PerSite]] - code - src/config.rs
- [[dhcp_server_gives_override_or_default()]] - code - src/config.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/DHCP_Settings
SORT file.name ASC
```

## Connections to other communities
- 7 edges to [[_COMMUNITY_Config Loading]]
- 4 edges to [[_COMMUNITY_Speed and DHCP Sources]]
- 3 edges to [[_COMMUNITY_Router Circuit Collection]]
- 1 edge to [[_COMMUNITY_Config Root and Checks]]
- 1 edge to [[_COMMUNITY_Topology and Node Templates]]

## Top bridge nodes
- [[DhcpMode]] - degree 7, connects to 3 communities
- [[DhcpServer]] - degree 8, connects to 2 communities
- [[BandwidthDefaults]] - degree 6, connects to 2 communities
- [[DhcpCfg]] - degree 5, connects to 2 communities
- [[HotspotCfg]] - degree 4, connects to 2 communities