---
type: community
cohesion: 0.19
members: 24
---

# Early Sessions and ADRs

**Cohesion:** 0.19 - loosely connected
**Members:** 24 nodes

## Members
- [[ADR-0001 One model, many writers]] - rationale - brain/20-decisions/ADR-0001-one-model-many-writers.md
- [[ADR-0002 v3 target selection]] - rationale - brain/20-decisions/ADR-0002-v3-target-selection.md
- [[Architecture (LQoSync3)]] - document - docs/ARCHITECTURE.md
- [[Handoff (current project state)]] - document - brain/HANDOFF.md
- [[Home (vault map of content)]] - document - brain/Home.md
- [[Installation (LQoSync3)]] - document - docs/INSTALLATION.md
- [[LQoSync3 (lqosync3 tool)]] - concept - README.md
- [[LQoSync3 README]] - document - README.md
- [[LibreQoS v3 ingress facts]] - document - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[Operations (LQoSync3)]] - document - docs/OPERATIONS.md
- [[Orphan branch lqosync3]] - concept - brain/10-sessions/2026-10-31-updatecsv-dissection.md
- [[Output target v2_pair]] - concept - README.md
- [[Output target v3_network]] - concept - README.md
- [[Output target v3_pair]] - concept - README.md
- [[RouterOS listen command (untested)]] - concept - brain/10-sessions/2026-10-31-updatecsv-dissection.md
- [[Session 2026-10-31 project setup and v3 simulation]] - document - brain/10-sessions/2026-10-31-project-setup-and-v3-simulation.md
- [[Session updatecsv.py dissection]] - document - brain/10-sessions/2026-10-31-updatecsv-dissection.md
- [[Target auto-detection]] - rationale - README.md
- [[Write files only on content change]] - rationale - README.md
- [[topology source setting]] - concept - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[libreqos_network.json schema_version 2]] - concept - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[mikrotik-rs RouterOS API crate]] - concept - README.md
- [[no_parent guard on v3 targets]] - rationale - brain/20-decisions/ADR-0006-v3-guards-and-node-types.md
- [[topology_import.json (internal, do not write)]] - concept - brain/30-knowledge/libreqos-v3-ingress-facts.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Early_Sessions_and_ADRs
SORT file.name ASC
```

## Connections to other communities
- 14 edges to [[_COMMUNITY_Scenario Test Harness]]
- 14 edges to [[_COMMUNITY_Router State Decisions]]
- 12 edges to [[_COMMUNITY_v3 Ingress Evidence]]
- 10 edges to [[_COMMUNITY_Plan 1 Session and Review]]
- 8 edges to [[_COMMUNITY_DHCP Discovery and Exclusions]]
- 3 edges to [[_COMMUNITY_Terminal UI]]
- 3 edges to [[_COMMUNITY_Architecture Parts]]

## Top bridge nodes
- [[Handoff (current project state)]] - degree 22, connects to 7 communities
- [[Home (vault map of content)]] - degree 22, connects to 6 communities
- [[LQoSync3 README]] - degree 17, connects to 4 communities
- [[LibreQoS v3 ingress facts]] - degree 14, connects to 3 communities
- [[Session updatecsv.py dissection]] - degree 12, connects to 3 communities