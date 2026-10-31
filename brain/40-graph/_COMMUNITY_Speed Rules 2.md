---
type: community
members: 17
---

# Speed Rules 2

**Members:** 17 nodes

## Members
- [[ADR-0008 Terminal UI]] - document - brain/20-decisions/ADR-0008-terminal-ui.md
- [[All policy in TOML]] - rationale - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Fresh design, not a port]] - rationale - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Live production values (2026-10-31)]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Profile name number+unit rule]] - rationale - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Python output format for shadow compare]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Router state model (onlinefailedunreachableoffline)]] - rationale - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[SIGUSR1 sync now  SIGHUP reload]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[Speed source order TOML lists]] - concept - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[TUI config editor (toml_edit)]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[Temp file then rename write]] - rationale - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[lqosync3 tui (ratatuicrossterm)]] - rationale - brain/20-decisions/ADR-0008-terminal-ui.md
- [[offline_action (default remove)]] - concept - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[offline_after (default 300 s)]] - concept - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[status.json state file]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[updatecsv.py]] - code - brain/30-knowledge/updatecsv-py-dissection.md
- [[updatecsv.py defects to fix]] - concept - brain/30-knowledge/updatecsv-py-dissection.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Speed_Rules_2
SORT file.name ASC
```

## Connections to other communities
- 14 edges to [[_COMMUNITY_Community 36]]
- 2 edges to [[_COMMUNITY_Topology 2]]
- 1 edge to [[_COMMUNITY_Status and Diff 2]]

## Top bridge nodes
- [[ADR-0008 Terminal UI]] - degree 8, connects to 2 communities
- [[updatecsv.py]] - degree 6, connects to 1 community
- [[updatecsv.py defects to fix]] - degree 5, connects to 1 community
- [[Router state model (onlinefailedunreachableoffline)]] - degree 5, connects to 1 community
- [[All policy in TOML]] - degree 5, connects to 1 community