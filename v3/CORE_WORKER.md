# Guarded worker and fresh verifier

`core_worker` connects fresh verification to the scoped RouterOS transport and the bounded executor.
This stage adds a library. No CLI, HTTP action, scheduler, or automatic-mode option starts it.
The console still rejects automatic mode.

The operator selected code and mock tests because no isolated RouterOS lab is available.
No live router read, router change, console restart, or production activation formed part of this task.

## Preparation and approval

`RouterSource::new` binds collection to one canonical application data directory.
`RouterSource::preview` reads current settings and selected routers, then computes a read-only reconciliation round.
Initial health requires advancing completed probe rounds. A first sample cannot authorize a batch.
`Source::capture` does fresh verification reads without recomputing reconciliation for every send.

`Prepared::new` accepts a complete snapshot and an approval lifetime of 1–300 seconds.
It rejects Blocked decisions, empty mutation batches, and batches above the configured limit or 20 operations.
It does not truncate a plan or chain more batches automatically.
Each later batch needs fresh preparation and review.

`Prepared::revision` identifies the exact batch and its context.
`Prepared::open` requires that revision and the same canonical data directory.
A batch from directory A cannot get a different fence in directory B.
The unchanged exclusive executor lease excludes every console, maintenance operation, and competing writer for that directory.
The offline open cannot run alongside a console. Do not stop the live console to try this library without separate authorization.

Approval starts closed. An exact approved open creates one expiring grant.
A caller can keep `Worker::approval` and revoke it. No method silently restores a revoked grant.

## Coordinated library open

`Prepared::open_coordinated` adds a separate, explicit library path. No deployed action calls it.
The caller must pass the existing console's `core_coordinator`, the exact approved batch revision, and a fresh source.
The coordinator binds to one canonical data directory. A coordinator from another directory rejects.
The open checks persisted settings before it creates a recovery marker.

The coordinated worker retains a shared application lease and holds the same exclusive process slot as shadow previews.
This allows the existing console to remain open. It excludes maintenance, offline writers, shadow work, and other coordinated workers.
New consoles still reject an active recovery marker.
This does not create a RouterOS transaction or exclude external router scripts and administrators.

Approval checks the local coordinator epoch. Pause and every successful settings replacement revoke prior work immediately.
A restore of previous settings does not restore the old grant. Lease drop also revokes detached approval handles.
Fresh source verification checks persisted settings and actual router evidence after awaited collection work.

[Durable control generations](DISTRIBUTED_CONTROL.md) also revoke work across updated console processes with the same canonical data directory.
Old binaries, other hosts, and external router writers do not participate.
A request that already entered the transport can complete after pause. Do not infer that pause reverses an in-flight request.

Coordinated open accepts new sessions only. It cannot resume an old journal, replace another marker, or reconstruct approval after restart.
Cancellation, lost replies, and unsuccessful completion preserve the recovery marker.
Only freshly verified completion with active approval can remove its matching marker.
The read-only [recovery viewer](SHADOW_CONTROLS.md) can inspect retained records. It cannot approve recovery.

The console rejects automatic mode. There is no scheduler, CLI, or HTTP activation for this library path.
Do not add a live caller before isolated RouterOS acceptance and separately reviewed production authorization.

## Fresh verification

The production source reads settings before and after router collection.
It collects at most four routers concurrently. Failed collection cancels outstanding reads and clears retained health confidence.
It does not save inventory, candidates, assignments, or shaping output.

The verifier checks:

- The canonical root and settings revision.
- Selected subscriber identities, rates, and session continuity in manual, count, and planned-load modes.
- Complete, fresh source and core inventories, with router uptime checks.
- WAN health, completed-sample deadlines, and supported probe attribution.
- Core routes, interfaces, interface lists, tables, rules, mangle, and filter records.
- Complete actual address-list rows, separately from the unchanged routing-policy context.

The policy check preserves rule order. It rejects any changed record rather than guessing which fields are harmless.
If RouterOS returns volatile policy fields, their changes can block execution. Lab acceptance must identify such cases before rollout.
Owned address-list changes remain subject to the executor's exact expected-row checks.
No desired rows can replace actual membership evidence.

CPU work runs in the blocking pool. The guard retains active sessions, not duplicate candidate assignment maps.
The source confirms settings again after parsing. The verifier checks approval and deadlines after awaited work.
A failed check revokes the grant before another send.

The transport retains its final target-row read and another verifier call immediately before each scoped command.
These checks do not create a RouterOS transaction. External changes after the final check remain possible.

## Completion and recovery

`Worker::forward` and `Worker::rollback` use the existing durable journal.
`Worker::finish` requires an active grant and fresh successful completion verification before it removes the recovery marker.
It does not publish a generation or apply production shaping.

An uncertain request blocks retry and rollback. A dropped worker does not remove its recovery marker.
A settings or source failure revokes the grant. Expired or revoked approval cannot authorize worker rollback or finalization.
Do not delete the marker or edit the journal to force progress.

The worker now saves an original guard manifest before execution.
[Reviewed recovery](RECOVERY_GUARD.md) adds an explicit library-only approval gate with fresh source checks and exact artifact binding.
A restart cannot reconstruct approval automatically. Long continuity gaps and unknown outcomes remain blocked.
The protected-row maintenance CLI uses a different schema and cannot recover a writer journal.
See [guarded writer recovery](CORE_WRITER.md) and [maintenance](CORE_MAINTENANCE.md).

## Acceptance boundary

Mock API tests cover fresh reads, settings changes, session changes, exact add/remove fields, actual returned IDs, rollback, and lost replies.
They also cover approval, directory binding, batch limits, routing-policy changes, and console exclusion.
Mocks do not prove RouterOS field behavior, WAN attribution, same-ID moves, connection-mark behavior, or uninterrupted failover.

[Shadow controls](SHADOW_CONTROLS.md) add read-only coordination, pause, and saved-journal inspection.
Coordinated execution has mock protocol proof. The recovery library has mock approval and execution tests. Runtime activation and isolated RouterOS acceptance remain pending.
Keep automatic writes disabled until those stages and the exact production scope have passed review.
