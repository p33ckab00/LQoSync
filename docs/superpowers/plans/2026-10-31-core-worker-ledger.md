# Guarded worker and verifier ledger

Base: `b6efb03`. Branch: `codex/core-worker`.
Worktree: `/home/pi/lqosync-core-worker`.
The operator selected code and mock tests. No RouterOS lab is available.
Do not restart the live console or change live routers.
The previous uncommitted worktree and logs disappeared from `/tmp` during an interruption.
Reconstruct the changes in a persistent worktree. Run all required checks again.

Ruling: retain the exclusive offline executor fence. No shared-console lease upgrade or automatic enable action belongs in this stage.
Implement fresh RouterSource -> GuardedVerifier -> scoped RouterTransport -> Executor integration.
Bind exact approval to the canonical data root, settings, all selected sessions/rates, health attribution, and routing policy.
Keep actual core rows separate so known owned-host progress can pass exact executor checks.
Recheck settings and approval after collection and CPU work. Expire and revoke grants conservatively.
Keep batches bounded by the configured limit and 20 operations. Do not silently truncate or chain work.
Unknown outcomes retain the journal and startup marker. No restart creates implicit approval.
Online console coordination and real RouterOS acceptance remain pending.


Fresh final checks: 295 tests passed, zero failures, 12 ignored. Seven worker integration tests and one approval unit test passed.
Format, strict Clippy, local documentation links, and diff checks passed. Fresh review found no concrete defect.
The existing 50,000-session synthetic workload passed separately. Debug times: collect 592.8 ms, assign 854.9 ms, retain 1.062 s, candidate save 1.331 s, candidate load 2.448 s.
Build used a separate persistent Cargo target with debug symbols disabled. This is not an end-to-end worker throughput or paired baseline result.
No real RouterOS lab evidence exists. Do not mark online Task 4 or lab acceptance complete.
