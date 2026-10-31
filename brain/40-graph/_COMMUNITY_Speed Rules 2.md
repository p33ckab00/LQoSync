---
type: community
members: 22
---

# Speed Rules 2

**Members:** 22 nodes

## Members
- [[ADR-0004 TOML policy, Python defaults]] - document - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[ADR-0007 Fresh design, not a port]] - document - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[All policy in TOML]] - rationale - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Differences from updatecsv.py]] - concept - docs/CONFIGURATION.md
- [[Fresh design, not a port]] - rationale - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Live production values (2026-10-31)]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Profile name number+unit rule]] - rationale - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Python output format for shadow compare]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Random 8-char CircuitDevice IDs]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Router state model (onlinefailedunreachableoffline)]] - rationale - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[Scenario tests (replace golden parity tests)]] - concept - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Semantic compare with live files before cutover]] - concept - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Shadow mode]] - rationale - README.md
- [[ShapedDevices.csv as only state]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Speed source order TOML lists]] - concept - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Static comment rows never removed]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[lqosync-updatecsv.service (Python)]] - concept - README.md
- [[offline_action (default remove)]] - concept - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[offline_after (default 300 s)]] - concept - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[updatecsv.py]] - code - brain/30-knowledge/updatecsv-py-dissection.md
- [[updatecsv.py defects to fix]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[updatecsv.py dissection]] - document - brain/30-knowledge/updatecsv-py-dissection.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Speed_Rules_2
SORT file.name ASC
```

## Connections to other communities
- 16 edges to [[_COMMUNITY_CLI and Checks 2]]
- 6 edges to [[_COMMUNITY_Topology 2]]
- 5 edges to [[_COMMUNITY_Topology 4]]
- 3 edges to [[_COMMUNITY_Speed Rules]]
- 3 edges to [[_COMMUNITY_MikroTik Session 3]]
- 3 edges to [[_COMMUNITY_Status and Diff 4]]
- 1 edge to [[_COMMUNITY_Router Reads 2]]
- 1 edge to [[_COMMUNITY_Community 41]]

## Top bridge nodes
- [[ADR-0007 Fresh design, not a port]] - degree 16, connects to 6 communities
- [[updatecsv.py dissection]] - degree 18, connects to 4 communities
- [[ADR-0004 TOML policy, Python defaults]] - degree 12, connects to 4 communities
- [[Shadow mode]] - degree 6, connects to 2 communities
- [[Differences from updatecsv.py]] - degree 4, connects to 2 communities