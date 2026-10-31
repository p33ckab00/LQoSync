---
type: community
members: 6
---

# Status and Diff 4

**Members:** 6 nodes

## Members
- [[ADR-0008 Terminal UI]] - document - brain/20-decisions/ADR-0008-terminal-ui.md
- [[Delivery in five plans (core, runtime, v3, TUI, install)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[SIGUSR1 sync now  SIGHUP reload]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[TUI config editor (toml_edit)]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[lqosync3 tui (ratatuicrossterm)]] - rationale - brain/20-decisions/ADR-0008-terminal-ui.md
- [[status.json state file]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Status_and_Diff_4
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Speed Rules 2]]
- 1 edge to [[_COMMUNITY_Topology 3]]
- 1 edge to [[_COMMUNITY_Topology 5]]

## Top bridge nodes
- [[Delivery in five plans (core, runtime, v3, TUI, install)]] - degree 3, connects to 2 communities
- [[ADR-0008 Terminal UI]] - degree 6, connects to 1 community
- [[TUI config editor (toml_edit)]] - degree 3, connects to 1 community
- [[status.json state file]] - degree 3, connects to 1 community