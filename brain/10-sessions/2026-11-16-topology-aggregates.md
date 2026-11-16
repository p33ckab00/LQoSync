---
type: session
created: 2026-11-16
tags: [topology, capacity, minimums, reliability, performance, ste]
status: aggregate-boundary-complete
related: ["[[2026-11-16-topology-resolver]]", "[[HANDOFF]]"]
---

# Aggregate capacities and minimum commitments

The owner directs the next task after the placement resolver.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `b94ed7c` plus prior uncommitted slices.
Agent: Codex `/root`. Timestamp: 2026-11-16T20:58:59+08:00, Asia/Shanghai.
Unrelated work remains intact. No commit, push, deployment, service, router, or live configuration changed.

## Changes

Added `src/topology_manager/aggregate.rs` and `aggregate_tests.rs`.
Exported the stage from `mod.rs` and reused the rate constraint helper.
Added optional `GroupDefaults.floor` and positive-value validation in `catalog.rs`.
Updated catalog fixtures for the new optional field and added public legacy parity proof in `tests/topology_foundation.rs`.

The evaluator consumes a complete placement candidate and checks current account/endpoint evidence.
Each current circuit has one placement and one contribution to demand and minimum commitments.
Direct automatic groups use PlanSum or CountTimesUnit. Custom groups use PlanSum.
The compiled factor applies once. Source automatic groups then apply backstop and floor inputs.
Manual nodes keep explicit capacities. Intermediate inheritance and WAN percentages respect effective ancestor ceilings.

Output separates raw demand, pre-backstop calculation, local target, and effective ceiling.
Descendant sums support checks and display. They do not recalculate intermediate capacities from subscriber plans.
Empty automatic branches disappear from the result. Their definitions and stored intent remain intact.
Empty manual/WAN nodes remain.

The evaluator checks summed minima at every applicable ancestor, including common parents of sibling groups.
It does not require the sum of child ceilings to fit a parent.
Plan/maximum oversubscription produces warnings. It does not change subscriber maxima.
Impossible minima or floors produce typed node conflicts and withhold the candidate.

## Floor compatibility

An absent floor loads as None and does not add a JSON field.
Existing schema-2 documents without a floor keep their policy meaning.
A later migration must copy current floor inputs explicitly and prove parity.
A previous reader cannot accept a configured floor it does not support. Backup and rollback remain activation gates.
The source floor applies only to automatic groups and cannot bypass an ancestor or backstop ceiling.

## Review and proof

Initial behavior tests: 19 failures before implementation. All 21 initial aggregate tests then passed.
The floor test failed strict decoding before the new field and evaluation logic existed.
Final review found that stripped endpoint evidence could still produce a candidate. Its regression failed before the guard.
Conflict order now keeps a consistent floor/minimum reason order for each node.

Added 35 checks: 34 aggregate tests and one public integration test.
The final full suite passes: 859 passed, zero failed, 9 ignored.
`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass.
Logs: `.superpowers/sdd/2026-11-16-topology-aggregates/`.

Tests cover PPP, DHCP site/per-plan, Hotspot, custom nodes, direct WAN subscribers, nested limits, percentages, floors, and backstops.
They also cover sibling minimum sums, independent directions, alias counts, absence, overflow, malformed stage input, and canonical order.
The public path compares legacy PPP factor/backstop/floor results with the new stage.
It does not prove complete configuration migration.

The graph/demand pass uses flat child links and iterative traversal, with linear work in nodes plus placements.
Canonical result order adds an ID sort. The reversed 20,000-level graph passes without recursion.
This is not a production benchmark.
The older graph supplied orientation only. Current code and tests supply evidence.
The live builder, writer, native verifier, dependencies, and original fixture expectations remain unchanged.

## Next action

Implement explicit scoped rebalance around this checked placement/aggregate pipeline.
Then add verified source/identity adapters, ID creation, complete migration/recovery, and export/native integration.
Existing export rate bounds, precision checks, unique names, and DPDK evidence remain required before publication.
Minimum checks establish feasibility of the proposed candidate. They do not prove packet scheduling guarantees or native admission.
The TUI and scheduler remain unfinished. Scheduler overlap, overnight, and DST rules still need decisions.
No live path calls this stage.

Plan: `docs/superpowers/plans/2026-11-16-topology-aggregates.md`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.
