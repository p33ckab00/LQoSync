# Implementation plan for scoped rebalance

> **For agentic workers:** Use superpowers:executing-plans inline. Do not delegate unless the owner requests delegation.

**Goal:** Add an explicit revision-checked rebalance for one source pool or topology branch.

**Architecture:** Reuse the resolver's private eligibility/capacity index through a child module. Start from normal fresh resolution. Change only active unpinned placements within the selected destinations. Run the aggregate stage before returning a candidate.

**Tech Stack:** Rust and current standard-library/anyhow dependencies.

**Spec:** `docs/design/TOPOLOGY_MANAGER_DISCUSSION.md`, decisions 3 through 6 and 10, and `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Global Constraints

- Preserve prior implementation and unrelated changes. Do not commit, push, deploy, or change live state.
- Require an expected policy revision and a fresh complete collection.
- Keep pins, offline intent, eligibility restrictions, subscriber rates, and outside-scope assignments.
- A branch action uses its descendant destinations only. A source-pool action uses its selected destinations and current verified source members.
- Do not pull a placement from outside the selected destination set.
- Normal resolution assigns new circuits before the explicit action. Report rebalance moves relative to that normal result.
- Use exact directional plan-demand scores. Count fixed placements in all applicable loads.
- Keep timer-based redistribution disabled. This action performs no routing or publication.

## Review Focus

- A balanced pool must produce no equal-quality churn.
- A source definition can occur below multiple WANs. Scope must use IDs and current membership, not labels.
- Pins and placements outside scope contribute load without becoming movable.
- A quality improvement must still pass aggregate minimum/floor checks.
- Repeated collection/action order must remain deterministic and revision checks must reject stale intent.

## Task 1: Define scopes and prove bounded movement

**Files:** create `src/topology_manager/rebalance.rs` and `rebalance_tests.rs`. Update `resolver.rs` and `mod.rs`.

- [x] Add failing tests for source/branch scope, balanced no-op, pins, and stale revisions.
- [x] Add `RebalanceScope::{SourcePool(String),Branch(String)}` and `RebalanceRequest { expected_revision, scope }`.
- [x] Return proposed moves plus the checked aggregate result. Keep all durable writes outside this API.
- [x] Validate selected scope IDs and use current source membership for pool actions.
- [x] Reuse fresh resolver validation, eligibility, and independent capacity context.

## Task 2: Implement deterministic improvement

- [x] Build demand from all current placements, including fixed pins and out-of-scope circuits.
- [x] Visit active movable circuits in descending plan size, then stable ID order.
- [x] Require strict improvement in the affected worst directional ratio. Keep equal bottleneck scores in place.
- [x] Rank improving moves through changes to the descending score multiset, with independent constraints before destination scores.
- [x] Use sparse score deltas for affected constraints. Do not scan the full graph for each trial move.
- [x] Execute one bounded sweep. Do not claim a global optimum or repeat moves in a background loop.
- [x] Run aggregate checks once on the final proposal. Withhold its candidate on conflict.
- [x] Test asymmetric rates, unequal plans, shared ancestor load, restrictions, reconnect/absence, deterministic order, and scale.

## Task 3: Review and capture

- [x] Run focused tests, the full suite, formatting, Clippy with denied warnings, and whitespace checks.
- [x] Review scope boundaries and score arithmetic. Fix material findings with regressions.
- [x] Update design, session, and handoff records with checks and limits.

## Execution Rulings

Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `b94ed7c` plus prior uncommitted slices.
One action performs a bounded greedy sweep. A strict bottleneck test prevents equal-quality churn from indivisible plan sizes.
Sparse global score changes rank improving moves without a full graph scan for each trial.
This favors fewer moves in ties but does not solve global minimum-move or optimal-partition problems.
Branch scope permits movement within that branch. Cross-WAN movement requires a source pool whose selected destinations span those WANs.
A pool without an explicit target list uses selected source instances and their descendants.
An unselected discovery or a WAN inventory source is not a subscriber pool.
An aggregate conflict rejects the proposal rather than reducing minima, bypassing pins, or changing scope.
A scope with mixed bounded and unbounded destinations requires explicit compatible capacity inputs. Do not invent allocation weights.
No live caller, scheduler, writer, migration, or source adapter is part of this slice.


## Completion Record

Added 25 checks: 24 rebalance tests and one public API/store integration test.
The full suite passes with 884 passed, zero failed, and 9 ignored.
Format and Clippy with denied warnings pass.
The 10,000-circuit action produces no moves on repeat. A score oracle checks 768 trials against full recomputation.

Review regressions fixed equal-bottleneck churn and mixed bounded/unbounded scope handling.
The action supplies proposed moves and an aggregate result. It performs no writes or publication.
Logs remain under `.superpowers/sdd/2026-11-16-topology-rebalance/`.
All implementation work remains uncommitted. No live caller or production state changed.

Next: verified source/identity adapters, stable ID creation, complete migration/recovery, and export/native integration.
The TUI and scheduler remain unfinished.
