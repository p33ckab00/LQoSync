# Implementation plan for aggregate capacity

> **For agentic workers:** Use superpowers:executing-plans inline. Do not delegate unless the owner requests delegation.

**Goal:** Compute aggregate capacities and check minimum commitments from fresh resolved placements.

**Architecture:** Add a pure stage after the placement resolver. Use iterative graph traversal and checked integer sums. Reuse catalog capacity rules and rate arithmetic. Withhold the candidate on an aggregate conflict.

**Tech Stack:** Rust and current standard-library/serde/anyhow dependencies.

**Spec:** `docs/design/TOPOLOGY_MANAGER_DISCUSSION.md`, decisions 7 through 10, and `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Global Constraints

- Preserve prior implementation and unrelated changes. Do not commit, push, deploy, or change live state.
- Count each active circuit once. Use current placements and rates only.
- Automatic groups use direct members. Ancestors inherit or use explicit capacities.
- Preserve local calculation, backstop, floor, and ancestor ceiling as distinct values.
- Keep subscriber maxima unchanged when aggregate demand exceeds capacity.
- Check summed descendant minima against each applicable ceiling. Do not divide parent capacity among child ceilings.
- Treat this as mathematical feasibility. Export precision, native admission, and DPDK verification remain later gates.
- Do not add dependencies, a writer, a scheduler, or a live caller.

## Review Focus

- A WAN with many subscribers must keep its explicit capacity.
- DHCP/Hotspot count formulas must count circuits rather than endpoints.
- A broad intermediate branch must check all descendant minima without recalculating its capacity from their plans.
- Empty automatic branches must disappear from the result without loss of durable definitions.
- Backstop/floor policy must not inflate a node above an ancestor or backstop ceiling.

## Task 1: Compute the aggregate graph

**Files:** create `src/topology_manager/aggregate.rs` and `aggregate_tests.rs`. Update `mod.rs`.

- [x] Add tests before implementation and observe the absent behavior.
- [x] Accept only complete valid placement candidates and check one current placement per active circuit.
- [x] Compute direct basis with PlanSum or CountTimesUnit. Custom groups use PlanSum.
- [x] Apply the compiled factor once, then the source backstop for automatic nodes.
- [x] Resolve manual, inherited, and configured-WAN percentage capacities with all ancestor ceilings.
- [x] Keep calculated/local/ceiling values separate. Omit empty automatic branches and keep manual nodes.
- [x] Test PPP, DHCP site/per-plan, Hotspot, custom nodes, shared instances, nested constraints, and no-WAN roots.

## Task 2: Check minimum commitments and supported floors

**Files:** change `aggregate.rs`, `catalog.rs`, and the catalog fixtures.

- [x] Add optional explicit `GroupDefaults.floor` with a lossless absent-field default.
- [x] Apply floors only to automatic source-backed groups. Block floors above a backstop or ancestor ceiling.
- [x] Test summed minima at every active ancestor, directional conflicts, and multiple endpoints per circuit.
- [x] Report plan/maximum oversubscription as warnings. Do not change subscriber rates.
- [x] Return typed node conflicts and withhold the candidate if any aggregate check fails.
- [x] Test zero/overflow guards, malformed stage input, canonical order, restart policy persistence, and a deep graph.

## Task 3: Review and record proof

- [x] Run focused tests, the full suite, format checks, Clippy with denied warnings, and whitespace checks.
- [x] Review the stage boundaries. Fix material findings with red/green regressions.
- [x] Update the design, implementation record, session note, and handoff.

## Execution Rulings

Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `b94ed7c` plus prior uncommitted slices.
The approved minimum rule checks whether configured commitments can fit beneath hard aggregate ceilings.
It does not prove packet scheduling guarantees or native acceptance.
Current compatibility checks in `src/validate.rs` enforce the existing export rate bounds. This stage does not replace them.
An optional explicit floor preserves a source floor input for later migration. An absent floor does not invent a default.
Source-backed automatic nodes use the legacy factor/backstop/floor order. A floor cannot bypass a hard capacity limit.
No original fixture expectation or legacy builder behavior may change.


## Completion Record

Added 35 checks: 34 aggregate tests and one public integration test.
The final suite passes with 859 passed, zero failed, and 9 ignored.
Format and Clippy with denied warnings pass.
Public parity covers legacy PPP factors, backstop, and floor values.
The reversed 20,000-level graph passes without recursion.

Final review added a regression and guard for stripped current account/endpoint evidence.
Stable conflict order preserves the floor/minimum reason order for each node.
Logs remain under `.superpowers/sdd/2026-11-16-topology-aggregates/`.
All implementation work remains uncommitted. No live caller or production state changed.

Next: explicit scoped rebalance, verified adapters/ID creation, complete migration/recovery, and export/native integration.
The TUI and scheduler remain unfinished.
