---
type: session
created: 2026-11-20
tags: [frontend, design, planning, rust]
status: documented
related: ["[[2026-11-20-frontend-blueprint-research]]", "[[2026-11-18-topology-service-api]]"]
---

# Frontend documentation and atomic plan

Checkpoint: 2026-11-20T16:11:44+08:00 (Asia/Shanghai).
Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`.
Source baseline: `9a465e7`.

## Owner request

Save every generated picture under `docs/frontend` with documentation.
Write the feature design and atomic implementation plan in STE.
Keep Rust application logic, connect the frontend to the current backend, and record missing functions.
Address performance, reliability, and integrity. Commit and push to `origin/lqosync3`.

## Deliverables

The [frontend index](../../docs/frontend/README.md) links the complete package.
It contains 11 original PNG copies, hashes, prompts, screen descriptions, feature design, contracts, backend gaps, and quality requirements.
The implementation plan has 19 task cards with dependencies, interfaces, files, tests, and stop conditions.
The package preserves the TUI and uses Leptos CSR with Rust/Wasm.

The proposed integration uses an unprivileged Rust HTTP gateway and a narrow local broker.
The existing service remains the policy and publication owner.
Local plan workflows replace the mockups' router-profile sync behavior.
Routing writes, schedules, remote control, and destination intelligence retain explicit domain gates.

## Checks and limits

All image hashes and relative document links pass.
The task order satisfies the declared dependencies. Whitespace checks pass.
The STE heuristic and manual review cover the documents. No certified compliance is claimed.
No application code, runtime state, or dependency changed. No Rust or browser tests were necessary for this documentation-only change.
The graph remains the earlier index. Current source and the backend map take precedence.
The [validation report](../../docs/frontend/VALIDATION.md) records the scope of proof.

## Next action

Execute F00 when implementation starts. Record compatible dependencies and a measured baseline before feature code.
This request authorizes documentation and its GitHub publication, not production deployment.
