---
type: session
created: 2026-10-31
tags: [lqosync-v3, core-control, coordination]
status: active
related:
  - "[[10-sessions/2026-10-31-shadow-controls]]"
  - "[[10-sessions/2026-10-31-core-worker]]"
---

# Coordinated worker library

The operator authorized continued work with no destructive change to live MikroTik configuration.
This stage uses code and synthetic loopback protocol peers only. No live router access or service restart occurred.

## Implementation

`Prepared::open_coordinated` accepts an exact approved batch, the matching root, a source, and the existing console coordinator.
It holds the shared console lease and the exclusive shadow slot. Offline and maintenance opens retain their original exclusive fence.
The coordinator binds to a canonical root. New coordinated sessions cannot resume journals or replace markers.

Approval includes a local policy epoch and lease lifetime. Pause, saved settings changes, and lease drop revoke old grants.
All settings handlers use `App::replace_settings`. Restore of an old configuration does not restore an earlier grant.
Source verification still checks settings, sessions, health, policy, and complete actual membership after awaited work.
The implementation moves the existing batch into the executor. It does not clone its full baseline for the new path.

There is no scheduler, CLI, HTTP activation, or accepted automatic mode. There is no cold-resume or recovery grant.
Local epochs do not deliver a distributed pause to another console process.
Fresh verification checks persisted policy, but it cannot undo a request already inside the transport.
External router writers remain outside these locks. Mock proof does not establish uninterrupted traffic.

## Review and verification

Fresh review found that ordinary settings handlers did not update the coordinator directly.
A regression called the ordinary options handler without middleware and confirmed the defect before the fix.
After the fix, an edit followed by restoration kept the original grant revoked.
The focused worker suite passed with console coexistence, scope rejection, pause during awaits, known replies, rollback, lost replies, and request cancellation.
The synthetic peers bound only to `127.0.0.1`.
Existing shadow and offline writer checks passed. Documentation links and diff checks passed.
Final checks passed: format, 314 tests, zero failures, 12 ignored, and strict all-target Clippy.
No new browser or performance benchmark ran for this backend stage.
No UI layout or dependency change formed part of this stage.

See [the worker guide](../../v3/CORE_WORKER.md) and [the execution ledger](../../docs/superpowers/plans/2026-10-31-coordinated-worker.md).
The graph remains a recall source from an earlier stage. No broad graph rebuild occurred.

## Next

Runtime activation, distributed controls, recovery approval, and isolated RouterOS acceptance remain incomplete.
No isolated lab is available. Keep live activation disabled until acceptance and a concrete production scope receive review.
