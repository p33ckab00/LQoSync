---
type: community
cohesion: 0.20
members: 15
---

# v3 Ingress Evidence

**Cohesion:** 0.20 - loosely connected
**Members:** 15 nodes

## Members
- [[runlqosbus 0777 safety caution]] - rationale - docs/LIBREQOS-V3-INGRESS.md
- [[Last-good topology retention]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[LibreQoS v3 ingress document]] - document - docs/LIBREQOS-V3-INGRESS.md
- [[LibreQoS.py --updateonly no-op in v3]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[Offline v3 simulation]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[topology source in lqos.conf]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[liblqos_python.so binding]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[libreqos-v3-preflight]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[libreqos_3.0.7 beta package]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[libreqos_network.json schema 2]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[lqosd (v3 control plane)]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[lqosd inotify file watcher]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[topology_import.json (internal)]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[v3 offline simulation method]] - document - brain/30-knowledge/v3-offline-simulation-method.md
- [[v3 schema validation results]] - concept - docs/LIBREQOS-V3-INGRESS.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/v3_Ingress_Evidence
SORT file.name ASC
```

## Connections to other communities
- 12 edges to [[_COMMUNITY_Early Sessions and ADRs]]
- 1 edge to [[_COMMUNITY_Scenario Test Harness]]
- 1 edge to [[_COMMUNITY_Router State Decisions]]

## Top bridge nodes
- [[lqosd inotify file watcher]] - degree 7, connects to 2 communities
- [[v3 offline simulation method]] - degree 5, connects to 2 communities
- [[LibreQoS v3 ingress document]] - degree 17, connects to 1 community
- [[libreqos_3.0.7 beta package]] - degree 3, connects to 1 community
- [[libreqos-v3-preflight]] - degree 2, connects to 1 community