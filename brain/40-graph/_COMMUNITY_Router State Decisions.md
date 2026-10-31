---
type: community
cohesion: 0.16
members: 22
---

# Router State Decisions

**Cohesion:** 0.16 - loosely connected
**Members:** 22 nodes

## Members
- [[ADR-0003 Router failed vs offline]] - document - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[ADR-0004 TOML policy, Python defaults]] - document - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[All policy in TOML]] - rationale - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Fresh design, not a port]] - rationale - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Live production values (2026-10-31)]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Policy in TOML, formats in code]] - rationale - docs/CONFIGURATION.md
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
TABLE source_file, type FROM #community/Router_State_Decisions
SORT file.name ASC
```

## Connections to other communities
- 14 edges to [[_COMMUNITY_Early Sessions and ADRs]]
- 8 edges to [[_COMMUNITY_Plan 1 Session and Review]]
- 6 edges to [[_COMMUNITY_DHCP Discovery and Exclusions]]
- 3 edges to [[_COMMUNITY_Scenario Test Harness]]
- 2 edges to [[_COMMUNITY_Terminal UI]]
- 1 edge to [[_COMMUNITY_v3 Ingress Evidence]]

## Top bridge nodes
- [[ADR-0004 TOML policy, Python defaults]] - degree 13, connects to 4 communities
- [[updatecsv.py dissection]] - degree 17, connects to 3 communities
- [[ADR-0003 Router failed vs offline]] - degree 12, connects to 3 communities
- [[Temp file then rename write]] - degree 4, connects to 3 communities
- [[Shadow mode]] - degree 6, connects to 2 communities