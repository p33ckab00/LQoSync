---
type: community
members: 19
---

# MikroTik Session 4

**Members:** 19 nodes

## Members
- [[Plan 2 review focus (5 risks)]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[RouterCfg.tls_verify  tls_ca_file]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[RouterSession  Connector traits]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[cycleassemble (pure core)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[filesbackup (keep_backups)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[filesload_state (state.json or seed)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[fileslock (lqosync3.lock)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[filesread_live  Live]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[filesrun_reload]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[fileswrite_if_changed (atomic write)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[mikrotikMikrotikConnector  MikrotikSession]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[mikrotiktls_client_config (rustls, webpki-roots)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[network.json]] - concept - docs/ARCHITECTURE.md
- [[routerosReadError]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[routerosread_router (connect, retries)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[routerosread_snapshot (TABLES, api_timeout)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[runnerRunnercycle (one full cycle)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[secretsread_secret]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[testsmock_router.rs (mock RouterOS server)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/MikroTik_Session_4
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Router State 3]]
- 2 edges to [[_COMMUNITY_Plan 2 Runtime]]
- 1 edge to [[_COMMUNITY_MikroTik Session 3]]
- 1 edge to [[_COMMUNITY_Telegram Notifier 3]]

## Top bridge nodes
- [[runnerRunnercycle (one full cycle)]] - degree 12, connects to 3 communities
- [[Plan 2 review focus (5 risks)]] - degree 4, connects to 1 community
- [[routerosReadError]] - degree 3, connects to 1 community
- [[network.json]] - degree 2, connects to 1 community