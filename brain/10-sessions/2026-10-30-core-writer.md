---
type: session
created: 2026-10-30
tags: [lqosync-v3, core-writer, recovery]
status: implemented-library
related: ["[[2026-10-30-core-reconciliation-planner]]"]
---

# Guarded writer and recovery

Base: `909b4b8`. Branch: `codex/core-writer`. Worktree: `/tmp/lqosync-core-writer`.
Scope: Task 3 execution library and scoped transport only. No live caller or production enable action.
Added bounded owned-host batches, exact revision approval, durable intent/result history, and recovery fencing.
The existing exclusive maintenance lease prevents concurrent consoles, maintenance, and writer executors.
This stage does not upgrade a console shared lease. Integrated worker coordination remains Task 4.
The existing protected-row migration engine and its membership invariant remain separate.

The RouterOS transport permits owned host additions, exact removals, and same-core list/comment changes.
Move preserves row ID and address. Add requires the actual returned row ID.
A separate Verifier contract must prove settings, source/session identity, WAN health, and explicit writer authorization.
No production implementation of that verifier or scheduler caller exists in this stage.
Before send, the transport verifies fresh context, reads exact target rows, and verifies context/rows again after that awaited read.
Unknown network outcomes preserve pending state and block automatic retry and rollback.
Known-unsent preflight errors clear the unsent intent durably. Persistence failure poisons the current executor.

Rollback removes added hosts by returned IDs and restores list/comment changes on the same row ID.
Deleted hosts are recreated with their original fields and a new actual RouterOS ID. The old deleted ID cannot be restored.
The journal records those replacement IDs and validates every completed/pending operation on reopen.
Reopen clears historical completion verification and requires fresh observation before finalization.

Review found incomplete binding-core baselines, stale context after the target read, and inherited completion verification.
Regression tests reproduced all three before fixes. Follow-up found no material defect in the corrections.
Other tests cover malformed unrelated owned entries, expiry before marker creation, storage failure, exact batch limits, and wire traps/lost IDs.
Updated: 2026-10-30T22:43:30+08:00 (Asia/Shanghai).
Fresh final verification: 278 tests passed, zero failures, 12 manual tests ignored.
Twenty-one writer tests and the deterministic completion-window unit test passed.
Format, strict Clippy, diff, documentation links, and Bash syntax checks passed.
Final review found no outstanding defect after the completion re-verification correction.
No live router reads/writes, settings changes, shaping changes, or restart occurred.
Next: finalize checks and integrate the verified library. Then implement console/scheduler controls under Task 4.
