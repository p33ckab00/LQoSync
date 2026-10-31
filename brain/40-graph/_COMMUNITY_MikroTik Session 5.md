---
type: community
members: 7
---

# MikroTik Session 5

**Members:** 7 nodes

## Members
- [[Connector trait (connect - Session)]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[MikrotikConnector  MikrotikSession  tls_client_config]] - code - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[RouterCfg tls_verify  tls_ca_file]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[Secret files (_file  _env, mode 0600, owner root)]] - rationale - docs/CONFIGURATION.md
- [[Secrets (password_file read each connect, never logged)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Troubleshooting table]] - concept - docs/OPERATIONS.md
- [[secretsread_secret (mode & 0o077 == 0, owner uid)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/MikroTik_Session_5
SORT file.name ASC
```

## Connections to other communities
- 5 edges to [[_COMMUNITY_Router Reads 2]]
- 2 edges to [[_COMMUNITY_MikroTik Session 4]]
- 2 edges to [[_COMMUNITY_CLI and Checks 2]]
- 1 edge to [[_COMMUNITY_Router Reads 3]]
- 1 edge to [[_COMMUNITY_Topology 2]]
- 1 edge to [[_COMMUNITY_Status and Diff 3]]

## Top bridge nodes
- [[MikrotikConnector  MikrotikSession  tls_client_config]] - degree 6, connects to 2 communities
- [[Connector trait (connect - Session)]] - degree 3, connects to 2 communities
- [[Troubleshooting table]] - degree 3, connects to 2 communities
- [[RouterCfg tls_verify  tls_ca_file]] - degree 3, connects to 2 communities
- [[secretsread_secret (mode & 0o077 == 0, owner uid)]] - degree 3, connects to 1 community