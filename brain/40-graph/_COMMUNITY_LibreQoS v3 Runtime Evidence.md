---
type: community
cohesion: 0.27
members: 11
---

# LibreQoS v3 Runtime Evidence

**Cohesion:** 0.27 - loosely connected
**Members:** 11 nodes

## Members
- [[runlqosbus 0777 safety caution]] - rationale - docs/LIBREQOS-V3-INGRESS.md
- [[Last-good topology retention]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[LibreQoS v3 ingress document]] - document - docs/LIBREQOS-V3-INGRESS.md
- [[LibreQoS.py --updateonly no-op in v3]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[Offline v3 simulation]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[liblqos_python.so binding]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[libreqos-v3-preflight]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[libreqos_3.0.7 beta package]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[lqosd (v3 control plane)]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[lqosd inotify file watcher]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[topology_import.json (internal)]] - concept - docs/LIBREQOS-V3-INGRESS.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/LibreQoS_v3_Runtime_Evidence
SORT file.name ASC
```

## Connections to other communities
- 7 edges to [[_COMMUNITY_Brain Vault Notes]]
- 3 edges to [[_COMMUNITY_Sync Pipeline Internals]]
- 2 edges to [[_COMMUNITY_Output Target Selection]]
- 1 edge to [[_COMMUNITY_Operations and Cutover]]

## Top bridge nodes
- [[LibreQoS v3 ingress document]] - degree 15, connects to 3 communities
- [[libreqos-v3-preflight]] - degree 3, connects to 2 communities
- [[Offline v3 simulation]] - degree 5, connects to 1 community
- [[lqosd inotify file watcher]] - degree 4, connects to 1 community
- [[Last-good topology retention]] - degree 3, connects to 1 community