---
type: session
created: 2026-11-16
tags: [topology, rebalance, scope, reliability, performance, ste]
status: scoped-rebalance-complete
related: ["[[2026-11-16-topology-aggregates]]", "[[HANDOFF]]"]
---

# Explicit scoped rebalance

The owner directs the next task after aggregate validation.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `b94ed7c` plus prior uncommitted slices.
Agent: Codex `/root`. Timestamp: 2026-11-16T21:21:39+08:00, Asia/Shanghai.
Unrelated work remains intact. No commit, push, deployment, service, router, or live configuration changed.

## Changes

Added `src/topology_manager/rebalance.rs` and `rebalance_tests.rs`.
The new module is a child of the resolver and has a public alias at `topology_manager::rebalance`.
It reuses the private resolver index rather than duplicating eligibility rules.
Added a public API/store integration test to `tests/topology_catalog.rs`.

The request selects a source pool or branch and requires the expected policy revision.
Normal fresh resolution assigns new circuits before the explicit action.
Proposed moves refer to that normal baseline.
Only current unpinned circuits within selected destinations enter the sweep.
Pool scope also requires current source membership. Branch scope stays within descendant destinations.
Pins, offline intent, and outside-scope assignments remain fixed and contribute load.

The action visits larger plans first, then stable subscriber IDs.
It requires a strict bottleneck improvement or better destination balance without worse independent scores.
Equal-quality ties keep the current placement.
Sparse global score changes rank improving moves. Shared ancestor loads cancel when a transfer stays below that ancestor.
The final proposal passes through aggregate minimum/floor checks before it can supply a candidate.

## Review corrections

Initial red: 12 behavior checks failed before implementation. All 14 initial checks then passed.
Review found unnecessary movement for a two-circuit normal allocation with the same worst bottleneck score.
The rounding regression failed before the stricter move rule. Normal allocations for counts 1 through 20 now stay in place.

A second regression failed for a scope with mixed bounded and unbounded destinations.
Such a scope requires compatible explicit capacity policy. The action does not invent weights for missing capacity.
All-unbounded scopes still use equal plan demand.

## Proof and limits

Added 25 checks: 24 rebalance tests and one public integration test.
The full suite passes: 884 passed, zero failed, 9 ignored.
`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass.
Logs: `.superpowers/sdd/2026-11-16-topology-rebalance/`.

Tests cover source/branch scope, pins, outside load, target restrictions, asymmetric rates, unequal plans, and fresh source membership.
They also cover offline intent, multiple endpoints, stale revisions, invalid scopes, private persistence, and restart.
The 10,000-circuit action preserves the expected capacity ratio and produces no moves on repeat.
This is not a production benchmark.

The score oracle compares 768 trial changes against full recomputation and checks rankings for alternative destinations.
It also checks that demand updates preserve shared ancestor loads.
Trial work depends on eligible destination count and independent path depth. It does not scan the full graph per move.
Each action uses one bounded greedy sweep. It does not prove a global optimum or globally minimum movement.
It never enables background redistribution.

The older project graph supplied orientation only. Current code and tests supply evidence.
The implementation leaves dependencies, the legacy builder, writer, native verifier, and original fixture expectations unchanged.
No live path calls the action.

## Next action

Add verified source/identity adapters and stable source/instance ID creation.
Then complete migration/recovery, export/native integration, and the TUI.
Publication must keep current rate bounds, precision checks, serialized writes, and native evidence verification.
The scheduler still needs overlap, overnight, and DST decisions.
No production activation occurred.

Plan: `docs/superpowers/plans/2026-11-16-topology-rebalance.md`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.
