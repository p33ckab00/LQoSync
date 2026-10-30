# Guarded core writer library

The `core_writer` library executes bounded owned-host batches with durable recovery journals.
`core_writer_transport` provides scoped RouterOS I/O with the existing credential and verified TLS client.
No CLI, web endpoint, scheduler, or continuous writer invokes these modules yet.
This implementation does not enable production writes or change console startup behavior.
The existing protected-row maintenance engine retains its separate schema and membership-preservation rules.

## Integration boundary

A future integration must get explicit writer authorization and supply a `Verifier` implementation.
That verifier must freshly check settings revision, source identities, session continuity, WAN health, and enable authorization.
It must return actual complete address-list rows for every core router in the configured bindings.
Its context must identify the exact safety conditions under which the approved decisions remain valid.
No desired or simulated rows can substitute for actual evidence.
Dynamic counters and timestamps do not belong in a stable context unless their changes invalidate the plan.

`Batch::from_reconcile` checks the canonical settings revision and rejects every Blocked decision.
It compiles Add, Remove, and Move decisions. Keep decisions do not create mutations.
A batch contains 1–20 mutations, at most one per subscriber, and an approval lifetime of at most 300 seconds.
The caller must split larger work into freshly reviewed/revalidated batches. This library does not truncate an oversized plan.

Desired assignments stay separate from actual membership. The writer never saves candidates or publishes shaping output.

The verifier contract is not a production implementation of source/health reconciliation.
The console/scheduler integration must implement and test that contract before automatic mode can exist.
No HTTP enable action or implicit authorization token exists in this stage.

## Ownership and scope

Every binding core needs a complete baseline, including protected and unrelated rows.
All observed row IDs must be unique per router. Owned rows within configured lists must be enabled permanent static hosts.
The marker, subscriber identity, list, router, and WAN scope must agree.
Duplicate owned identities, duplicate owned host IPs, broad owned entries, and malformed markers reject the batch.
Unresolved protected coverage blocks additions and moves that cannot prove non-overlap.

Supported operations:

- Add an absent owned host and record the exact returned RouterOS row ID.
- Remove the exact observed owned host by its full precondition and row ID.
- Move a host between configured WAN lists on the same core by changing only its list and ownership comment.

Moves preserve the row ID, IP address, static/enabled flags, timeout representation, and subscriber identity.
A move cannot change the router or address. No command edits protected broad ranges, routes, tables, or firewall rules.
The private RouterOS move method sends only `.id`, `list`, and `comment` to address-list set.
The original maintenance set method still accepts only `.id` and `address`.
Neither method exposes a generic command endpoint.

## Fence and journal

`Executor::open` requires the exact batch revision and an exclusive application maintenance lease.
Every console for that data directory must stop before an executor can open.
This stage does not upgrade a shared console lease. Task 4 must coordinate an integrated worker separately.

The lease excludes concurrent cooperating consoles, writers, and protected-row maintenance executors.
It cannot exclude external RouterOS administrators or scripts. External writer exclusion remains an operational requirement.

The private journal uses `data/state/core-writer/BATCH_REVISION.json` with schema version 1.
The existing application recovery marker binds the journal path and revision.
Dropping an executor releases the kernel lease but preserves the marker.
A process crash releases the kernel lock. A new executor can reopen only the matching journal and revision.
Console startup remains blocked while recovery is unresolved.

Before each send, the executor compares a fresh verifier result with the complete expected rows and context.
It persists the pending intent before it invokes the transport.
The transport connects, verifies again, rereads the actual target router rows, and verifies context and rows after that awaited read.
It rechecks approval and evidence deadlines immediately before it issues the scoped request.
After each known result, the journal records the outcome before further work and fresh actual verification.

These checks do not create a RouterOS transaction. External changes after the final read remain possible.
A one-request list/comment move does not prove packet-level atomicity or preserve established connection routing.
The library does not flush connection tracking or claim that production shaping moved.

## Uncertainty and recovery

Only failures proved to occur before request send carry `RequestNotSent`.
That typed result clears the unsent intent durably and permits a later try after fresh validation.

A trap, lost reply, timeout, missing add ID, or invalid returned ID remains uncertain.
Pending uncertain work blocks both automatic retries and rollback. The library does not identify additions by comment or IP.
A persistence failure poisons the current executor and preserves recovery fencing.
Do not edit a journal or remove the marker to force progress.

Reopening validates the journal history and clears its saved completion-verification flag.
Fresh observation through forward or rollback is required before `finish` can remove the recovery marker.
Finalization checks the runtime completion deadline. A failed later verification revokes prior completion confidence.
Successful finalization removes the marker only after the chosen direction has fully verified.
This finalization does not publish output or start services.

## Rollback

Rollback reverses known completed operations in reverse order.
An addition is removed using its actual returned ID. A move restores the original list and comment on the same ID.
A removed host is recreated with its original member fields and its actual newly returned ID.
RouterOS allocates that replacement ID. Rollback cannot restore the deleted internal ID.

The journal records replacement IDs so reopened rollback can compare exact current state.

Rollback needs fresh valid verifier context and exact expected rows. Forward approval expiry alone does not block rollback.
Settings, source, health, authorization, or foreign-row changes can still block rollback until separately reconciled.
Once rollback starts, the executor does not resume the forward direction.
No rollback can promise restoration of connection state, packet ordering, or production shaping.

## Verification boundary

Synthetic tests exercise the real API framing, same-ID set fields, actual returned IDs, lost replies, traps, and missing IDs.
Fault tests cover known-unsent errors, storage failure, stale completion, corrupted history, incomplete core scope, competing leases, and the 20-operation limit.
Mock tests do not prove uninterrupted failover on RouterOS hardware.
Isolated RouterOS provider and transition verification, console integration, and explicit production authorization remain required.
No live router reads/writes, settings changes, or restarts occurred during implementation.
