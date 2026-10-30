# Persisted worker guard and reviewed recovery

Base: `a366cd8`. Branch: `codex/recovery-guard`.
Worktree: `/home/pi/lqosync-recovery-guard`.
Use code and synthetic loopback tests. No live router access, restart, or activation.

## Rulings

- Save the original guard before a worker can send. Bind its digest to the existing batch context.
- Missing, changed, unsupported, or unsafe manifests block recovery. Never reconstruct approval from a journal alone.
- Review under the unchanged exclusive maintenance fence. Require an existing matching marker and journal. Do not create records during review.
- Reuse fresh source verification, session continuity, policy checks, and exact actual rows. Do not weaken the current continuity deadline for restart recovery.
- Bind review approval to exact journal/manifest bytes, canonical root, direction, fresh evidence, shared control generation, and a deadline of at most 30 seconds.
- Recollect before approval. Unknown outcomes reject without guessing, retry, rollback, or marker deletion.
- Recover through the same executor and scoped transport. Enforce the approved direction. Expired original batches cannot resume forward.
- Preserve pause revocation and fresh verification before every request. Recovery does not extend the original forward batch lifetime.
- No HTTP, CLI, scheduler, automatic mode, or live recovery action is added. Library tests use mocks only.
- Longer continuity gaps require a new plan or separate procedure. Do not infer safe recovery from matching IPs alone.
- Preserve unrelated frontend planning edits in the main checkout.

## Steps

1. Observe missing guard/recovery tests fail.
2. Add bounded manifest persistence and pure artifact reads.
3. Add explicit review and direction-bound approval through the existing worker/executor.
4. Test restart, stale/missing/corrupt records, changed sources, journal drift, expiry, revocation, unknown replies, and exact mock rollback.
5. Obtain one fresh whole-branch review and fix concrete findings.
6. Run format, locked Rust tests, strict all-target Clippy, and documentation checks.
7. Capture results, commit, merge with unrelated edits preserved, and push after verification.

Ruling: verification reads must not initialize missing settings. RouterSource now uses a strict validated reader. Bootstrap behavior remains unchanged.

## Results

Format and strict all-target Clippy passed. The final locked suite passed: 331 passed, zero failed, 12 ignored.
Seventeen focused worker tests passed. Fresh branch review and final-delta review found no concrete defect.
An intermediate full run had two existing scheduler mock timeouts during a concurrent build. The final run passed without concurrent builds or changed deadlines.
Documentation paths, links, and diff checks passed. No live router access or service restart occurred.
Isolated RouterOS acceptance and runtime activation remain pending.
