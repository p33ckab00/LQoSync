---
type: community
members: 35
---

# Session Notes

**Members:** 35 nodes

## Members
- [[1415 CSV columns (v2_pairv3_pair)]] - concept - brain/20-decisions/ADR-0005-topology-follows-v3.md
- [[Applied backups (keep_backups, 12-digit folders)]] - concept - docs/OPERATIONS.md
- [[Checks after installation]] - concept - docs/INSTALLATION.md
- [[Cutover from the Python service]] - concept - docs/OPERATIONS.md
- [[Installation (docs)]] - document - docs/INSTALLATION.md
- [[Installer options (--mode, --version, --from-source, --import-json, --no-start)]] - concept - docs/INSTALLATION.md
- [[Kept-session single reconnect (no TCP keepalive)]] - rationale - brain/HANDOFF.md
- [[LQoSync3 README]] - document - README.md
- [[LQoSync3 sync tool]] - concept - README.md
- [[LibreQoS upgrade from 2.2 to 3.x]] - concept - docs/OPERATIONS.md
- [[Manual installation]] - concept - docs/INSTALLATION.md
- [[Operational rules]] - rationale - docs/OPERATIONS.md
- [[Operations guide]] - document - docs/OPERATIONS.md
- [[Orphan branch lqosync3]] - concept - brain/10-sessions/2026-10-31-updatecsv-dissection.md
- [[Plan 2 global constraints]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Plan phases (Plan 122b done; Plan 3 v3, Plan 4 TUI, Plan 5 installer)]] - concept - brain/HANDOFF.md
- [[Read-convert-validate-write pipeline]] - rationale - README.md
- [[Removal procedure]] - concept - docs/INSTALLATION.md
- [[Requirements (Ubuntu 24.04, LibreQoS 2.0-2.2 or 3.x, Rust 1.89)]] - concept - docs/INSTALLATION.md
- [[Rollback to the Python service]] - concept - docs/OPERATIONS.md
- [[RouterOS listen command (untested)]] - concept - brain/10-sessions/2026-10-31-updatecsv-dissection.md
- [[Runtime modes (shadow, apply)]] - concept - docs/CONFIGURATION.md
- [[Safety rules (shadow default, no router writes, 0600 secrets)]] - rationale - README.md
- [[What the installer does (10 steps)]] - concept - docs/INSTALLATION.md
- [[auto target selection]] - rationale - README.md
- [[diffdiff_pair (semantic pair diff)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[install.sh one-line installer (target contract)]] - concept - docs/INSTALLATION.md
- [[libreqos_network.json schema_version 2]] - concept - brain/30-knowledge/libreqos-v3-ingress-facts.md
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
- 13 edges to [[_COMMUNITY_Topology 2]]
- 6 edges to [[_COMMUNITY_Community 26]]
- 5 edges to [[_COMMUNITY_Community 16]]
- 3 edges to [[_COMMUNITY_Community 36]]
- 3 edges to [[_COMMUNITY_Community 18]]
- 1 edge to [[_COMMUNITY_MikroTik Session 5]]
- 1 edge to [[_COMMUNITY_Community 30]]

## Top bridge nodes
- [[Operations guide]] - degree 16, connects to 4 communities
- [[LQoSync3 README]] - degree 16, connects to 3 communities
- [[Runtime modes (shadow, apply)]] - degree 4, connects to 2 communities
- [[Plan phases (Plan 122b done; Plan 3 v3, Plan 4 TUI, Plan 5 installer)]] - degree 3, connects to 2 communities
- [[Plan 2 global constraints]] - degree 3, connects to 2 communities