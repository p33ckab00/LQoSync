# Implementation plan for topology placement

> **For agentic workers:** Use superpowers:executing-plans inline. Do not delegate unless the owner requests delegation.

**Goal:** Resolve fresh membership, subscriber rates, and stable capacity-weighted placements without a live caller.

**Architecture:** Extend the complete snapshot with current profile and plan evidence. Resolve one placement per circuit through catalog indexes. Return typed conflicts and withhold the candidate registry if any required placement fails.

**Tech Stack:** Rust, standard collections, existing anyhow and serde dependencies.

**Spec:** `docs/design/TOPOLOGY_MANAGER_DISCUSSION.md`, decisions 3 through 10, and `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Global Constraints

- Preserve prior implementation and unrelated edits. Do not commit, push, deploy, or change live configuration.
- Keep current profile evidence, rates, and endpoints outside the durable registry.
- Count each circuit once. Keep verified current endpoints together.
- Keep valid saved assignments and pins. Invalid intent blocks the candidate.
- Use explicit selected pools and target sets. Check source-backed ancestors and profile restrictions.
- Evaluate independent download and upload capacities. Do not derive allocation weights from current group members.
- Keep exact integer arithmetic and deterministic ties. Do not add dependencies.
- Keep full aggregate evaluation, minimum-feasibility proof, explicit rebalance, adapters, publication, scheduler, and TUI as later slices.

## Review Focus

- Missing profile evidence must differ from a verified absence of a source profile.
- Multiple active identities must not duplicate plan demand or conceal conflicting policy evidence.
- An automatic group must use independent ancestor capacity for allocation.
- Collection order and disconnected subscribers must not change valid durable intent.
- Unassigned subscribers and invalid pins must produce typed conflicts without a partially accepted registry.

## Task 1: Carry fresh policy evidence

**Files:** `src/topology_manager/observation.rs`, `tests/topology_registry.rs`.

- [x] Test current profile evidence and source rates through the snapshot and reconciliation path.
- [x] Add `ProfileObservation::{Unread,Known(Option<String>)}` and `ObservedPolicy`.
- [x] Validate supplied evidence and preserve one account observation per active identity.
- [x] Preserve all prior identity, endpoint, and snapshot checks.

## Task 2: Resolve membership, rates, and placements

**Files:** create `src/topology_manager/resolver.rs` and `resolver_tests.rs`. Update `mod.rs`.

- [x] Test one shared profile across two WANs, direct WAN destinations, and profile-only groups.
- [x] Test asymmetric capacities, independent child capacity, factors, and configured WAN percentage bases.
- [x] Test stable placements, pins, restriction failures, source-backed ancestor membership, and conflicting aliases.
- [x] Test missing fresh source rates, local authority, min/max controls, and one demand per circuit.
- [x] Test deterministic order, disconnect/return, overflow, and a synthetic large pool.
- [x] Implement indexed membership and iterative independent-capacity context.
- [x] Assign pins and saved placements before new circuits. Compare projected directional plan-demand ratios with exact cross products.
- [x] Return a candidate only if every required active circuit has a valid placement. This is not publication readiness.

## Task 3: Verify and capture

- [x] Run focused tests, the full suite, formatting, Clippy with denied warnings, and whitespace checks.
- [x] Review the connected path and fix material defects with regression tests.
- [x] Record results, performance limits, and the next activation gates in the design and brain notes.

## Execution Record

Worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, base `b94ed7c` plus prior uncommitted slices.
The implementation remains isolated from production. No new service or policy format is required.
Automatic allocation covers new circuits only. Explicit scoped rebalance needs a separate tested action.


## Completion Record

Added 31 checks: 2 observation checks and 29 resolver checks.
The final suite passes with 824 passed, zero failed, and 9 ignored.
Format and Clippy with denied warnings pass.
Review regressions fixed rational equality, explicit instances, target restrictions, ancestor bypass, and implicit rate authority.
Logs remain under `.superpowers/sdd/2026-11-16-topology-resolver/`.

The current implementation supplies a placement candidate only.
Aggregate formulas, minimum feasibility, explicit rebalance, verified adapters, migration, publication, scheduler, and TUI remain unfinished.
No production caller or live state changed. All implementation work remains uncommitted.
