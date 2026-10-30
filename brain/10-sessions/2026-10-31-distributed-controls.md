---
type: session
created: 2026-10-31
tags: [lqosync-v3, core-control, recovery]
status: active
related:
  - "[[10-sessions/2026-10-31-coordinated-worker]]"
  - "[[10-sessions/2026-10-31-shadow-controls]]"
---

# Durable pause and recovery assessment

The operator authorized continued code and mock tests with no destructive live MikroTik change.
No live router access, restart, production settings, or dependency change occurred.

## Implementation

`core_shared_control` stores a revision-bound random generation and a durable inhibit flag per canonical data directory.
A separate nonblocking process lock serializes settings and control saves.
The durable lock file also distinguishes first initialization from lost initialized state.
A save revokes old work before settings IO. Failed or interrupted saves preserve a pause.
Ordinary saves cannot clear an inhibit. Explicit Preview save compares persisted settings under the same lock before it resumes previews.
Old binaries and external router writers do not participate.

Shadow tickets and writer guards check the shared generation at their verification boundaries.
Observed cancellation or state loss permanently revokes the old authority. Old byte restoration cannot revive it.
Policy mutexes do not cover filesystem reads. A second local check detects a pause during that read.
The coordinated lease uses one boxed enum variant to keep the offline executor small.

The pure `assess_recovery` function matches exact journal history to complete fresh actual evidence.
It derives expected membership and the next rollback scope from actual returned IDs.
Unknown outcomes reject even when observed membership resembles a possible result.
The assessment cannot authorize execution. Persisted worker guard reconstruction and isolated RouterOS acceptance remain incomplete.
The read-only viewer displays exact address and list scope. It still has no mutation routes or live verifier caller.

## Review and proof

Fresh review found three defects: non-sticky Paused mode, missing state bootstrap, and shadow ticket revival.
Regression checks failed before fixes and passed afterward.
Tests cover actual child-process pause, a separate console's worker revocation, restart preservation, interrupted saves, competing saves, corrupt controls, stale resume, unknown replies, and stale evidence.
The browser used a synthetic pending journal. Five-width Chromium checks showed exact address/list details, 14px body text, no horizontal overflow, and disabled approval.
POST mutation routes rejected. Local fixture expiry was corrected to the existing 300-second batch limit before browser verification.

Final checks passed: format, 326 tests, zero failures, 12 ignored, and strict all-target Clippy.
Documentation links and diff checks passed. The STE heuristic received manual review for technical terms and list warnings.
No new performance benchmark ran. The existing graph supported targeted recall. It was not rebuilt.
See [the operator guide](../../v3/DISTRIBUTED_CONTROL.md) and [the execution ledger](../../docs/superpowers/plans/2026-10-31-distributed-controls.md).

## Remaining work

Recovery execution approval needs a persisted worker guard and separate reviewed authorization.
Runtime activation remains absent. No scheduler, CLI, HTTP action, or accepted automatic mode starts writes.
No isolated RouterOS lab is available. Mock proof cannot establish uninterrupted traffic or actual RouterOS behavior.
