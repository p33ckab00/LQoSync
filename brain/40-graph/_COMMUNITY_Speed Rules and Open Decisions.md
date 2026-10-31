---
type: community
cohesion: 0.40
members: 5
---

# Speed Rules and Open Decisions

**Cohesion:** 0.40 - moderately connected
**Members:** 5 nodes

## Members
- [[Backstop caps]] - concept - docs/CONFIGURATION.md
- [[Open item speed rule direction and units]] - concept - docs/CONFIGURATION.md
- [[Operator decisions pending (speed rules, crate location, repo URL)]] - concept - brain/HANDOFF.md
- [[Proposed crate layout]] - concept - docs/ARCHITECTURE.md
- [[Speed rules]] - concept - docs/ARCHITECTURE.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Speed_Rules_and_Open_Decisions
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Sync Pipeline Internals]]
- 2 edges to [[_COMMUNITY_Configuration and Secrets]]
- 1 edge to [[_COMMUNITY_Brain Vault Notes]]

## Top bridge nodes
- [[Operator decisions pending (speed rules, crate location, repo URL)]] - degree 3, connects to 1 community
- [[Speed rules]] - degree 3, connects to 1 community
- [[Open item speed rule direction and units]] - degree 3, connects to 1 community
- [[Proposed crate layout]] - degree 2, connects to 1 community
- [[Backstop caps]] - degree 2, connects to 1 community