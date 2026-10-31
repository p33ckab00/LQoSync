---
type: community
cohesion: 0.06
members: 91
---

# Decisions and Session Notes

**Cohesion:** 0.06 - loosely connected
**Members:** 91 nodes

## Members
- [[runlqosbus 0777 safety caution]] - rationale - docs/LIBREQOS-V3-INGRESS.md
- [[1415 CSV columns (v2_pairv3_pair)]] - concept - brain/20-decisions/ADR-0005-topology-follows-v3.md
- [[ADR-0001 One model, many writers]] - rationale - brain/20-decisions/ADR-0001-one-model-many-writers.md
- [[ADR-0002 v3 target selection]] - rationale - brain/20-decisions/ADR-0002-v3-target-selection.md
- [[ADR-0003 Router failed vs offline]] - document - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[ADR-0004 TOML policy, Python defaults]] - document - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[ADR-0005 Topology follows v3]] - document - brain/20-decisions/ADR-0005-topology-follows-v3.md
- [[ADR-0006 v3 guards and node types]] - document - brain/20-decisions/ADR-0006-v3-guards-and-node-types.md
- [[ADR-0007 Fresh design, not a port]] - document - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[ADR-0008 Terminal UI]] - document - brain/20-decisions/ADR-0008-terminal-ui.md
- [[All policy in TOML]] - rationale - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Architecture (LQoSync3)]] - document - docs/ARCHITECTURE.md
- [[Brain Home (MOC)]] - document - brain/Home.md
- [[Configuration (LQoSync3)]] - document - docs/CONFIGURATION.md
- [[Five implementation plans (core, runtime, v3, TUI, install)]] - concept - brain/HANDOFF.md
- [[Fresh design, not a port]] - rationale - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Group nodes typed site on v3]] - rationale - brain/20-decisions/ADR-0006-v3-guards-and-node-types.md
- [[Handoff]] - document - brain/HANDOFF.md
- [[How LibreQoS v3 handles nodes]] - document - brain/30-knowledge/v3-node-handling.md
- [[Installation (LQoSync3)]] - document - docs/INSTALLATION.md
- [[LQoSync3 (lqosync3 tool)]] - concept - README.md
- [[LQoSync3 README]] - document - README.md
- [[Last-good topology retention]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[LibreQoS v3 ingress document]] - document - docs/LIBREQOS-V3-INGRESS.md
- [[LibreQoS v3 ingress facts]] - document - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[LibreQoS.py --updateonly no-op in v3]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[Live production values (2026-10-31)]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Offline v3 simulation]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[Operations (LQoSync3)]] - document - docs/OPERATIONS.md
- [[Orphan branch lqosync3]] - concept - brain/10-sessions/2026-10-31-updatecsv-dissection.md
- [[Output target v2_pair]] - concept - README.md
- [[Output target v3_network]] - concept - README.md
- [[Output target v3_pair]] - concept - README.md
- [[PPPoE speed source order]] - concept - brain/HANDOFF.md
- [[Parent Node ID column]] - concept - brain/20-decisions/ADR-0005-topology-follows-v3.md
- [[Profile name number+unit rule]] - rationale - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Python output format for shadow compare]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Random 8-char CircuitDevice IDs]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Removal guard (20% or 50 rows)]] - rationale - brain/HANDOFF.md
- [[Router credentials in 0600 files]] - rationale - README.md
- [[Router state model (onlinefailedunreachableoffline)]] - rationale - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[RouterOS listen command (untested)]] - concept - brain/10-sessions/2026-10-31-updatecsv-dissection.md
- [[SIGUSR1 sync now  SIGHUP reload]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[Scenario tests (replace golden parity tests)]] - concept - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Semantic compare with live files before cutover]] - concept - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Session 2026-10-31 project setup and v3 simulation]] - document - brain/10-sessions/2026-10-31-project-setup-and-v3-simulation.md
- [[Session updatecsv.py dissection]] - document - brain/10-sessions/2026-10-31-updatecsv-dissection.md
- [[Shadow mode]] - rationale - README.md
- [[ShapedDevices.csv as only state]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[Speed rules rate_limit_orderunitless_unit]] - rationale - brain/HANDOFF.md
- [[Speed source order TOML lists]] - concept - brain/20-decisions/ADR-0004-toml-policy-python-defaults.md
- [[Stable node id (lq3- FNV-1a 64)]] - rationale - brain/20-decisions/ADR-0005-topology-follows-v3.md
- [[Static comment rows never removed]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[TUI config editor (toml_edit)]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[Target auto-detection]] - rationale - README.md
- [[Temp file then rename write]] - rationale - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[Tool must reject duplicate IP and node name]] - concept - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[Write files only on content change]] - rationale - README.md
- [[topology source in lqos.conf]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[topology source setting]] - concept - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[compile_mode (flatap_onlyap_sitefull)]] - concept - brain/30-knowledge/v3-node-handling.md
- [[compile_mode not full warning]] - concept - brain/20-decisions/ADR-0006-v3-guards-and-node-types.md
- [[flat_promotion parent fix]] - rationale - brain/20-decisions/ADR-0005-topology-follows-v3.md
- [[liblqos_python.so binding]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[libreqos-v3-preflight]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[libreqos_3.0.7 beta package]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[libreqos_network.json schema 2]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[libreqos_network.json schema_version 2]] - concept - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[lqosd (v3 control plane)]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[lqosd inotify file watcher]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[lqosd runtime rules (rates, unique names, parents)]] - concept - brain/30-knowledge/v3-node-handling.md
- [[lqosync-updatecsv.service (Python)]] - concept - README.md
- [[lqosync3 Core Implementation Plan (Plan 1)]] - document - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[lqosync3 core design spec]] - document - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[lqosync3 tui (ratatuicrossterm)]] - rationale - brain/20-decisions/ADR-0008-terminal-ui.md
- [[max_floor_mbps (0.2)  safe_min_mbps (0.1)]] - concept - brain/20-decisions/ADR-0005-topology-follows-v3.md
- [[mikrotik-rs RouterOS API crate]] - concept - README.md
- [[no_parent guard on v3 targets]] - rationale - brain/20-decisions/ADR-0006-v3-guards-and-node-types.md
- [[offline_action (default remove)]] - concept - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[offline_after (default 300 s)]] - concept - brain/20-decisions/ADR-0003-router-failed-vs-offline.md
- [[status.json state file]] - concept - brain/20-decisions/ADR-0008-terminal-ui.md
- [[topology_import.json (internal)]] - concept - docs/LIBREQOS-V3-INGRESS.md
- [[topology_import.json (internal, do not write)]] - concept - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[updatecsv.py]] - code - brain/30-knowledge/updatecsv-py-dissection.md
- [[updatecsv.py defects to fix]] - concept - brain/30-knowledge/updatecsv-py-dissection.md
- [[updatecsv.py dissection]] - document - brain/30-knowledge/updatecsv-py-dissection.md
- [[v3 bridge boot gate]] - concept - brain/30-knowledge/v3-node-handling.md
- [[v3 name-derived legacy node id]] - concept - brain/30-knowledge/v3-node-handling.md
- [[v3 offline compile results]] - concept - brain/30-knowledge/v3-node-handling.md
- [[v3 offline simulation method]] - document - brain/30-knowledge/v3-offline-simulation-method.md
- [[v3 schema validation results]] - concept - docs/LIBREQOS-V3-INGRESS.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Decisions_and_Session_Notes
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Targets and Applier]]
- 1 edge to [[_COMMUNITY_Config Rules and Modes]]
- 1 edge to [[_COMMUNITY_Pure Core Design]]

## Top bridge nodes
- [[lqosync3 Core Implementation Plan (Plan 1)]] - degree 6, connects to 2 communities
- [[Configuration (LQoSync3)]] - degree 9, connects to 1 community
- [[ADR-0002 v3 target selection]] - degree 8, connects to 1 community
- [[Installation (LQoSync3)]] - degree 4, connects to 1 community