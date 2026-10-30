---
type: session
created: 2026-10-31
tags: [lqosync-v3, core-control, recovery]
status: complete
related:
  - "[[10-sessions/2026-10-31-distributed-controls]]"
  - "[[10-sessions/2026-10-31-coordinated-worker]]"
---

# Persisted guard and exact recovery approval

The operator authorized continued code and mock tests. No live MikroTik change or service restart occurred.

## Implementation

The worker saves a schema 1 guard manifest before it can send. Its digest must match the exact batch context.
Private manifests contain subscriber data and no passwords. Keep them outside Git and public reports.
The initial recovery artifact snapshot uses descriptor-relative reads and a combined 64 MiB budget.
The exclusive recovery lease requires an existing matching marker and lock. It does not create records during review.

`Recovery::review` verifies original guard identity, fresh sources, settings, continuity, WAN health, policy, router uptime, and complete actual rows.
`Recovery::proposal` exposes concrete scope, exact artifact fingerprints, direction, deadline, and a new nonce-bound approval revision.
`Recovery::approve` recollects and verifies exact saved bytes before it returns a direction-limited worker.
The existing executor and scoped transport perform all requests. Unknown outcomes remain blocked.
Distributed pause and lease drop revoke recovery authority. Fresh completion and active approval remain necessary before marker removal.

The original forward deadline remains unchanged. Recovery approval lasts at most 30 seconds and cannot extend that deadline.
The existing source continuity window remains strict. Longer gaps and absent fresh probe confidence require another plan or procedure.
No old health status is treated as fresh evidence.
The read-only RouterSource now rejects missing settings without creating defaults. Application bootstrap retains its previous behavior.
Async review and approval perform substantial artifact parsing, hashing, and disk work in the blocking pool.

## Verification

The fresh reviewer found no concrete code defect. Negative and wire tests completed after that review.
The root review found missing-settings initialization in RouterSource. Its regression failed before the strict-reader fix and passed afterward.
Seventeen focused worker tests passed. These include synthetic forward resume and exact rollback after offline or coordinated execution.
Tests also cover unknown replies, missing/schema-changed/tampered/symlinked manifests, competing console leases, stale evidence, source changes, journal changes, expiry, pause, and direction limits.
The synthetic protocol peers bound only to loopback.
Final checks: format passed; 331 tests passed, zero failed, 12 ignored; strict all-target Clippy passed.
An intermediate full run had two existing scheduler mock timeouts during a concurrent build.
The final full run passed without a concurrent build or changes to test deadlines.
The reviewer also checked the final deltas and found no concrete defect.
Documentation links and diff checks passed. No UI change or browser test formed part of this stage.
No performance benchmark ran. The graph supported targeted recall and was not rebuilt.

See [the operator guide](../../v3/RECOVERY_GUARD.md) and [the execution ledger](../../docs/superpowers/plans/2026-10-31-recovery-guard.md).

## Remaining work

No scheduler, HTTP action, CLI, or accepted automatic mode activates recovery or live writes.
The operator has no isolated RouterOS lab. Actual acceptance and concrete production authorization remain pending.
Mocks cannot establish uninterrupted traffic, actual RouterOS field behavior, or connection-mark behavior.
