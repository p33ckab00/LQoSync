---
type: session
created: 2026-11-23
tags: [topology, frontend, backend, contract, tandem]
status: planned
related: ["[[2026-11-23-topology-studio-design-review]]", "[[ADR-0014-canvas-auto-apply-and-writer-retirement]]"]
---

# Topology Studio tandem plan

Timestamp: 2026-11-23T17:11:52+08:00 (Asia/Manila).

## Goal

Plan the approved Studio implementation with Codex on frontend and Claude Code on backend.
Use STE prose and an explicit shared interface.

## Findings

The source checkpoint is `d38113e` on `lqosync3` in `/home/pi/LQoSync3.0`.
The existing API uses browser HTTP, gateway, private broker, and core service commands.
Current topology edits already use revision checks, request keys, and durable receipts.

The router config digest and registry revision protect different stores.
Saved policy does not prove runtime acceptance.
The default HTTP body limit is 4 KiB. New bounded policy requests need a route-local limit.

The current registry schema is 9. The backend task owns its next migration.
The current browser check runner uses port `19581` and skips `pulse-check` by default.
No live deployment audit occurred.

## Deliverables

- `docs/frontend/TOPOLOGY_STUDIO_API.md`: current APIs, proposed typed additions, errors, limits, and state rules.
- `docs/superpowers/plans/2026-11-23-topology-studio-tandem.md`: task owners, dependency gates, files, and exact checks.
- `docs/handoffs/2026-11-23-claude-topology-backend.md`: backend start packet and required return evidence.
- `docs/handoffs/2026-11-23-codex-topology-frontend.md`: frontend start packet and integration duties.
- Updated product/index links and [[HANDOFF]].

## Decisions within the owner assignment

Claude Code owns the shared contract and browser fixture.
Codex owns the Leptos UI and browser checks.
Separate worktrees use one agreed G0 contract commit.

Preview, draft, saved topology, and runtime apply evidence stay separate.
New capability checks protect older services.
New APIs remain proposals until the backend implements and proves them.

## Results

Fourteen local links, the JSON example, task ownership, and the 11 unique task IDs passed document checks.
Current routes, schema 9, and transport limits matched source assertions.
Whitespace checks passed.

The STE checker reported technical noun groups and word-form flags. Manual review kept technical names exact.
The heuristic checker does not establish certified compliance.
No proposed Rust type or service endpoint has compiled from these documents.
Implementation tasks have not started. No application code, live outputs, routers, or services changed.

## Next action

Commit the plan package. Start backend Task B0 before parallel consumer work.
Use the two lane handoff files to record commits, owned files, checks, and the next dependency.

## Links

- [[2026-11-23-topology-studio-design-review]]
- [[ADR-0014-canvas-auto-apply-and-writer-retirement]]
- [Implementation plan](../../docs/superpowers/plans/2026-11-23-topology-studio-tandem.md)
