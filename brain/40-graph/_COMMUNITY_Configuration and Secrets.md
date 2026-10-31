---
type: community
cohesion: 0.40
members: 6
---

# Configuration and Secrets

**Cohesion:** 0.40 - moderately connected
**Members:** 6 nodes

## Members
- [[etclqosyncupdatecsv.toml]] - concept - docs/CONFIGURATION.md
- [[Config loader]] - concept - docs/ARCHITECTURE.md
- [[Configuration document]] - document - docs/CONFIGURATION.md
- [[Read-only router access rule]] - rationale - README.md
- [[Secret files (_file, mode 0600 root)]] - rationale - docs/CONFIGURATION.md
- [[import-json command (planned)]] - concept - docs/CONFIGURATION.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Configuration_and_Secrets
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Sync Pipeline Internals]]
- 2 edges to [[_COMMUNITY_Brain Vault Notes]]
- 2 edges to [[_COMMUNITY_Installation]]
- 2 edges to [[_COMMUNITY_Speed Rules and Open Decisions]]
- 1 edge to [[_COMMUNITY_Operations and Cutover]]
- 1 edge to [[_COMMUNITY_Output Target Selection]]
- 1 edge to [[_COMMUNITY_Runtime Mode]]

## Top bridge nodes
- [[Configuration document]] - degree 11, connects to 6 communities
- [[Config loader]] - degree 3, connects to 1 community
- [[import-json command (planned)]] - degree 2, connects to 1 community
- [[Read-only router access rule]] - degree 2, connects to 1 community