---
type: community
members: 12
---

# Decisions

**Members:** 12 nodes

## Members
- [[ADR-0002 v3 target selection_1]] - rationale - brain/HANDOFF.md
- [[Backups the tool keeps (applied, keep_backups)]] - concept - docs/OPERATIONS.md
- [[LibreQoS upgrade from 2.2 to 3.x]] - concept - docs/OPERATIONS.md
- [[LibreQoS.py --updateonly (v2 reload)]] - concept - docs/OPERATIONS.md
- [[Operations (docs)]] - document - docs/OPERATIONS.md
- [[Rollback to the Python service]] - concept - docs/OPERATIONS.md
- [[Troubleshooting table]] - concept - docs/OPERATIONS.md
- [[filesbackup (keep_backups)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[filesrun_reload]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[libreqos-v3-preflight]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[libreqos.target (auto  v2_pair  v3_pair  v3_network)]] - concept - docs/CONFIGURATION.md
- [[lqosd (LibreQoS v3 daemon)]] - concept - docs/OPERATIONS.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Decisions
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Status and Diff]]
- 3 edges to [[_COMMUNITY_Mock Router Tests 2]]
- 2 edges to [[_COMMUNITY_Config 2]]
- 2 edges to [[_COMMUNITY_Status and Diff 3]]
- 1 edge to [[_COMMUNITY_MikroTik Session 2]]
- 1 edge to [[_COMMUNITY_Speed Rules 3]]
- 1 edge to [[_COMMUNITY_MikroTik Session 5]]
- 1 edge to [[_COMMUNITY_Topology 4]]
- 1 edge to [[_COMMUNITY_Topology 5]]

## Top bridge nodes
- [[LibreQoS upgrade from 2.2 to 3.x]] - degree 7, connects to 3 communities
- [[Operations (docs)]] - degree 7, connects to 3 communities
- [[libreqos-v3-preflight]] - degree 3, connects to 2 communities
- [[LibreQoS.py --updateonly (v2 reload)]] - degree 4, connects to 1 community
- [[libreqos.target (auto  v2_pair  v3_pair  v3_network)]] - degree 3, connects to 1 community