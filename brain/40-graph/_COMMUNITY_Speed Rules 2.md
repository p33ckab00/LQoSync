---
type: community
members: 21
---

# Speed Rules 2

**Members:** 21 nodes

## Members
- [[ADR-0003 Router failed vs offline]] - document - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[ADR-0004 TOML policy, Python defaults_1]] - document - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[All policy in TOML]] - rationale - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Fresh design, not a port]] - rationale - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Live production values (2026-10-31)]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Profile name number+unit rule]] - rationale - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Python output format for shadow compare]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Random 8-char CircuitDevice IDs]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Router state model (onlinefailedunreachableoffline)]] - rationale - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[Semantic compare with live files before cutover]] - concept - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Shadow mode]] - rationale - README.md
- [[ShapedDevices.csv as only state]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Speed source order TOML lists]] - concept - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Static comment rows never removed]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Temp file then rename write]] - rationale - brain/30-knowledge/libreqos-v3-ingress-facts.md
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
- 11 edges to [[_COMMUNITY_Topology 4]]
- 7 edges to [[_COMMUNITY_Topology 2]]
- 3 edges to [[_COMMUNITY_Topology 3]]
- 2 edges to [[_COMMUNITY_Topology 6]]
- 2 edges to [[_COMMUNITY_Speed Rules 4]]
- 2 edges to [[_COMMUNITY_Status and Diff 5]]
- 1 edge to [[_COMMUNITY_Router State 2]]
- 1 edge to [[_COMMUNITY_Topology 5]]

## Top bridge nodes
- [[ADR-0004 TOML policy, Python defaults_1]] - degree 11, connects to 4 communities
- [[updatecsv.py dissection]] - degree 15, connects to 3 communities
- [[ADR-0003 Router failed vs offline]] - degree 11, connects to 3 communities
- [[Temp file then rename write]] - degree 4, connects to 3 communities
- [[Shadow mode]] - degree 6, connects to 2 communities