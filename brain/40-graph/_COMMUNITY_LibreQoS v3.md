---
type: community
members: 15
---

# LibreQoS v3

**Members:** 15 nodes

## Members
- [[dot-check_finite()]] - code - src/config.rs
- [[dot-server()]] - code - src/config.rs
- [[dot-sources()]] - code - src/config.rs
- [[dot-targets_v3()]] - code - src/config.rs
- [[dot-validate()]] - code - src/config.rs
- [[DhcpCfg]] - code - src/config.rs
- [[DhcpMode]] - code - src/config.rs
- [[DhcpServer]] - code - src/config.rs
- [[DhcpSource]] - code - src/config.rs
- [[LeaseComment]] - code - src/config.rs
- [[PerPlan]] - code - src/config.rs
- [[PerSite]] - code - src/config.rs
- [[ServerComment]] - code - src/config.rs
- [[Settings]] - code - src/config.rs
- [[dhcp_server_gives_override_or_default()]] - code - src/config.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/LibreQoS_v3
SORT file.name ASC
```

## Connections to other communities
- 7 edges to [[_COMMUNITY_Config]]
- 6 edges to [[_COMMUNITY_Circuit Collection]]
- 2 edges to [[_COMMUNITY_CLI and Checks 3]]
- 2 edges to [[_COMMUNITY_Secrets]]
- 1 edge to [[_COMMUNITY_Topology]]

## Top bridge nodes
- [[DhcpSource]] - degree 8, connects to 3 communities
- [[DhcpMode]] - degree 7, connects to 3 communities
- [[DhcpServer]] - degree 8, connects to 2 communities
- [[DhcpCfg]] - degree 5, connects to 2 communities
- [[dot-validate()]] - degree 5, connects to 2 communities