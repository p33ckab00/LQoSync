---
type: community
members: 7
---

# Router State 2

**Members:** 7 nodes

## Members
- [[ADR-0003 Router failed vs offline_1]] - rationale - brain/HANDOFF.md
- [[Router state (online  failed  unreachable  offline)]] - concept - docs/CONFIGURATION.md
- [[Router state classification (RouterResult, unreachable_since)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[RouterOS listen change detection (future)]] - concept - docs/ARCHITECTURE.md
- [[Subscriber presence and subscriber_grace]] - concept - docs/CONFIGURATION.md
- [[router_access timeouts, retries, offline_after]] - concept - docs/CONFIGURATION.md
- [[routerstateclassify]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Router_State_2
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Status and Diff]]
- 2 edges to [[_COMMUNITY_Topology 2]]
- 1 edge to [[_COMMUNITY_MikroTik Session 2]]
- 1 edge to [[_COMMUNITY_Speed Rules 2]]
- 1 edge to [[_COMMUNITY_Speed Rules 3]]
- 1 edge to [[_COMMUNITY_Mock Router Tests 2]]
- 1 edge to [[_COMMUNITY_MikroTik Session 4]]
- 1 edge to [[_COMMUNITY_MikroTik Session 5]]

## Top bridge nodes
- [[routerstateclassify]] - degree 4, connects to 2 communities
- [[Router state classification (RouterResult, unreachable_since)]] - degree 3, connects to 2 communities
- [[Router state (online  failed  unreachable  offline)]] - degree 6, connects to 1 community
- [[ADR-0003 Router failed vs offline_1]] - degree 4, connects to 1 community
- [[Subscriber presence and subscriber_grace]] - degree 3, connects to 1 community