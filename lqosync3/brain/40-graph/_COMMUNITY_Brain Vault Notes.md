---
type: community
cohesion: 0.86
members: 7
---

# Brain Vault Notes

**Cohesion:** 0.86 - tightly connected
**Members:** 7 nodes

## Members
- [[ADR-0001 One model, many writers]] - rationale - brain/20-decisions/ADR-0001-one-model-many-writers.md
- [[ADR-0002 v3 target selection]] - rationale - brain/20-decisions/ADR-0002-v3-target-selection.md
- [[Brain Home (MOC)]] - document - brain/Home.md
- [[Handoff note]] - document - brain/HANDOFF.md
- [[LibreQoS v3 ingress facts]] - document - brain/30-knowledge/libreqos-v3-ingress-facts.md
- [[Session 2026-10-31 project setup and v3 simulation]] - document - brain/10-sessions/2026-10-31-project-setup-and-v3-simulation.md
- [[v3 offline simulation method]] - document - brain/30-knowledge/v3-offline-simulation-method.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Brain_Vault_Notes
SORT file.name ASC
```

## Connections to other communities
- 7 edges to [[_COMMUNITY_LibreQoS v3 Runtime Evidence]]
- 4 edges to [[_COMMUNITY_Sync Pipeline Internals]]
- 3 edges to [[_COMMUNITY_Operations and Cutover]]
- 3 edges to [[_COMMUNITY_Output Target Selection]]
- 2 edges to [[_COMMUNITY_Configuration and Secrets]]
- 1 edge to [[_COMMUNITY_Speed Rules and Open Decisions]]
- 1 edge to [[_COMMUNITY_Installation]]

## Top bridge nodes
- [[Brain Home (MOC)]] - degree 12, connects to 6 communities
- [[Handoff note]] - degree 9, connects to 3 communities
- [[ADR-0001 One model, many writers]] - degree 8, connects to 3 communities
- [[ADR-0002 v3 target selection]] - degree 8, connects to 3 communities
- [[Session 2026-10-31 project setup and v3 simulation]] - degree 8, connects to 1 community