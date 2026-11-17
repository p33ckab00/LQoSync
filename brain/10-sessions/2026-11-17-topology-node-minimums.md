---
type: session
created: 2026-11-17
tags: [topology, minimums, guarantees, integrity, reliability, ste]
status: explicit-node-minimums-complete
related: ["[[2026-11-17-topology-dynamic-bounds]]", "[[HANDOFF]]"]
---

# Explicit node minimums

Time: 2026-11-17T17:56:39+08:00 (Asia/Shanghai).
Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`.

The owner authorized a commit and push, then the next task.
Checkpoint `e21f11d` contains DHCP identity overrides and imported dynamic bounds. The push to `origin/lqosync3` succeeded.
This node minimum slice remains local and uncommitted.

## Changes

- Added `NodeMinimum` and its checked legacy conversion in `rates.rs`. Added the optional catalog node policy.

- Extended aggregate guarantees, publication, and dormant legacy preflight.

- Extended aggregate, publication, admission, catalog, conversion, store, and installed compiler tests.

- Updated constructors in `tests/topology_catalog.rs` and `tests/topology_foundation.rs`.

- Updated design, discussion, plan, Home, and handoff. Preserve unrelated operational edits.

Each explicit direction uses positive whole-Mbps values. An absent direction stays absent.
Capacity floors, maximum rates, and group factors remain separate controls.
Empty policy fields do not change old serialized nodes. Schema 4 remains current.
An older reader rejects a nonempty unknown policy field.

The evaluator adds direct subscriber minimums and each child's explicit obligations from leaves to roots.
An explicit parent guarantee must cover that sum. Its propagated commitment replaces the smaller descendant sum.

Sibling guarantees share their parent budget. The evaluator checks the effective ceiling at each ancestor, including the WAN.
The loop uses flat indexes and checked integer additions. It does not recurse or shrink a guarantee.
Empty automatic groups disappear. Fixed empty branches reserve their declared minimums.

Publication repeats the budget check against quantized whole-Mbps ceilings.
It exports explicit directions through the current compatibility minimum keys.
The v3 node writer maps those keys to native commit fields. An absent direction retains commit equal to ceiling.
The manager's current adapter still publishes the legacy/manual pair.

Dormant preflight compares every selected source instance with the configured legacy template.
A missing or changed direction remains a `GroupMinimum` issue.
Capacity floors do not substitute for native minimums. Compiler admission rejects dropped or changed minimum metadata.

## Verification

- Full suite: 1,137 passed, zero failed, and 12 ignored. Ten new active tests.

- All 402 topology tests pass.

- Tests cover sibling excess, explicit parent limits, direct subscriber demand, nested budgets, and quantization.

- Tests also cover per-direction absence, strict numeric limits, store reopen, and compiler metadata changes.

- The installed-library scratch compiler preserves a pair with an explicit minimum. All compiler writes target temporary directories.

- Format, Clippy with warnings denied, and whitespace checks pass.

- Initial checks found missing integration constructors and test bounds that did not reach their intended condition. Corrected checks pass.

- The author reviewed the changes. No independent review or production benchmark occurred.

Logs: `.superpowers/sdd/2026-11-17-topology-node-minimums/`.
Proof: `export-red.log`, `topology-first.log`, `full-complete.log`, `clippy-complete.log`, and `native-minimum.log`.

## Limits and next action

The mathematical budgets cover explicit node guarantees and subscriber minimums.
Implicit native default commitments, shaping-planner transformations, and live DPDK acceptance remain activation gates.
The schema-2 writer test checks node conversion only. Generated manager device IDs exceed that writer's 64-character circuit/device limit.
Do not enable direct schema-2 circuit publication from this adapter without a compatible identity migration.

Next: authoritative static-input binding and source revision checks.
Shared branches, enrollment UI, schedules, service activation, and the Topology Manager TUI remain unfinished.

No live source read, router write, native apply, service action, deployment, or mode switch occurred.
Unrelated operational edits remain intact. The graph remains the earlier baseline.

Plan: `docs/superpowers/plans/2026-11-17-topology-node-minimums.md`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.
