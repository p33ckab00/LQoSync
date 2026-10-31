---
type: community
members: 14
---

# MikroTik Session 5

**Members:** 14 nodes

## Members
- [[Config loader validation rules]] - concept - docs/CONFIGURATION.md
- [[Configuration (docs)]] - document - docs/CONFIGURATION.md
- [[Manual installation]] - concept - docs/INSTALLATION.md
- [[RouterCfg.tls_verify  tls_ca_file]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[RouterSession  Connector traits]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Secret files (_file keys, mode 0600, owner root)]] - concept - docs/CONFIGURATION.md
- [[Secrets (password_file read each connect, never logged)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[mikrotik-rs =0.9.0 (mikrotik-tokio 0.3.0)_1]] - concept - brain/HANDOFF.md
- [[mikrotikMikrotikConnector  MikrotikSession]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[mikrotiktls_client_config (rustls, webpki-roots)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[routerosReadError]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[routerosread_router (connect, retries)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[routerosread_snapshot (TABLES, api_timeout)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[secretsread_secret]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/MikroTik_Session_5
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Speed Rules 3]]
- 3 edges to [[_COMMUNITY_Mock Router Tests 2]]
- 3 edges to [[_COMMUNITY_Config 2]]
- 1 edge to [[_COMMUNITY_Status and Diff]]
- 1 edge to [[_COMMUNITY_MikroTik Session 4]]
- 1 edge to [[_COMMUNITY_Status and Diff 3]]
- 1 edge to [[_COMMUNITY_Decisions]]
- 1 edge to [[_COMMUNITY_Router State 2]]

## Top bridge nodes
- [[Secret files (_file keys, mode 0600, owner root)]] - degree 6, connects to 2 communities
- [[Manual installation]] - degree 4, connects to 2 communities
- [[Config loader validation rules]] - degree 3, connects to 2 communities
- [[Configuration (docs)]] - degree 3, connects to 2 communities
- [[mikrotikMikrotikConnector  MikrotikSession]] - degree 6, connects to 1 community