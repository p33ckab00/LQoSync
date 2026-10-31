---
type: community
members: 29
---

# CLI and Checks 2

**Members:** 29 nodes

## Members
- [[etclqosync3lqosync3.toml config file]] - concept - docs/CONFIGURATION.md
- [[ADR-0003 Router failed vs offline]] - document - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[ADR-0004 TOML policy, Python defaults_1]] - rationale - brain/HANDOFF.md
- [[Architecture (LQoSync3)]] - document - docs/ARCHITECTURE.md
- [[Comment speed format (Name25M5M, 25M5M, 25M)]] - rationale - docs/CONFIGURATION.md
- [[Configfrom_toml_str (defaults merge)]] - code - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Configuration]] - document - docs/CONFIGURATION.md
- [[DHCP speed sources (per_site  per_plan)]] - rationale - docs/CONFIGURATION.md
- [[Deferred minor items from defaults review]] - concept - brain/HANDOFF.md
- [[Handoff]] - document - brain/HANDOFF.md
- [[Loader rules (unknown key, type, range rejection)]] - concept - docs/CONFIGURATION.md
- [[Naming branch, crate, binary, service lqosync3]] - concept - brain/HANDOFF.md
- [[PPPoE speed order decision]] - concept - brain/HANDOFF.md
- [[PPPoE speed sources (speed.pppoe_sources)]] - concept - docs/CONFIGURATION.md
- [[Policy in TOML, formats in code]] - rationale - docs/CONFIGURATION.md
- [[Rate direction and units (rate_limit_order, unitless_unit)]] - concept - docs/CONFIGURATION.md
- [[Router state (online, failed, unreachable, offline)]] - concept - docs/CONFIGURATION.md
- [[Router state classification (RouterResult, unreachable_since)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Router state model (onlinefailedunreachableoffline)]] - rationale - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[Session updatecsv.py dissection]] - document - brain/10-sessions/2026-10-31-updatecsv-dissection.md
- [[Speed rules decision (routeros_rx_tx, unitless bps)]] - rationale - brain/HANDOFF.md
- [[TLS (tls_verify, tls_ca_file, webpki-roots)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Task 1 Crate scaffold and config]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Verification checks (cargo test, clippy, fmt, v3sim check.sh)]] - concept - brain/HANDOFF.md
- [[router_defaults per-router defaults]] - concept - docs/CONFIGURATION.md
- [[configdefaults.toml embedded defaults]] - concept - docs/CONFIGURATION.md
- [[offline_action (default remove)]] - concept - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[offline_after (default 300 s)]] - concept - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[offline_after  offline_action]] - rationale - docs/CONFIGURATION.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/CLI_and_Checks_2
SORT file.name ASC
```

## Connections to other communities
- 12 edges to [[_COMMUNITY_Speed Rules 2]]
- 11 edges to [[_COMMUNITY_Topology 3]]
- 10 edges to [[_COMMUNITY_Topology 4]]
- 7 edges to [[_COMMUNITY_Topology 2]]
- 7 edges to [[_COMMUNITY_Config 2]]
- 6 edges to [[_COMMUNITY_Speed Rules]]
- 6 edges to [[_COMMUNITY_Router Reads 2]]
- 5 edges to [[_COMMUNITY_MikroTik Session 3]]
- 3 edges to [[_COMMUNITY_MikroTik Session 4]]
- 2 edges to [[_COMMUNITY_Status and Diff 3]]
- 2 edges to [[_COMMUNITY_MikroTik Session 5]]
- 1 edge to [[_COMMUNITY_Router Reads 3]]
- 1 edge to [[_COMMUNITY_Status and Diff 4]]

## Top bridge nodes
- [[Handoff]] - degree 29, connects to 11 communities
- [[Configuration]] - degree 37, connects to 9 communities
- [[Architecture (LQoSync3)]] - degree 10, connects to 5 communities
- [[ADR-0003 Router failed vs offline]] - degree 15, connects to 4 communities
- [[Session updatecsv.py dissection]] - degree 12, connects to 4 communities