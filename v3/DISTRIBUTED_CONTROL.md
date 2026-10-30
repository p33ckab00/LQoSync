# Durable pause and recovery assessment

This stage adds library code and mock tests. It does not activate live writes or restart a service.
The current live MikroTik configuration remains unchanged.
See [Core control](CORE_CONTROL.md), [the worker guide](CORE_WORKER.md), and [recovery inspection](SHADOW_CONTROLS.md).

## Pause across console processes

Use **Pause shadow preview** or save **Paused** in the policy modal.
Updated processes that share the same canonical data directory read one durable control generation.
Their old shadow tickets and coordinated writer grants reject after the generation changes.
A request that already entered the transport can complete after pause. Pause cannot recall that request.

The control record is `state/core-control-state.json`. The separate control lock serializes local settings saves.
A save revokes the previous generation before settings IO. A failed or interrupted save leaves controls paused.

An ordinary settings save cannot clear a durable pause, even if another console retains an old Preview configuration.
A missing record after initialization remains paused. Corrupt, oversized, or unsafe records block work.
Observed state loss permanently revokes an old ticket or grant. A restore of its old bytes does not restore that authority.

To resume previews:

1. Reload Core control to get current settings.
2. Open the policy cog.
3. Select **Preview** and save the policy.

The save compares the console settings with persisted settings under the control lock.
If another process changed them, the save rejects. Reload before another try.
A successful explicit save creates a new generation. It never revives old grants or clears writer recovery markers.

Automatic mode remains rejected. This preview action cannot start router writes.

These controls apply to updated processes on one host with the same data directory.
Older binaries, other hosts, and external RouterOS scripts do not participate.
Do not delete control files or locks to force progress. State repair needs review.
The implementation does not change the current live process until a separately authorized deployment occurs.

## Review recovery evidence

The read-only viewer now displays the address, router, row ID, and source/destination lists for saved operations.
A pending addition has no known returned row ID. Its exact saved address does not establish the request outcome.
The viewer does not contact routers or claim fresh verification.

`core_writer::assess_recovery` is a separate pure library function.
The caller supplies exact journal bytes, their revision, complete fresh evidence, and the current time.
The function validates the schema, batch identity, saved history, evidence lifetime, context, and complete actual rows.
It derives expected rows from known replies. It uses actual returned IDs when it computes the next rollback operation.

It returns the exact journal fingerprint, evidence deadline, and next operation descriptions.
An unknown pending outcome rejects even if observed membership resembles a possible result.
Changed, incomplete, expired, or future-dated evidence rejects.

An assessment is not execution approval. It cannot create an executor, get a writer lease, send a command, or remove a marker.
`require_execution_approval` always rejects at this stage.
A separate [persisted worker guard and recovery gate](RECOVERY_GUARD.md) now provides explicit library-only approval.
That gate recollects evidence and checks exact artifacts. This pure assessment cannot replace it.
No approval, resume, rollback, or marker-removal HTTP action exists.
Isolated RouterOS acceptance and exact production authorization remain necessary before live activation.
