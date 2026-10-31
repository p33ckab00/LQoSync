---
type: community
members: 24
---

# CLI and Checks 2

**Members:** 24 nodes

## Members
- [[ADR-0001 One model, many writers]] - rationale - brain/20-decisions/ADR-0001-one-model-many-writers.md
- [[ADR-0002 v3 target selection]] - rationale - brain/20-decisions/ADR-0002-v3-target-selection.md
- [[ADR-0003 Router failed vs offline]] - document - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[Architecture (LQoSync3)]] - document - docs/ARCHITECTURE.md
- [[Comment speed format (Name25M5M, 25M5M, 25M)]] - rationale - docs/CONFIGURATION.md
- [[Configuration]] - document - docs/CONFIGURATION.md
- [[Handoff]] - document - brain/HANDOFF.md
- [[Home (vault map of content)]] - document - brain/Home.md
- [[LibreQoS v3 ingress facts]] - document - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[Loader rules (unknown key, type, range rejection)]] - concept - docs/CONFIGURATION.md
- [[Naming branch, crate, binary, service lqosync3]] - concept - brain/HANDOFF.md
- [[PPPoE speed order decision]] - concept - brain/HANDOFF.md
- [[PPPoE speed sources (speed.pppoe_sources)]] - concept - docs/CONFIGURATION.md
- [[Policy in TOML, formats in code]] - rationale - docs/CONFIGURATION.md
- [[Rate direction and units (rate_limit_order, unitless_unit)]] - concept - docs/CONFIGURATION.md
- [[Router state (online, failed, unreachable, offline)]] - concept - docs/CONFIGURATION.md
- [[Router state classification (RouterResult, unreachable_since)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md
- [[Session 2026-10-31 project setup and v3 simulation]] - document - brain/10-sessions/2026-10-31-project-setup-and-v3-simulation.md
- [[Session updatecsv.py dissection]] - document - brain/10-sessions/2026-10-31-updatecsv-dissection.md
- [[Speed rules decision (routeros_rx_tx, unitless bps)]] - rationale - brain/HANDOFF.md
- [[Verification checks (cargo test, clippy, fmt, v3sim check.sh)]] - concept - brain/HANDOFF.md
- [[offline_after  offline_action]] - rationale - docs/CONFIGURATION.md
- [[topology_import.json (internal, do not write)]] - concept - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[v3 offline simulation method]] - document - brain/30-knowledge/v3-offline-simulation-method.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/CLI_and_Checks_2
SORT file.name ASC
```

## Connections to other communities
- 16 edges to [[_COMMUNITY_Topology 2]]
- 16 edges to [[_COMMUNITY_Speed Rules 2]]
- 11 edges to [[_COMMUNITY_Topology 4]]
- 10 edges to [[_COMMUNITY_Topology 3]]
- 9 edges to [[_COMMUNITY_MikroTik Session 3]]
- 8 edges to [[_COMMUNITY_Config 2]]
- 7 edges to [[_COMMUNITY_Speed Rules]]
- 6 edges to [[_COMMUNITY_Router Reads 2]]
- 6 edges to [[_COMMUNITY_Community 41]]
- 3 edges to [[_COMMUNITY_Status and Diff 3]]
- 3 edges to [[_COMMUNITY_Status and Diff 4]]
- 2 edges to [[_COMMUNITY_MikroTik Session 4]]
- 1 edge to [[_COMMUNITY_Router Reads 3]]
- 1 edge to [[_COMMUNITY_MikroTik Session]]
- 1 edge to [[_COMMUNITY_MikroTik Session 5]]

## Top bridge nodes
- [[Handoff]] - degree 34, connects to 12 communities
- [[Configuration]] - degree 37, connects to 10 communities
- [[Home (vault map of content)]] - degree 23, connects to 9 communities
- [[LibreQoS v3 ingress facts]] - degree 14, connects to 4 communities
- [[Architecture (LQoSync3)]] - degree 10, connects to 4 communities