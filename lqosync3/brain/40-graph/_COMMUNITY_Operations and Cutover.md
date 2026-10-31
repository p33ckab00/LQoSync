---
type: community
cohesion: 0.20
members: 11
---

# Operations and Cutover

**Cohesion:** 0.20 - loosely connected
**Members:** 11 nodes

## Members
- [[Applied version backups (last 10)]] - concept - docs/OPERATIONS.md
- [[Command reference (check, detect, once, run)]] - concept - docs/OPERATIONS.md
- [[Cutover from the Python service]] - concept - docs/OPERATIONS.md
- [[LQoSync UpdateCSV (Rust tool)]] - concept - README.md
- [[Map from config.json to TOML]] - concept - docs/CONFIGURATION.md
- [[Operational rules]] - rationale - docs/OPERATIONS.md
- [[Operations document]] - document - docs/OPERATIONS.md
- [[Rollback to the Python service]] - concept - docs/OPERATIONS.md
- [[Troubleshooting table]] - concept - docs/OPERATIONS.md
- [[mikrotik-rs RouterOS API crate]] - concept - README.md
- [[updatecsv.py (Python predecessor)]] - concept - README.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Operations_and_Cutover
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Output Target Selection]]
- 3 edges to [[_COMMUNITY_Brain Vault Notes]]
- 2 edges to [[_COMMUNITY_Sync Pipeline Internals]]
- 1 edge to [[_COMMUNITY_Configuration and Secrets]]
- 1 edge to [[_COMMUNITY_Installation]]
- 1 edge to [[_COMMUNITY_Runtime Mode]]
- 1 edge to [[_COMMUNITY_LibreQoS v3 Runtime Evidence]]

## Top bridge nodes
- [[Operations document]] - degree 8, connects to 2 communities
- [[Cutover from the Python service]] - degree 5, connects to 2 communities
- [[updatecsv.py (Python predecessor)]] - degree 5, connects to 2 communities
- [[mikrotik-rs RouterOS API crate]] - degree 3, connects to 2 communities
- [[LQoSync UpdateCSV (Rust tool)]] - degree 3, connects to 1 community