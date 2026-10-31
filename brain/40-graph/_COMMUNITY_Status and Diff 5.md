---
type: community
members: 6
---

# Status and Diff 5

**Members:** 6 nodes

## Members
- [[ADR-0008 Terminal UI]] - document - brain/20-decisions/ADR-0008-terminal-ui.md
- [[Router credentials in 0600 files]] - rationale - README.md
- [[SIGUSR1 sync now  SIGHUP reload]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[TUI config editor (toml_edit)]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[lqosync3 tui (ratatuicrossterm)]] - rationale - brain/20-decisions/ADR-0008-terminal-ui.md
- [[status.json state file]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Status_and_Diff_5
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Speed Rules 2]]
- 2 edges to [[_COMMUNITY_Topology 2]]
- 1 edge to [[_COMMUNITY_Topology 4]]
- 1 edge to [[_COMMUNITY_Topology 6]]

## Top bridge nodes
- [[ADR-0008 Terminal UI]] - degree 7, connects to 2 communities
- [[TUI config editor (toml_edit)]] - degree 4, connects to 1 community
- [[status.json state file]] - degree 3, connects to 1 community
- [[Router credentials in 0600 files]] - degree 2, connects to 1 community