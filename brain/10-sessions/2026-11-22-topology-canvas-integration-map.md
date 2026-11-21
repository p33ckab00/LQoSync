---
type: session
created: 2026-11-22
tags: [topology, canvas, integration, migration]
status: mapped-not-implemented
related: ["[[2026-11-21-infinite-canvas-research]]", "[[ADR-0014-canvas-auto-apply-and-writer-retirement]]"]
---

# Canvas integration and writer integrity map

The owner requested a complete frontend/backend map and a proven transition away from the legacy producer.
The source checkpoint was `f8898b4` on branch `lqosync3` in `/home/pi/LQoSync3.0`.
The current source already selects the scheduled topology writer through durable ownership.
The production owner and deployed binary were not inspected in this task.

## Deliverables

- `docs/frontend/TOPOLOGY_CANVAS_INTEGRATION.md` maps features, service operations, publication invariants, and transition stages.
- `docs/superpowers/plans/2026-11-22-topology-canvas-integration.md` defines Tasks 0–8, their file owners, interfaces, and acceptance checks.
- The product design and older plan/connection map link to the new direction.
- [[ADR-0014-canvas-auto-apply-and-writer-retirement]] records the owner decisions.

## Source findings

The receipt tracker replaces the two output files sequentially after it saves intent.
Each file replacement is atomic. This alone does not prove atomic pair visibility to an importer.
The registry/static migration journal does not establish recovery of every live pair replacement.
The plan therefore requires reader-gate evidence or a coordinated publication protocol before cutover.

Saved policy, written files, imported state, and native acceptance remain separate milestones.
The current proposal/confirmation browser flow must become one automatic edit operation.
The topology publisher reuses existing infrastructure without the legacy generation loop.
Shared CSV/JSON serializers do not require removal merely because the legacy loop uses them.

## Verification and limits

The project graph supplied source leads. Direct source inspection confirmed the map.
Checks passed for 14 local links, 16 primary source paths, all nine task IDs, and whitespace.
The heuristic prose checker reported style findings. The documents do not claim certified compliance.
Planned runtime tests and migration gates did not run in this documentation task.
No application code, live exports, ownership, routers, or services changed.
No commit, push, cutover, or legacy deletion occurred.
