---
type: session
date: 2026-11-17
tags: [topology, migration, ownership, reliability]
---

# Exact shared legacy adoption proposals

The user requested the next topology task after commit `6e5debd`.
That checkpoint contains shared static attachments and budgets. Its push to `origin/lqosync3` succeeded.
The adoption proposal slice remains local and uncommitted. Preserve unrelated operational edits.

## Proposal boundary

Added `legacy_static::adopt` and `adopt_configured`. They accept the fresh prepared pair and complete explicit snapshot mappings.
The helper checks revision, observation generation, baseline digest, candidate digest, and the original source deadline.
It extracts only selected static subtrees and explicitly protected static rows.

A selected shared root must match its actual legacy parent through the declared managed node mapping.
An independent root stays independent. Missing roots, overlapping selections, duplicate attachments, and mapped managed nodes inside selected branches fail.
Static node IDs cannot collide with managed export IDs, including dormant managed nodes.

The helper reuses existing managed adoption and durable alias construction.
It reconstructs the complete aliased snapshot, merges the extracted static branches, and checks full parity and shared budgets.
It also checks import compatibility and output bounds before it returns a proposal.
The configured wrapper keeps current filter proof and legacy configuration preflight mandatory.

## Source draft and persistence

`StaticAdoption` contains the candidate registry and a private static payload.
`static_input()` exposes that payload for inspection. `source_document(revision)` returns a schema-1 source draft.
The revision must be positive. The draft follows current 100,000-row and 16-MiB source limits.
Serialization borrows the payload instead of constructing another full JSON value.

Dynamic endpoints enter neither the returned registry nor the static source draft.
A temporary-store test confirms that an offline dynamic subscriber disappears from output after restart.
On return, its legacy alias remains stable and its IP/MAC come from current observations.
The explicit static row remains in its attached branch.

The helper does not write files, commit policy, select a writer, or activate the service.
Repeated calls with the same fresh evidence produce the same proposal. Expired evidence fails.
The caller can discard the proposal without rollback writes.

Controlled source installation, registry adoption, and recovery remain a separate transaction before activation.
This API does not prove that a later caller installed the returned source exactly.
Existing private-file checks, source revisions, verified backups, and forward registry recovery remain in force.
A missing or mismatched source blocks configured publication.

## Code and verification

Added `legacy_static.rs` and `legacy_static_tests.rs`. Extended configured coordinator adoption coverage.
The current tree remap and snapshot parity functions now serve both adoption paths. Existing behavior remains covered by prior tests.
Updated module registration, design, discussion, plan, Home, and HANDOFF. No dependencies changed.

The extraction copies each selected subtree once. It does not clone a selected subtree again for each retained ancestor.
The traversal uses the existing depth limit. Tests cover supported depth, deeper trees, source bounds, and unchanged repeated proposals.
No production benchmark occurred.

The first red run failed because the new proposal API was absent.
Focused tests then passed for aliases, independent/shared roots, stale snapshots, endpoint/rate changes, wrong parents, and incomplete classification.
Other tests reject collisions, overlapping roots, impossible shared guarantees, oversized children, and source drafts above current reader limits.
Configured tests reject changed filters and unmatched legacy rate policy.

The full suite passed: 1,167 tests, zero failures, and 13 ignored tests across 13 result groups.
All 431 active topology tests passed. This slice adds ten active tests and one optional installed-library test.
Format, Clippy with warnings denied, and whitespace checks passed.
The optional installed-library scratch test preserved the adopted legacy snapshot without live apply.

Logs: `.superpowers/sdd/2026-11-17-topology-shared-adoption/`.
Proof: `full.log`, `clippy.log`, `format.log`, and `native-adoption.log`.
Author review only. The STE checker is heuristic. Its output does not certify compliance.
No live source read, router write, service action, deployment, mode switch, or native apply occurred.

## Remaining work

Next: controlled migration staging and recovery before activation.
Native implicit commitments, planner/live proof, enrollment UI, schedules, TUI, and service activation remain gates.
Direct schema-2 circuit activation still needs the manager device-ID length migration.
The graph remains the earlier baseline. Current files take precedence.

Plan: `docs/superpowers/plans/2026-11-17-topology-shared-adoption.md`.
Related: [[2026-11-17-topology-shared-static]], [[2026-11-17-topology-static-input]], [[HANDOFF]], [[Home]].
