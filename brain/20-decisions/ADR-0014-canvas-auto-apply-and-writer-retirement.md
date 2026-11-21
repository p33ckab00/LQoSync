---
type: decision
created: 2026-11-22
tags: [adr, topology, canvas, writer, integrity]
status: accepted
related: ["[[ADR-0005-topology-follows-v3]]", "[[2026-11-22-topology-canvas-integration-map]]"]
---

# ADR-0014 — Automatic canvas edits and proven writer retirement

## Context

The owner finalized an infinite canvas with FAB icons and modal settings.
Valid edits must save and apply automatically. Preview remains optional.
The owner requires reliable, correct publication before removal of the legacy writer.

## Decision

Use the existing topology engine and service publisher as the final runtime path.
Do not require operator preview or approval for ordinary valid edits.
Preserve automatic validation, eligibility, revision checks, and runtime acceptance evidence.
Map each canvas feature to frontend, contract, and backend owners before implementation.

Keep the legacy producer available during development, shadow comparison, and controlled transition.
Only one producer can write live output at any time.
After topology takes ownership, a failure must not silently invoke legacy.
Require coordinated-pair recovery, exact runtime acceptance, and rollback evidence before legacy retirement.
Remove the legacy producer in a separate patch after the required evidence passes.

## Consequences

Release gates do not add approval screens to the normal edit workflow.
Shared serializers and receipt infrastructure can remain after legacy retirement.
Actual MikroTik route control remains a separate unresolved scope.
Implementation and live acceptance are not complete from this decision.

The source map is `docs/frontend/TOPOLOGY_CANVAS_INTEGRATION.md`.
The task plan is `docs/superpowers/plans/2026-11-22-topology-canvas-integration.md`.
