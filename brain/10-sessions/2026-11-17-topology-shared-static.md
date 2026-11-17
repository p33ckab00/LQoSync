---
type: session
date: 2026-11-17
tags: [topology, ownership, capacity, reliability]
---

# Explicit shared static branches

The user requested a commit and push, then the next topology task.
Commit `f104356` contains authoritative static-input binding. The push to `origin/lqosync3` succeeded.
The shared static branch slice remains local and uncommitted. Preserve unrelated operational edits.

## Policy and hierarchy

`ExportOwnership::shared_attachments` maps each protected static source root to an internal managed parent ID.
The parent must be a WAN or intermediate node. Subscriber groups remain leaves.
One root has at most one attachment. Several roots can share a parent. Independent roots keep their previous behavior.

The authoritative source remains a forest. Publication resolves parent aliases and inserts attached roots below managed parents.
Static IDs, labels, capacities, minimums, rows, and descendants remain unchanged.
Global collision and depth checks reject ambiguous or excessive hierarchies before export.

Schema 4 remains current. Empty attachment policy stays absent from serialized documents.
Old documents load with no attachments. Older strict readers reject a nonempty new field.
The revision-checked store canonicalizes attachments and retains them across restart.
Reparenting changes hierarchy intent without removing protected ownership.

## Shared budgets

The checker uses exported whole-Mbps capacities. Attached static ceilings must fit immediate parents in both directions.
It does not clip static policy or subscriber rates. Subscriber maximums can exceed a node ceiling, but minimum guarantees must fit.

The checker counts each circuit once across its device rows. Those rows must agree on parent and rates.
A circuit cannot span different shared parents or a shared and independent parent.
Shared nodes cannot mix direct circuits and child groups.

An attached static node without an explicit minimum reserves its full ceiling for that direction, even with no rows.
Explicit static and managed minimums must cover their descendants.
The reverse traversal propagates a parent guarantee once, rather than adding that guarantee to its smaller descendant sum.
Every propagated obligation must fit the parent/WAN budget after quantization. Integer sums use checked arithmetic.

Managed nodes without explicit minimums retain the current mathematical budget rule. Native implicit commitments remain an activation gate.
This checker does not change placement weights or saved assignments. A new reservation can expose a placement conflict.
Existing explicit rebalance scope still applies. This slice does not move subscribers automatically to resolve such conflicts.

## Implementation and proof

Added `shared_static.rs` and `shared_static_tests.rs`. Extended ownership policy, coordinator tests, compiler tests, and module registration.
Updated design, discussion, plan, Home, and HANDOFF. No dependencies changed.

Insertion traverses the bounded tree once. Budget checks use flat indexes and an iterative reverse pass.
Independent-only policy skips shared budget evaluation. No production performance benchmark occurred.

The red tests showed that the old merger kept attached roots independent and ignored shared budget obligations.
A later fixture failure showed that the allocator selected WAN2. The budget fixture now pins its current subscriber to WAN1 explicitly.

Tests cover parent aliases, multiple WANs, nested guarantees, empty branches, quantization, invalid children, collisions, and circuit identity splits.
The multiple-device test retains 50 Mbps subscriber maximums below a 20/10 Mbps static node.
Configured collection exercises current static revisions and current dynamic endpoints below the shared WAN hierarchy.
Compiler admission rejects moved or removed attachment evidence.

The full suite passed: 1,157 tests, zero failures, and 12 ignored tests across 13 result groups.
All 421 active topology tests passed. This slice adds 12 active tests above the 1,145-test checkpoint.
Format, Clippy with warnings denied, and whitespace checks passed.

The installed-library scratch compiler preserved an attached static root and explicit managed minimum without a live apply.

Logs: `.superpowers/sdd/2026-11-17-topology-shared-static/`.
Final proof: `full-verified.log`, `clippy-verified.log`, `format.log`, and `native-shared.log`.
The STE checker is heuristic. Author review corrected paragraph-length findings. No certified STE claim or independent code review occurred.
No live source read, router write, service action, deployment, mode switch, or native apply occurred.

## Remaining work

Next: exact shared legacy snapshot extraction and ownership adoption.
The existing adoption helper still classifies independent top-level static roots only.
This slice supports explicitly classified static roots. It does not infer shared legacy ownership.

Native implicit commitments, planner/live proof, enrollment UI, schedules, TUI, and service activation remain gates.
Direct schema-2 circuit activation still needs a durable migration for manager device IDs above its 64-character limit.
The static source checks still occur at boundaries and do not lock external editors across output writes.
The graph remains the earlier baseline. Current files take precedence.

Plan: `docs/superpowers/plans/2026-11-17-topology-shared-static.md`.
Related: [[2026-11-17-topology-static-input]], [[2026-11-17-topology-node-minimums]], [[HANDOFF]], [[Home]].
