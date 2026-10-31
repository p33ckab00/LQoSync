---
type: community
members: 19
---

# MikroTik Session 3

**Members:** 19 nodes

## Members
- [[Applier (hash, atomic write, reload)]] - concept - docs/ARCHITECTURE.md
- [[Circuit Model]] - concept - docs/ARCHITECTURE.md
- [[Collector (RouterOS API reads)]] - concept - docs/ARCHITECTURE.md
- [[Config Loader]] - concept - docs/ARCHITECTURE.md
- [[MikroTik RouterOS API]] - concept - docs/ARCHITECTURE.md
- [[Performance Rules]] - rationale - docs/ARCHITECTURE.md
- [[Proposed Crate Layout]] - concept - docs/ARCHITECTURE.md
- [[Router State Machine (onlinefailedunreachableoffline)]] - rationale - docs/ARCHITECTURE.md
- [[ShapedDevices.csv]] - concept - docs/ARCHITECTURE.md
- [[State Store (state.json)]] - concept - docs/ARCHITECTURE.md
- [[Target Detector]] - concept - docs/ARCHITECTURE.md
- [[Target v2_pair]] - concept - docs/ARCHITECTURE.md
- [[Target v3_network]] - concept - docs/ARCHITECTURE.md
- [[Target v3_pair]] - concept - docs/ARCHITECTURE.md
- [[Validator]] - concept - docs/ARCHITECTURE.md
- [[Writers (one per target)]] - concept - docs/ARCHITECTURE.md
- [[libreqos_network.json]] - concept - docs/ARCHITECTURE.md
- [[lqosd (LibreQoS daemon)]] - concept - docs/ARCHITECTURE.md
- [[v2 vs v3 writer differences]] - concept - docs/ARCHITECTURE.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/MikroTik_Session_3
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Topology 2]]
- 1 edge to [[_COMMUNITY_Speed Rules]]
- 1 edge to [[_COMMUNITY_Speed Rules 3]]
- 1 edge to [[_COMMUNITY_MikroTik Session 4]]

## Top bridge nodes
- [[Writers (one per target)]] - degree 7, connects to 1 community
- [[Collector (RouterOS API reads)]] - degree 6, connects to 1 community
- [[Target Detector]] - degree 5, connects to 1 community
- [[Target v2_pair]] - degree 3, connects to 1 community