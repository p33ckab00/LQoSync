---
type: community
members: 21
---

# MikroTik Session

**Members:** 21 nodes

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
- [[Task 7 Pair writer (render_csv, render_network_json)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Validator]] - concept - docs/ARCHITECTURE.md
- [[Writers (one per target)]] - concept - docs/ARCHITECTURE.md
- [[libreqos_network.json]] - concept - docs/ARCHITECTURE.md
- [[lqosd (LibreQoS daemon)]] - concept - docs/ARCHITECTURE.md
- [[network.json]] - concept - docs/ARCHITECTURE.md
- [[v2 vs v3 writer differences]] - concept - docs/ARCHITECTURE.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/MikroTik_Session
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Speed Rules]]
- 1 edge to [[_COMMUNITY_Topology 3]]

## Top bridge nodes
- [[Collector (RouterOS API reads)]] - degree 6, connects to 1 community
- [[Target Detector]] - degree 5, connects to 1 community
- [[Task 7 Pair writer (render_csv, render_network_json)]] - degree 2, connects to 1 community