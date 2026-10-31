# Persisted worker guard and explicit recovery approval

This stage adds a library and mock protocol tests. No HTTP, CLI, scheduler, or automatic action starts recovery.
It does not stop or restart the console. It does not activate writes on live MikroTik routers.
A successful `/health` response proves console availability only.

## Saved guard

A new worker saves `state/core-worker/<batch-revision>.json` before it can send a request.
Schema 1 binds the original guard to the exact batch revision and batch context digest.
The guard includes the canonical root, settings revision, subscriber identities and rates, and source sessions.
It also includes WAN health context, router uptimes, and routing policy fingerprints.
The private file contains subscriber data. Keep it outside Git, exports, and public reports.

It contains no router passwords. The normal application data permissions still apply.

Artifact snapshots have a combined 64 MiB read limit. New workers reserve 128 KiB for bounded operation receipts.
A missing, changed, unsupported, oversized, or unsafe guard blocks recovery.
Old journals without a matching guard cannot get worker recovery approval.

The library cannot attach a new guard after execution starts. Do not edit files to bypass this check.
The protected-row maintenance engine has a separate schema and [recovery procedure](CORE_MAINTENANCE.md).

## Review and approval

`core_worker::Recovery::review` accepts the data directory, batch revision, action, fresh source, and a review lifetime of 1–30 seconds.
The action is `RecoveryAction::Forward` or `RecoveryAction::Rollback`.
The review acquires the existing exclusive application fence. An active console, maintenance session, or writer blocks it.
This task does not stop the live console to get that fence.

The review requires an existing exact marker and journal. It does not create or change recovery records.

The production source rejects absent settings without creating defaults.
The review checks fresh settings, selected subscribers, rates, session continuity, WAN health, routing policy, router uptime, and complete actual membership.
Known replies determine expected rows. Actual returned IDs determine exact inverse operations.
Unknown outcomes reject before approval, even when current rows resemble a possible result.
The library never infers a missing reply, retries that request, or deletes its marker.

`Recovery::proposal` exposes concrete review data: action, approval revision, manifest fingerprint, journal fingerprint, evidence deadline, and next operation scope.
`Recovery::revision` returns the exact approval revision. Each review has a new random nonce.
`Recovery::approve` requires that revision before the deadline.
It recollects evidence and checks exact journal and manifest bytes again under the held fence.

It then returns the existing guarded worker with permission for only the approved direction.
A forward grant cannot call rollback. A rollback grant cannot call forward.
No caller can reconstruct approval automatically from saved verification flags.

Pause, settings changes, control-state loss, expiry, and lease drop revoke recovery authority.
The same source verifier and scoped transport check fresh evidence before each request.
Only active approval and fresh successful completion verification can remove the matching marker.
An in-flight request can complete after pause. These checks do not create a RouterOS transaction.

## Conservative limits

The original forward batch deadline remains unchanged. Recovery cannot extend it.
Rollback can use a new short approval only if all fresh evidence and continuity checks pass.
The existing source continuity window remains at most 30 seconds, or the shorter configured evidence lifetime.
A longer restart gap blocks recovery. Fresh probe confidence must also exist within the accepted window.

Equal IPs alone cannot prove session continuity. Do not copy old health status into new evidence.
Long gaps, reconnects, changed policy, and uncertain outcomes need a new plan or a separately reviewed procedure.

The pure [recovery assessment](DISTRIBUTED_CONTROL.md) remains read-only and cannot grant execution by itself.
The [recovery viewer](SHADOW_CONTROLS.md) still has locked action buttons and GET-only routes.
Use synthetic sources and isolated loopback peers for the current tests.
Isolated RouterOS acceptance and separately reviewed production authorization remain necessary before runtime activation.

Production acceptance on 2026-10-31 found a practical limit. A new process needs about 50 seconds of probe rounds
before WAN health is Healthy. That is longer than the 30-second continuity window, so reviewed recovery from a
new process stops with "Source continuity window expired". Any change to that window needs a separate design review.
Mocks do not prove uninterrupted traffic, actual probe behavior, or RouterOS connection-mark behavior.
