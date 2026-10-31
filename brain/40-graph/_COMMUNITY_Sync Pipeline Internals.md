---
type: community
cohesion: 0.19
members: 15
---

# Sync Pipeline Internals

**Cohesion:** 0.19 - loosely connected
**Members:** 15 nodes

## Members
- [[Applier]] - concept - docs/ARCHITECTURE.md
- [[Architecture document]] - document - docs/ARCHITECTURE.md
- [[Atomic write via temp file and rename]] - rationale - docs/ARCHITECTURE.md
- [[Circuit model]] - concept - docs/ARCHITECTURE.md
- [[Collector]] - concept - docs/ARCHITECTURE.md
- [[Notifier (Telegram)]] - concept - docs/ARCHITECTURE.md
- [[Performance rules]] - rationale - docs/ARCHITECTURE.md
- [[SHA-256 content hash skip]] - rationale - docs/ARCHITECTURE.md
- [[Six network modes]] - concept - docs/ARCHITECTURE.md
- [[Target selection flowchart]] - concept - docs/ARCHITECTURE.md
- [[Topology builder]] - concept - docs/ARCHITECTURE.md
- [[Validator]] - concept - docs/ARCHITECTURE.md
- [[libreqos_network.json schema 2]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[topology.compile_mode = full]] - concept - docs/ARCHITECTURE.md
- [[v3 schema validation results]] - concept - docs/LIBREQOS-V3-INGRESS.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Sync_Pipeline_Internals
SORT file.name ASC
```

## Connections to other communities
- 7 edges to [[_COMMUNITY_Output Target Selection]]
- 4 edges to [[_COMMUNITY_Brain Vault Notes]]
- 3 edges to [[_COMMUNITY_LibreQoS v3 Runtime Evidence]]
- 2 edges to [[_COMMUNITY_Operations and Cutover]]
- 2 edges to [[_COMMUNITY_Configuration and Secrets]]
- 2 edges to [[_COMMUNITY_Speed Rules and Open Decisions]]

## Top bridge nodes
- [[Architecture document]] - degree 19, connects to 4 communities
- [[Target selection flowchart]] - degree 3, connects to 2 communities
- [[libreqos_network.json schema 2]] - degree 3, connects to 2 communities
- [[Circuit model]] - degree 4, connects to 1 community
- [[topology.compile_mode = full]] - degree 4, connects to 1 community