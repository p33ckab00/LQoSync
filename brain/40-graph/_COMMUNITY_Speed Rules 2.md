---
type: community
members: 26
---

# Speed Rules 2

**Members:** 26 nodes

## Members
- [[ADR-0003 Router failed vs offline]] - document - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[ADR-0004 TOML policy, Python defaults]] - document - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[ADR-0007 Fresh design, not a port]] - document - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[ADR-0008 Terminal UI]] - document - brain/20-decisions/ADR-0008-terminal-ui.md
- [[All policy in TOML]] - rationale - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Fresh design, not a port]] - rationale - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Live production values (2026-10-31)]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Profile name number+unit rule]] - rationale - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Python output format for shadow compare]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Random 8-char CircuitDevice IDs]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Router state model (onlinefailedunreachableoffline)]] - rationale - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[SIGUSR1 sync now  SIGHUP reload]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[Scenario tests (replace golden parity tests)]] - concept - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Semantic compare with live files before cutover]] - concept - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[ShapedDevices.csv as only state]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Speed source order TOML lists]] - concept - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Static comment rows never removed]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[TUI config editor (toml_edit)]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[Temp file then rename write]] - rationale - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[lqosync3 tui (ratatuicrossterm)]] - rationale - brain/20-decisions/ADR-0008-terminal-ui.md
- [[offline_action (default remove)]] - concept - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[offline_after (default 300 s)]] - concept - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[status.json state file]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[updatecsv.py]] - code - brain/30-knowledge/updatecsv-py-dissection.md
- [[updatecsv.py defects to fix]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[updatecsv.py dissection]] - document - brain/30-knowledge/updatecsv-py-dissection.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Speed_Rules_2
SORT file.name ASC
```

## Connections to other communities
- 16 edges to [[_COMMUNITY_Topology 2]]
- 12 edges to [[_COMMUNITY_Community 36]]
- 3 edges to [[_COMMUNITY_Speed Rules 3]]
- 1 edge to [[_COMMUNITY_Community 16]]
- 1 edge to [[_COMMUNITY_Community 30]]
- 1 edge to [[_COMMUNITY_Community 26]]

## Top bridge nodes
- [[ADR-0007 Fresh design, not a port]] - degree 14, connects to 4 communities
- [[ADR-0003 Router failed vs offline]] - degree 13, connects to 4 communities
- [[updatecsv.py dissection]] - degree 15, connects to 3 communities
- [[ADR-0004 TOML policy, Python defaults]] - degree 12, connects to 3 communities
- [[ADR-0008 Terminal UI]] - degree 8, connects to 2 communities