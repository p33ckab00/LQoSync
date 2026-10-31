---
type: community
cohesion: 0.07
members: 40
---

# Architecture Parts

**Cohesion:** 0.07 - loosely connected
**Members:** 40 nodes

## Members
- [[Applied backups (last 10 versions)]] - concept - docs/OPERATIONS.md
- [[Applier (hash, atomic write, reload)]] - concept - docs/ARCHITECTURE.md
- [[CLI commands (check, detect, once, run)]] - concept - docs/OPERATIONS.md
- [[Circuit Model]] - concept - docs/ARCHITECTURE.md
- [[Collector (RouterOS API reads)]] - concept - docs/ARCHITECTURE.md
- [[Config Loader]] - concept - docs/ARCHITECTURE.md
- [[Cutover from the Python service]] - concept - docs/OPERATIONS.md
- [[LibreQoS upgrade 2.2 to 3.x]] - concept - docs/OPERATIONS.md
- [[LibreQoS v3 topology.compile_mode]] - concept - docs/ARCHITECTURE.md
- [[Manual installation procedure]] - concept - docs/INSTALLATION.md
- [[MikroTik RouterOS API]] - concept - docs/ARCHITECTURE.md
- [[Operational rules]] - rationale - docs/OPERATIONS.md
- [[Performance Rules]] - rationale - docs/ARCHITECTURE.md
- [[Proposed Crate Layout]] - concept - docs/ARCHITECTURE.md
- [[Removal procedure]] - concept - docs/INSTALLATION.md
- [[Requirements (Ubuntu 24.04, LibreQoS 2.0-2.2 or 3.x, Rust 1.88)]] - concept - docs/INSTALLATION.md
- [[Rollback to the Python service]] - concept - docs/OPERATIONS.md
- [[Router State Machine (onlinefailedunreachableoffline)]] - rationale - docs/ARCHITECTURE.md
- [[Runtime modes (shadow  apply)]] - concept - docs/CONFIGURATION.md
- [[Secret files (_file  _env, mode 0600 root)]] - rationale - docs/CONFIGURATION.md
- [[ShapedDevices.csv]] - concept - docs/ARCHITECTURE.md
- [[Six Network Modes]] - concept - docs/ARCHITECTURE.md
- [[State Store (state.json)]] - concept - docs/ARCHITECTURE.md
- [[Target Detector]] - concept - docs/ARCHITECTURE.md
- [[Target v2_pair]] - concept - docs/ARCHITECTURE.md
- [[Target v3_network]] - concept - docs/ARCHITECTURE.md
- [[Target v3_pair]] - concept - docs/ARCHITECTURE.md
- [[Task 7 Pair writer (render_csv, render_network_json)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Topology Builder]] - concept - docs/ARCHITECTURE.md
- [[Troubleshooting table]] - concept - docs/OPERATIONS.md
- [[Validator]] - concept - docs/ARCHITECTURE.md
- [[Writers (one per target)]] - concept - docs/ARCHITECTURE.md
- [[install.sh one-line installer]] - concept - docs/INSTALLATION.md
- [[libreqos_network.json]] - concept - docs/ARCHITECTURE.md
- [[lqosd (LibreQoS daemon)]] - concept - docs/ARCHITECTURE.md
- [[lqosync-updatecsv.service (Python production service)]] - concept - brain/HANDOFF.md
- [[lqosync-updatecsv.service (Python service)]] - concept - docs/OPERATIONS.md
- [[lqosync3.service systemd unit]] - concept - docs/INSTALLATION.md
- [[network.json]] - concept - docs/ARCHITECTURE.md
- [[v2 vs v3 writer differences]] - concept - docs/ARCHITECTURE.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Architecture_Parts
SORT file.name ASC
```

## Connections to other communities
- 5 edges to [[_COMMUNITY_DHCP Discovery and Exclusions]]
- 3 edges to [[_COMMUNITY_Early Sessions and ADRs]]
- 1 edge to [[_COMMUNITY_Scenario Test Harness]]

## Top bridge nodes
- [[Collector (RouterOS API reads)]] - degree 6, connects to 1 community
- [[Target Detector]] - degree 6, connects to 1 community
- [[Runtime modes (shadow  apply)]] - degree 5, connects to 1 community
- [[Cutover from the Python service]] - degree 5, connects to 1 community
- [[Secret files (_file  _env, mode 0600 root)]] - degree 4, connects to 1 community