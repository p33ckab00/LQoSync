---
type: community
cohesion: 0.13
members: 21
---

# Targets and Applier

**Cohesion:** 0.13 - loosely connected
**Members:** 21 nodes

## Members
- [[Applied backups (last 10 versions)]] - concept - docs/OPERATIONS.md
- [[Applier (hash, atomic write, reload)]] - concept - docs/ARCHITECTURE.md
- [[CLI commands (check, detect, once, run)]] - concept - docs/OPERATIONS.md
- [[Cutover from the Python service]] - concept - docs/OPERATIONS.md
- [[Delivery in five plans (Core, Runtime, v3, TUI, Install)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[LibreQoS upgrade 2.2 to 3.x]] - concept - docs/OPERATIONS.md
- [[Map from config.json]] - concept - docs/CONFIGURATION.md
- [[Operational rules]] - rationale - docs/OPERATIONS.md
- [[Removal procedure]] - concept - docs/INSTALLATION.md
- [[Requirements (Ubuntu 24.04, LibreQoS 2.0-2.2 or 3.x, Rust 1.88)]] - concept - docs/INSTALLATION.md
- [[Rollback to the Python service]] - concept - docs/OPERATIONS.md
- [[Runtime modes (shadow  apply)]] - concept - docs/CONFIGURATION.md
- [[Target Detector]] - concept - docs/ARCHITECTURE.md
- [[Target v3_network]] - concept - docs/ARCHITECTURE.md
- [[Target v3_pair]] - concept - docs/ARCHITECTURE.md
- [[install.sh one-line installer]] - concept - docs/INSTALLATION.md
- [[libreqos_network.json]] - concept - docs/ARCHITECTURE.md
- [[lqosd (LibreQoS daemon)]] - concept - docs/ARCHITECTURE.md
- [[lqosync-updatecsv.service (Python service)]] - concept - docs/OPERATIONS.md
- [[lqosync3 diff semantic compare]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[lqosync3 import-json command (planned)]] - concept - docs/CONFIGURATION.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Targets_and_Applier
SORT file.name ASC
```

## Connections to other communities
- 5 edges to [[_COMMUNITY_Output Files and State Store]]
- 3 edges to [[_COMMUNITY_Decisions and Session Notes]]
- 3 edges to [[_COMMUNITY_Config Rules and Modes]]
- 1 edge to [[_COMMUNITY_Collector and Router State]]

## Top bridge nodes
- [[Target Detector]] - degree 7, connects to 2 communities
- [[install.sh one-line installer]] - degree 7, connects to 1 community
- [[Cutover from the Python service]] - degree 6, connects to 1 community
- [[LibreQoS upgrade 2.2 to 3.x]] - degree 5, connects to 1 community
- [[Applier (hash, atomic write, reload)]] - degree 4, connects to 1 community