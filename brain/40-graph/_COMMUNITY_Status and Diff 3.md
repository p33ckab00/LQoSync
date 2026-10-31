---
type: community
members: 13
---

# Status and Diff 3

**Members:** 13 nodes

## Members
- [[ADR-0006 v3 guards and node types_1]] - rationale - brain/HANDOFF.md
- [[Checks after installation]] - concept - docs/INSTALLATION.md
- [[Command reference]] - concept - docs/OPERATIONS.md
- [[LibreQoS v3 checks (no_parent, compile_mode)]] - concept - docs/CONFIGURATION.md
- [[Next action manual shadow run on production shaper]] - concept - brain/HANDOFF.md
- [[Node types (v2_type, v3_type)]] - concept - docs/CONFIGURATION.md
- [[network mode (grouped_router_root, flat_promotion, no_parent, ...)]] - concept - docs/CONFIGURATION.md
- [[clicheck  CheckLine]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[cliparse  cliCommand]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-runtime.md
- [[lqosync3 check]] - concept - docs/OPERATIONS.md
- [[lqosync3 defaults]] - concept - docs/OPERATIONS.md
- [[lqosync3 diff]] - concept - docs/OPERATIONS.md
- [[lqosync3 once --force]] - concept - docs/OPERATIONS.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Status_and_Diff_3
SORT file.name ASC
```

## Connections to other communities
- 5 edges to [[_COMMUNITY_Status and Diff]]
- 3 edges to [[_COMMUNITY_Speed Rules 3]]
- 3 edges to [[_COMMUNITY_Config 2]]
- 2 edges to [[_COMMUNITY_Decisions]]
- 1 edge to [[_COMMUNITY_Mock Router Tests 2]]
- 1 edge to [[_COMMUNITY_MikroTik Session 5]]
- 1 edge to [[_COMMUNITY_Topology 2]]

## Top bridge nodes
- [[Command reference]] - degree 8, connects to 3 communities
- [[lqosync3 check]] - degree 6, connects to 2 communities
- [[lqosync3 diff]] - degree 6, connects to 2 communities
- [[Next action manual shadow run on production shaper]] - degree 5, connects to 1 community
- [[lqosync3 once --force]] - degree 5, connects to 1 community