# LQoSync4 agent tandem workflow

This workflow makes the [engineering rules](AGENTIC_ENGINEERING_RULES.md) and [production completion gate](PRODUCTION_READY_GATE.md) actionable. Root [AGENTS.md](../../../AGENTS.md) is the entry point. The [agent-tandem skill](/home/pi/.codex/skills/agent-tandem/SKILL.md) routes supporting skills and tools by task relevance.

## Role handoff

| Role | Owns | Handoff evidence |
|---|---|---|
| Coordinator | Scope, file ownership, accepted decisions, integration, final status | Task goal, affected layers, owner decisions, final diff, verification summary |
| Mapper/investigator | Graphify query, source trace, callers, data contracts, existing tests | File and function locations; current versus proposed behavior; unresolved assumptions |
| Builder | Smallest complete implementation and local checks | Changed paths, behavior, tests, known limits; no completion claim without integration evidence |
| Reviewer/verifier | Independent diff review, failure paths, security, performance, final checks | Findings with paths; commands actually run; pass/fail; unverified risks |

For an independent, bounded task, roles may run concurrently if each edit has one owner. Dependent edits and integration remain sequential. If the environment has no subagents, one agent performs each role in order; the review still reads the actual diff rather than relying on intent. Do not create parallel agents for a one-line change merely to satisfy this table.

## Shared source of truth

1. Read [HANDOFF](HANDOFF.md) and [DECISIONS](DECISIONS.md). An accepted owner decision outranks a proposal. Keep open decisions open.
2. Query [Graphify](../graphify-out/README.md) for architecture or route relationships. Verify graph claims against current Rust source, [product spec](../docs/PRODUCT_SPEC.md), [function map](../docs/FEATURE_FUNCTION_MAP.md), and [route contract](../docs/ROUTES_AND_COMPONENTS.md). Refresh the graph when those relationships change.
3. Record a durable handoff only when a task spans agents or sessions: objective, branch, files owned, facts and source paths, accepted/open decisions, changes made, checks and results, risks, and next action. Avoid repeating a full chat transcript in the brain.
4. Do not claim that a proposed API, route, component, or router writer is implemented until the source and end-to-end checks prove it.

## Completion gate

- Establish the existing behavior and meaningful alternatives before a nontrivial change. Preserve compatibility or document migration and rollback.
- Complete every affected layer and wire every visible control to real behavior. Use explicit pending, success, failure, stale, and recovery states. Do not ship placeholder success paths.
- Inspect the final diff for correctness, state consistency, bounded resource use, secret handling, authorization, cancellation, and partial failure. Fix confirmed issues and recheck after the last edit.
- Run relevant checks supported by the actual Cargo workspace and UI build. Include focused regression, integration, or browser checks for risk-bearing flows. A docs-only edit needs link/diff validation, not a full Rust test suite.
- Report separately what was implemented, compiled, linted, unit-tested, integration-tested, browser-tested, benchmarked, and exercised against production-like systems. State missing evidence and blockers without calling partial work complete.

Live RouterOS writes, deployment, and repository publication follow the user's authorization. Planning, read-only inspection, and reversible local edits can proceed within the task scope.
