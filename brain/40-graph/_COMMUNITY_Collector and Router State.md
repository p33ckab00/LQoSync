---
type: community
cohesion: 0.29
members: 8
---

# Collector and Router State

**Cohesion:** 0.29 - loosely connected
**Members:** 8 nodes

## Members
- [[Circuit Model]] - concept - docs/ARCHITECTURE.md
- [[Collector (RouterOS API reads)]] - concept - docs/ARCHITECTURE.md
- [[MikroTik RouterOS API]] - concept - docs/ARCHITECTURE.md
- [[Performance Rules]] - rationale - docs/ARCHITECTURE.md
- [[Router State Machine (onlinefailedunreachableoffline)]] - rationale - docs/ARCHITECTURE.md
- [[Router State settings (offline_after, offline_action)]] - concept - docs/CONFIGURATION.md
- [[RouterOS listen change detection (future)]] - concept - docs/ARCHITECTURE.md
- [[Subscriber Presence and subscriber_grace]] - concept - docs/CONFIGURATION.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Collector_and_Router_State
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Config Rules and Modes]]
- 1 edge to [[_COMMUNITY_Output Files and State Store]]
- 1 edge to [[_COMMUNITY_Cycle and Node Rules]]
- 1 edge to [[_COMMUNITY_Targets and Applier]]
- 1 edge to [[_COMMUNITY_Pure Core Design]]

## Top bridge nodes
- [[Collector (RouterOS API reads)]] - degree 6, connects to 1 community
- [[Router State Machine (onlinefailedunreachableoffline)]] - degree 3, connects to 1 community
- [[Subscriber Presence and subscriber_grace]] - degree 3, connects to 1 community
- [[Circuit Model]] - degree 2, connects to 1 community
- [[Performance Rules]] - degree 2, connects to 1 community