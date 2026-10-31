---
type: community
members: 24
---

# Session Notes

**Members:** 24 nodes

## Members
- [[Applied backups (keep_backups, 12-digit folders)]] - concept - docs/OPERATIONS.md
- [[Cutover from the Python service]] - concept - docs/OPERATIONS.md
- [[Kept-session single reconnect (no TCP keepalive)]] - rationale - brain/HANDOFF.md
- [[LQoSync3 README]] - document - README.md
- [[LQoSync3 sync tool]] - concept - README.md
- [[LibreQoS upgrade from 2.2 to 3.x]] - concept - docs/OPERATIONS.md
- [[Operational rules]] - rationale - docs/OPERATIONS.md
- [[Operations guide]] - document - docs/OPERATIONS.md
- [[Orphan branch lqosync3]] - concept - brain/10-sessions/2026-10-31-updatecsv-dissection.md
- [[Plan 2 global constraints]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Plan phases (Plan 122b done; Plan 3 v3, Plan 4 TUI, Plan 5 installer)]] - concept - brain/HANDOFF.md
- [[Read-convert-validate-write pipeline]] - rationale - README.md
- [[Rollback to the Python service]] - concept - docs/OPERATIONS.md
- [[RouterOS listen command (untested)]] - concept - brain/10-sessions/2026-10-31-updatecsv-dissection.md
- [[Safety rules (shadow default, no router writes, 0600 secrets)]] - rationale - README.md
- [[auto target selection]] - rationale - README.md
- [[diffdiff_pair (semantic pair diff)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[lqosync3 detect (Plan 3)]] - concept - docs/OPERATIONS.md
- [[mikrotik-rs RouterOS API library]] - concept - README.md
- [[no_parent guard on v3 targets]] - rationale - brain/20-decisions/ADR-0006-v3-guards-and-node-types.md
- [[updatecsv.py (Python service)]] - concept - README.md
- [[v2_pair output target]] - concept - README.md
- [[v3_network output target (libreqos_network.json)]] - concept - README.md
- [[v3_pair output target]] - concept - README.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Session_Notes
SORT file.name ASC
```

## Connections to other communities
- 6 edges to [[_COMMUNITY_Status and Diff 2]]
- 4 edges to [[_COMMUNITY_Topology 4]]
- 4 edges to [[_COMMUNITY_Speed Rules]]
- 3 edges to [[_COMMUNITY_Telegram Notifier]]
- 2 edges to [[_COMMUNITY_Topology 5]]
- 2 edges to [[_COMMUNITY_Plan 2 Runtime]]
- 2 edges to [[_COMMUNITY_Telegram Notifier 3]]
- 1 edge to [[_COMMUNITY_Decisions]]
- 1 edge to [[_COMMUNITY_Topology 2]]
- 1 edge to [[_COMMUNITY_Router State 3]]

## Top bridge nodes
- [[Operations guide]] - degree 15, connects to 5 communities
- [[LQoSync3 README]] - degree 13, connects to 2 communities
- [[Plan phases (Plan 122b done; Plan 3 v3, Plan 4 TUI, Plan 5 installer)]] - degree 3, connects to 2 communities
- [[Plan 2 global constraints]] - degree 3, connects to 2 communities
- [[Cutover from the Python service]] - degree 6, connects to 1 community