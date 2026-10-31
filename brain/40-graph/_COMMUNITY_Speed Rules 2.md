---
type: community
members: 17
---

# Speed Rules 2

**Members:** 17 nodes

## Members
- [[ADR-0004 TOML policy, Python defaults]] - document - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[ADR-0007 Fresh design, not a port]] - document - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[All policy in TOML]] - rationale - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Fresh design, not a port]] - rationale - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Live production values (2026-10-31)]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Profile name number+unit rule]] - rationale - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Python output format for shadow compare]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Random 8-char CircuitDevice IDs]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Scenario tests (replace golden parity tests)]] - concept - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Semantic compare with live files before cutover]] - concept - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[ShapedDevices.csv as only state]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Speed source order TOML lists]] - concept - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Static comment rows never removed]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Temp file then rename write]] - rationale - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[updatecsv.py]] - code - brain/30-knowledge/updatecsv-py-dissection.md
- [[updatecsv.py defects to fix]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[updatecsv.py dissection]] - document - brain/30-knowledge/updatecsv-py-dissection.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Speed_Rules_2
SORT file.name ASC
```

## Connections to other communities
- 5 edges to [[_COMMUNITY_Decisions]]
- 4 edges to [[_COMMUNITY_Topology 4]]
- 4 edges to [[_COMMUNITY_Topology 5]]
- 3 edges to [[_COMMUNITY_Topology 3]]
- 3 edges to [[_COMMUNITY_Speed Rules 3]]
- 3 edges to [[_COMMUNITY_Status and Diff 4]]
- 2 edges to [[_COMMUNITY_Topology 2]]

## Top bridge nodes
- [[ADR-0007 Fresh design, not a port]] - degree 11, connects to 5 communities
- [[ADR-0004 TOML policy, Python defaults]] - degree 10, connects to 5 communities
- [[updatecsv.py dissection]] - degree 14, connects to 4 communities
- [[Temp file then rename write]] - degree 4, connects to 2 communities
- [[updatecsv.py]] - degree 6, connects to 1 community