---
type: session
date: 2026-11-17
tags: [topology, ownership, reliability]
---

# Authoritative independent static input

The user requested a commit and push, then the next topology task.
Commit `fa964c4` contains explicit node minimums. The push to `origin/lqosync3` succeeded.
The static-input implementation remains local. Preserve unrelated operational edits.

## Source contract

Configured collection reads `<state_dir>/topology-manager/static-input.json` only for declared protected static ownership.
The private document contains schema version 1, a positive revision, rows, and the compatibility forest.
The loader never derives it from generated output. Dynamic subscriber endpoints still require current router observations.

One source document prevents mixed CSV/network generations. The source requires a private regular file and directory owned by the current user.
Reads reject unsafe permissions, symlinks, and files above 16 MiB. The row decoder stops above 100,000 entries.
Existing ownership validation requires exact protected rows and independent roots. Shared managed/static branches remain unsupported.

## Publication and recovery

The source path, revision, and exact byte digest follow prepared and compiled pairs.
Configured owner checks prevent reuse across runtime state directories. Changed or deleted sources block admission and publication.
A raw static preview has no source proof and cannot publish.

The runner rejects a revision conflict before compilation. Publication repeats the check before registry commit.
Against the current receipt, a lower revision, different path, or changed bytes at the same revision fails.
Every source edit requires a higher revision and an atomic complete-document replacement with mode 0600.

Receipts retain the optional static binding. A new source revision creates a new receipt even when output bytes remain equal.
Old receipts without the field still load. Older strict binaries reject a nonempty new field. The receipt version remains 1.
Apply evidence compares the complete context, including source path, revision, and digest.

Checks detect changes at admission and publication boundaries. They do not lock an external editor during output writes.
A change detected before registry commit leaves policy and output unchanged. A later detected change blocks the file intent after policy commit.
Revision continuity uses the current receipt, not a separate permanent ledger. Existing interrupted-write recovery remains in use.

## Files and checks

Added `src/topology_manager/static_input.rs` and a plan. Extended coordinator, publication, runner, receipt tracking, and focused tests.
Updated compiler scratch coverage, apply-evidence tests, design, discussion, Home, and HANDOFF.

The red receipt test reproduced a reused receipt ID after a static revision change with equal output bytes.
The fix compares the static binding before it reuses a receipt. A restart test confirms the new revision remains available.
Other tests cover missing sources before connection, ownership mismatch, revision rollback, source drift during compilation, and unbound publication.
A disconnect test keeps the explicit static row while it removes current dynamic rows.

The installed-library scratch test preserves the static root and explicit node minimum. It does not apply a live topology.
No live source read, router write, service action, deployment, mode switch, or native apply occurred.
Logs: `.superpowers/sdd/2026-11-17-topology-static-input/`.

## Remaining work

Shared managed/static branches and their capacity budgets need a separate slice.
Enrollment UI, schedules, TUI, service activation, implicit native commitments, and planner/live proof remain gates.
Generated manager device IDs still exceed the direct schema-2 circuit writer's 64-character limit. Current output remains legacy/manual.
No production benchmark or independent review occurred. The project graph remains the earlier baseline; current files take precedence.

Plan: `docs/superpowers/plans/2026-11-17-topology-static-input.md`.
Related: [[2026-11-17-topology-node-minimums]], [[HANDOFF]], [[Home]].

## Final verification

The full suite passed: 1,145 tests, zero failures, and 12 ignored tests across 13 result groups.
All 409 active topology tests passed. This slice adds eight active tests above the 1,137-test checkpoint.
The installed-library scratch compiler also passed with the static root and explicit node minimum.
Format, Clippy with warnings denied, and whitespace checks passed.

Proof: `full-verified.log`, `clippy-verified.log`, `format-verified.log`, and `native-static.log` in the log directory.
The STE heuristic reports technical-term findings. Author review corrected paragraph and passive-voice issues. This is not certified STE compliance.
