---
type: community
cohesion: 0.24
members: 10
---

# Config Root and Checks

**Cohesion:** 0.24 - loosely connected
**Members:** 10 nodes

## Members
- [[dot-check_finite()]] - code - src/config.rs
- [[dot-targets_v3()]] - code - src/config.rs
- [[dot-validate()]] - code - src/config.rs
- [[CircuitNames]] - code - src/config.rs
- [[ClientTemplate]] - code - src/config.rs
- [[Config]] - code - src/config.rs
- [[Exclude]] - code - src/config.rs
- [[Nodes]] - code - src/config.rs
- [[Safety]] - code - src/config.rs
- [[Telegram]] - code - src/config.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Config_Root_and_Checks
SORT file.name ASC
```

## Connections to other communities
- 11 edges to [[_COMMUNITY_Router Circuit Collection]]
- 9 edges to [[_COMMUNITY_Topology and Node Templates]]
- 9 edges to [[_COMMUNITY_Config Loading]]
- 7 edges to [[_COMMUNITY_Cycle Assembly]]
- 3 edges to [[_COMMUNITY_v3 Node Rules]]
- 3 edges to [[_COMMUNITY_Embedded Defaults Merge]]
- 2 edges to [[_COMMUNITY_Speed and DHCP Sources]]
- 1 edge to [[_COMMUNITY_DHCP Settings]]
- 1 edge to [[_COMMUNITY_Runtime Settings]]
- 1 edge to [[_COMMUNITY_Offline Action]]
- 1 edge to [[_COMMUNITY_LibreQoS Targets]]
- 1 edge to [[_COMMUNITY_Network Modes]]
- 1 edge to [[_COMMUNITY_Backstop Caps]]

## Top bridge nodes
- [[Config]] - degree 48, connects to 13 communities
- [[dot-validate()]] - degree 5, connects to 2 communities
- [[Nodes]] - degree 4, connects to 2 communities
- [[Exclude]] - degree 3, connects to 2 communities
- [[CircuitNames]] - degree 2, connects to 1 community