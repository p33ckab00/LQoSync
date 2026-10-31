---
type: community
members: 5
---

# Router Reads 3

**Members:** 5 nodes

## Members
- [[Router paths and .proplist fields table]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Router reads (print only, timeouts, retries, concurrent sessions)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[RouterSession trait (print path proplist)]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[TABLES const (6 paths with .proplist)]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[read_snapshot  read_router (timeouts, retries, session reuse)]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Router_Reads_3
SORT file.name ASC
```

## Connections to other communities
- 4 edges to [[_COMMUNITY_Router Reads 2]]
- 2 edges to [[_COMMUNITY_MikroTik Session 4]]
- 1 edge to [[_COMMUNITY_CLI and Checks 2]]
- 1 edge to [[_COMMUNITY_MikroTik Session 5]]
- 1 edge to [[_COMMUNITY_Speed Rules]]

## Top bridge nodes
- [[read_snapshot  read_router (timeouts, retries, session reuse)]] - degree 7, connects to 3 communities
- [[Router reads (print only, timeouts, retries, concurrent sessions)]] - degree 4, connects to 2 communities
- [[TABLES const (6 paths with .proplist)]] - degree 3, connects to 1 community
- [[Router paths and .proplist fields table]] - degree 3, connects to 1 community
- [[RouterSession trait (print path proplist)]] - degree 2, connects to 1 community