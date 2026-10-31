---
type: community
members: 20
---

# Router Reads 2

**Members:** 20 nodes

## Members
- [[CycleSummarynext_sleep (error_retry  active  idle)]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Deferred minor items from Plan 2 review]] - concept - brain/HANDOFF.md
- [[Global constraints (print-only, no secrets in logs, defaults in TOML)]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[LibreQoS 2.x reload (v2_command v2_args, v2_timeout)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Manual shadow run on production shaper (next action)]] - concept - brain/HANDOFF.md
- [[Plan 2b Telegram]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Review Focus (silent router, two processes, crash between writes, bad network.json, SIGHUP bad config)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Runnercycle (lock, read, classify, assemble, write, reload, status)]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Task 9 Step 5 manual shadow read (operator approval)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Write order state.json first, then LibreQoS pair]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[filesload_state (state.json or seed)]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[fileslock (try_lock, 'another lqosync3 is running')]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[filesread_live  Live (static rows, preserve network)]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[filesrun_reload (kill on timeout, last 20 stderr lines; pending retry)]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[fileswrite_if_changed (atomic write)]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[lqosync3 Runtime Implementation Plan (Plan 2)]] - document - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[routerosReadError (Unreachable, Login, Tls, Timeout, Trap, Protocol)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[routerstateclassify]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[run loop with tokioselect! on SIGUSR1, SIGHUP, SIGTERMSIGINT]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[statusStatus  RouterStatus]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Router_Reads_2
SORT file.name ASC
```

## Connections to other communities
- 10 edges to [[_COMMUNITY_MikroTik Session 4]]
- 6 edges to [[_COMMUNITY_Status and Diff 3]]
- 6 edges to [[_COMMUNITY_CLI and Checks 2]]
- 5 edges to [[_COMMUNITY_MikroTik Session 5]]
- 4 edges to [[_COMMUNITY_Router Reads 3]]
- 3 edges to [[_COMMUNITY_Topology 4]]
- 2 edges to [[_COMMUNITY_Status and Diff 4]]
- 2 edges to [[_COMMUNITY_Config 2]]
- 1 edge to [[_COMMUNITY_Speed Rules]]
- 1 edge to [[_COMMUNITY_Speed Rules 2]]

## Top bridge nodes
- [[lqosync3 Runtime Implementation Plan (Plan 2)]] - degree 38, connects to 9 communities
- [[Runnercycle (lock, read, classify, assemble, write, reload, status)]] - degree 12, connects to 2 communities
- [[Review Focus (silent router, two processes, crash between writes, bad network.json, SIGHUP bad config)]] - degree 6, connects to 1 community
- [[Write order state.json first, then LibreQoS pair]] - degree 5, connects to 1 community
- [[fileslock (try_lock, 'another lqosync3 is running')]] - degree 4, connects to 1 community