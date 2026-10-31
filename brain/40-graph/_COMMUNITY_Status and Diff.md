---
type: community
members: 20
---

# Status and Diff

**Members:** 20 nodes

## Members
- [[ADR-0001 One model, many writers]] - rationale - brain/HANDOFF.md
- [[ADR-0005 Topology follows v3]] - rationale - brain/HANDOFF.md
- [[ADR-0007 Fresh design, not a port]] - rationale - brain/HANDOFF.md
- [[ADR-0009]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Core design spec (2026-10-31-lqosync3-core-design.md)]] - document - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Differences from updatecsv.py]] - concept - docs/CONFIGURATION.md
- [[Handoff 2026-10-31 (documentation phase)]] - document - brain/HANDOFF.md
- [[Operational rules]] - rationale - docs/OPERATIONS.md
- [[Plan 2 global constraints]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Plan 2b Telegram]] - concept - brain/HANDOFF.md
- [[Plan 3 LibreQoS v3 detection and writers]] - concept - brain/HANDOFF.md
- [[Runtime design spec (2026-10-31-lqosync3-runtime-design.md)]] - document - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Speed rule decision (routeros_rx_tx, unitless bps)]] - rationale - brain/HANDOFF.md
- [[telegram notification settings]] - concept - docs/CONFIGURATION.md
- [[libreqos-v3-ingress-facts (vault note)]] - document - brain/HANDOFF.md
- [[lqosync3 Runtime Implementation Plan (Plan 2)]] - document - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[lqosync3 detect (Plan 3)]] - concept - docs/OPERATIONS.md
- [[rate_limit_order and unitless_unit]] - concept - docs/CONFIGURATION.md
- [[toolsv3simcheck.sh (v3 3.0.7 compiler check)]] - concept - brain/HANDOFF.md
- [[v3-offline-simulation-method (vault note)]] - document - brain/HANDOFF.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Status_and_Diff
SORT file.name ASC
```

## Connections to other communities
- 8 edges to [[_COMMUNITY_Speed Rules 3]]
- 5 edges to [[_COMMUNITY_Status and Diff 3]]
- 4 edges to [[_COMMUNITY_Config 2]]
- 3 edges to [[_COMMUNITY_Decisions]]
- 2 edges to [[_COMMUNITY_Router State 2]]
- 1 edge to [[_COMMUNITY_MikroTik Session 2]]
- 1 edge to [[_COMMUNITY_MikroTik Session 5]]
- 1 edge to [[_COMMUNITY_Mock Router Tests 2]]

## Top bridge nodes
- [[Handoff 2026-10-31 (documentation phase)]] - degree 19, connects to 6 communities
- [[lqosync3 Runtime Implementation Plan (Plan 2)]] - degree 14, connects to 6 communities
- [[Differences from updatecsv.py]] - degree 6, connects to 3 communities
- [[lqosync3 detect (Plan 3)]] - degree 3, connects to 2 communities
- [[Plan 2 global constraints]] - degree 4, connects to 1 community