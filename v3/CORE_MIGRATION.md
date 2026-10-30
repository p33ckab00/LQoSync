# Guarded core migration engine

The `core_migration` module implements the isolated execution engine and rollback journal.
It does not install a production RouterOS transport, a console write route, or a background writer.
The current console and RouterOS client remain read-only. This module does not publish output or apply LibreQoS changes.

## Plan preparation

`Plan::prepare` accepts evidence, WAN bindings, and the ownership UUID. It also accepts desired hosts and explicitly selected protected row IDs.
It computes residual intervals from the selected host addresses. It does not accept arbitrary replacement commands.

The plan requires these conditions:

- Every selected host already belongs to exactly one observed WAN, and that WAN matches its desired destination.

- Each host has a unique canonical subscriber UUID and IPv4 address.

- Every selected protected row belongs to an explicit binding.

- Each selected row is enabled, static, permanent, and has no application ownership marker.

- Every selected host replaces protected membership. No manual or current owned overlap can remain for that host.

- The residual intervals preserve each original row ID and comment.

- The union of IPv4 addresses in each bound list stays identical.

The engine does not support complete removal of an original protected row. Preparation rejects that case.
Prefixes and ranges use the current core planner parser. Interval arithmetic supports the full IPv4 range.
Desired host entries use the current `lqosync3` ownership marker format.

`plan.revision()` identifies the exact serialized plan, with captured evidence and expiry.
The revision is an approval correlation value, not an authentication secret or a user permission check.
The application that calls the engine must authenticate the operator and get explicit protected-entry authorization.
The engine rejects an approval value for a different plan.

Approval expires within 300 seconds of evidence capture. Each observation must be at most 30 seconds old.
The engine rechecks approval and observation age after journal persistence, immediately before each transport request.

## Forward execution

1. Create a private journal with `Executor::create(path, plan, approved_revision)`.

2. Call `executor.forward(&mut transport).await`.

3. Require fresh configuration/session evidence and exact expected router rows before every write.

4. Persist each intent before the transport call.

5. Add owned hosts and residual rows before the engine narrows any original row.

6. Persist each successful result, with the exact returned ID for an addition.

7. Read all evidence again before the engine reports `ForwardComplete`.

A repeated forward call checks current evidence without duplicate writes after completion.
The `create` function cannot replace a current journal. Use `Executor::open` with the same approved plan revision.
The engine can continue a known partial execution after a read failure if evidence still matches and approval has not expired.

`ForwardComplete` describes only this router migration engine. It does not mean that LibreQoS has applied production shaping.
Actual application validation must Pass before actual publication. Simulation results cannot satisfy that gate.

## Rollback and recovery

Call `executor.rollback(&mut transport).await` to compensate known successful operations.
Rollback restores original broad addresses first. It then removes additions by their exact returned IDs and expected complete values.
Original rows keep their IDs and comments. Rows outside the plan remain unchanged.
Each rollback intent and result uses the same durable journal protocol.

You can repeat a completed rollback without duplicate writes.
Rollback can run after approval expiry, but still requires fresh matching source/configuration evidence and exact router rows.

If approval or evidence expires before the transport request starts, the engine clears that known-unsent intent.
It preserves intents for all requests with unknown outcomes.

A transport error can occur after a successful write. The engine treats that outcome as unknown.
A pending intent blocks automatic retries and rollback, also after process restart.
Do not remove an unknown addition by comment or address. Its returned ID may be unavailable.
Inspect the journal and reconcile the actual router state through a separately authorized recovery procedure.

This stage does not implement automatic reconciliation of unknown outcomes.

A journal write failure prevents all subsequent writes from that executor instance.
The durable file may still contain the earlier pending intent. Reopen it only after storage recovery and state review.
The engine validates journal progress against the original plan before it accepts a reopen operation.

Journal files use mode `0600`. Use an operator-owned private local directory.
An exclusive sibling `.lock` file prevents two engine instances from concurrent access to the same journal.
A process crash can leave that lock file. Inspect the owner process and journal before manual lock removal.
Never remove a lock while its executor can still act.
The journal lock does not exclude another journal or another RouterOS producer.

## Required production integration

A production adapter is a separate deliverable. It must satisfy the `Transport` contract before this engine can affect real routers.

- Hold an exclusive writer fence across the complete forward or rollback call. Include other address-list producers in the maintenance procedure.

- Collect complete fresh inventory. Reject table errors, stale data, ambiguous WAN membership, and invalid routing policy.

- Validate settings, source selections, eligible node/WAN pairs, normalized client identities, and session continuity.

- Compute `Evidence.revision` from those inputs. Continuous-session counters must not create false reconnects. Proven reconnects must change the evidence.

- Include every expected router row, with rows outside the selected lists. Normalize default flags consistently and preserve all meaningful fields.

- Recheck exact expected rows before set/remove calls. Restrict commands to the supplied address-list fields and IDs.

- Return the actual new row ID after an add. Treat timeout, cancellation, lost reply, and uncertain errors as unknown outcomes.

- Align candidate assignments to observed WANs through guarded application APIs. Do not edit session files or fabricate reconnects.

- Require fresh actual end-to-end validation before publication. Keep production shaping apply separately authorized.

The engine cannot create a RouterOS transaction or exclude external writers by itself.
A fresh read followed by a write is insufficient without the adapter's writer fence.
The private audit JSON is review evidence. It is not directly executable by this module.

## Verification

Run from `v3/`:

```bash
cargo test --locked --test core_migration
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Tests use synthetic addresses and an in-memory transport. The journal tests use isolated temporary directories.
They cover one-WAN and two-WAN migrations, exact rollback, row scope, and foreign edits.
They also cover partial execution, lost replies, returned IDs, journal failures, and restart recovery.
These tests do not prove real RouterOS write behavior, external-writer exclusion, or interruption-free production migration.
