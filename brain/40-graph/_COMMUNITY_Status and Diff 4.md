---
type: community
members: 9
---

# Status and Diff 4

**Members:** 9 nodes

## Members
- [[ADR-0008 Terminal UI]] - document - brain/20-decisions/ADR-0008-terminal-ui.md
- [[Delivery in five plans (core, runtime, v3, TUI, install)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Plan 4 TUI]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Router credentials in 0600 files]] - rationale - README.md
- [[SIGUSR1 sync now  SIGHUP reload]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[TUI config editor (toml_edit)]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[Temp file then rename write]] - rationale - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[lqosync3 tui (ratatuicrossterm)]] - rationale - brain/20-decisions/ADR-0008-terminal-ui.md
- [[status.json state file]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Status_and_Diff_4
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Topology 3]]
- 3 edges to [[_COMMUNITY_Speed Rules 2]]
- 2 edges to [[_COMMUNITY_Topology 4]]
- 2 edges to [[_COMMUNITY_Router Reads 2]]
- 1 edge to [[_COMMUNITY_MikroTik Session 3]]
- 1 edge to [[_COMMUNITY_CLI and Checks 2]]

## Top bridge nodes
- [[ADR-0008 Terminal UI]] - degree 10, connects to 4 communities
- [[Temp file then rename write]] - degree 4, connects to 2 communities
- [[TUI config editor (toml_edit)]] - degree 4, connects to 1 community
- [[Delivery in five plans (core, runtime, v3, TUI, install)]] - degree 3, connects to 1 community
- [[Plan 4 TUI]] - degree 2, connects to 1 community