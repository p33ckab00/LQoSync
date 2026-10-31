---
type: community
cohesion: 0.20
members: 12
---

# Config Rules and Modes

**Cohesion:** 0.20 - loosely connected
**Members:** 12 nodes

## Members
- [[Config Loader]] - concept - docs/ARCHITECTURE.md
- [[DHCP speed sources (per_site  per_plan)]] - concept - docs/CONFIGURATION.md
- [[LibreQoS v3 checks (no_parent, compile_mode)]] - concept - docs/CONFIGURATION.md
- [[LibreQoS v3 topology.compile_mode]] - concept - docs/ARCHITECTURE.md
- [[Manual installation procedure]] - concept - docs/INSTALLATION.md
- [[Secret files (_file, mode 0600 root)]] - concept - docs/CONFIGURATION.md
- [[Six Network Modes]] - concept - docs/ARCHITECTURE.md
- [[Task 1 Crate scaffold and config]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Topology Builder]] - concept - docs/ARCHITECTURE.md
- [[Troubleshooting table]] - concept - docs/OPERATIONS.md
- [[lqosync3.service systemd unit]] - concept - docs/INSTALLATION.md
- [[lqosync3.toml config file]] - concept - docs/CONFIGURATION.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Config_Rules_and_Modes
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Targets and Applier]]
- 2 edges to [[_COMMUNITY_Cycle and Node Rules]]
- 2 edges to [[_COMMUNITY_Speed Rules Design]]
- 2 edges to [[_COMMUNITY_Output Files and State Store]]
- 1 edge to [[_COMMUNITY_Decisions and Session Notes]]
- 1 edge to [[_COMMUNITY_Collector and Router State]]

## Top bridge nodes
- [[Six Network Modes]] - degree 5, connects to 2 communities
- [[lqosync3.service systemd unit]] - degree 3, connects to 2 communities
- [[LibreQoS v3 topology.compile_mode]] - degree 4, connects to 1 community
- [[Secret files (_file, mode 0600 root)]] - degree 4, connects to 1 community
- [[Manual installation procedure]] - degree 3, connects to 1 community