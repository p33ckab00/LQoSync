---
type: community
members: 13
---

# MikroTik Session 4

**Members:** 13 nodes

## Members
- [[Atomic write (temp file, sync_all, rename, fsync folder; only on byte change)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Files lock, state.json, status.json, candidate, applied]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[First-run seed (seed_state, seed_nodes from live files)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Instance lock statelqosync3.lock (Filetry_lock)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Mock RouterOS server tests (testsmock_router.rs)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Plan 2 scope (in  not in)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Router library mikrotik-rs =0.9.0 in this project only]] - concept - brain/HANDOFF.md
- [[Test strategy (fake session, mock RouterOS server, temp folders)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[lqosync3 runtime design (Plan 2)]] - document - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[mikrotik-rs =0.9.0 (mikrotik-tokio 0.3.0)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[mikrotik-tokio 0.1.0 16-reply burst cancel bug]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[rust-version 1.89 (stdfsFiletry_lock)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[status.json status file]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/MikroTik_Session_4
SORT file.name ASC
```

## Connections to other communities
- 10 edges to [[_COMMUNITY_Router Reads 2]]
- 3 edges to [[_COMMUNITY_CLI and Checks 2]]
- 2 edges to [[_COMMUNITY_Config 2]]
- 2 edges to [[_COMMUNITY_Status and Diff 3]]
- 2 edges to [[_COMMUNITY_Router Reads 3]]
- 2 edges to [[_COMMUNITY_MikroTik Session 5]]
- 1 edge to [[_COMMUNITY_Topology 4]]

## Top bridge nodes
- [[lqosync3 runtime design (Plan 2)]] - degree 19, connects to 6 communities
- [[Files lock, state.json, status.json, candidate, applied]] - degree 8, connects to 3 communities
- [[Mock RouterOS server tests (testsmock_router.rs)]] - degree 4, connects to 1 community
- [[Instance lock statelqosync3.lock (Filetry_lock)]] - degree 4, connects to 1 community
- [[mikrotik-rs =0.9.0 (mikrotik-tokio 0.3.0)]] - degree 4, connects to 1 community