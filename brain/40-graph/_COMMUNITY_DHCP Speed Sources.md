---
type: community
cohesion: 0.29
members: 7
---

# DHCP Speed Sources

**Cohesion:** 0.29 - loosely connected
**Members:** 7 nodes

## Members
- [[dot-sources()]] - code - src/config.rs
- [[dot-targets_v3()]] - code - src/config.rs
- [[dot-validate()]] - code - src/config.rs
- [[DhcpSource]] - code - src/config.rs
- [[LeaseComment]] - code - src/config.rs
- [[ServerComment]] - code - src/config.rs
- [[Settings]] - code - src/config.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/DHCP_Speed_Sources
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Router Circuit Collection]]
- 2 edges to [[_COMMUNITY_DHCP Settings]]
- 2 edges to [[_COMMUNITY_Config Loading]]

## Top bridge nodes
- [[DhcpSource]] - degree 7, connects to 3 communities
- [[dot-validate()]] - degree 4, connects to 2 communities
- [[dot-sources()]] - degree 3, connects to 1 community
- [[dot-targets_v3()]] - degree 2, connects to 1 community