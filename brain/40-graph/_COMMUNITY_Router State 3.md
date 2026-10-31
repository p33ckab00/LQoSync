---
type: community
members: 9
---

# Router State 3

**Members:** 9 nodes

## Members
- [[ADR-0003 Router failed vs offline]] - rationale - brain/HANDOFF.md
- [[ADR-0008 Terminal UI (lqosync3 tui, Plan 4)]] - rationale - brain/HANDOFF.md
- [[ADR-0009]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Core design spec (2026-10-31-lqosync3-core-design.md)]] - document - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Router state (onlinefailedunreachableoffline)]] - concept - docs/CONFIGURATION.md
- [[Router state classification (RouterResult, unreachable_since)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Runtime design spec (2026-10-31-lqosync3-runtime-design.md)]] - document - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[lqosync3 Runtime Implementation Plan (Plan 2)]] - document - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[routerstateclassify]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Router_State_3
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_MikroTik Session 4]]
- 2 edges to [[_COMMUNITY_Status and Diff 2]]
- 1 edge to [[_COMMUNITY_Telegram Notifier]]
- 1 edge to [[_COMMUNITY_Session Notes]]
- 1 edge to [[_COMMUNITY_Topology 3]]
- 1 edge to [[_COMMUNITY_Speed Rules]]
- 1 edge to [[_COMMUNITY_MikroTik Session 5]]
- 1 edge to [[_COMMUNITY_Telegram Notifier 3]]
- 1 edge to [[_COMMUNITY_Decisions]]

## Top bridge nodes
- [[Router state (onlinefailedunreachableoffline)]] - degree 6, connects to 3 communities
- [[lqosync3 Runtime Implementation Plan (Plan 2)]] - degree 7, connects to 2 communities
- [[ADR-0003 Router failed vs offline]] - degree 5, connects to 2 communities
- [[Router state classification (RouterResult, unreachable_since)]] - degree 3, connects to 2 communities
- [[routerstateclassify]] - degree 4, connects to 1 community