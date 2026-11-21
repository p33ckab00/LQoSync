# Canvas integration and writer transition

Status: implementation map, not implementation or live acceptance.
Date: 2026-11-22 (Asia/Manila). Source checkpoint: `f8898b4`, branch `lqosync3`.
The worktree also contains uncommitted canvas documents and unrelated Routers work.

The owner requires the canvas workflow to reach both frontend and backend.
The topology path must publish without the legacy generation loop.
Keep the legacy path available until correctness, recovery, and runtime acceptance have evidence.
Do not equate a successful file write with successful shaping.

Read the [product design](TOPOLOGY_CANVAS_DESIGN.md) and [implementation plan](../superpowers/plans/2026-11-22-topology-canvas-integration.md).

## Current source findings

- [Runner::cycle](../../src/runner.rs) already selects either topology or legacy through durable runtime ownership.
- [Scheduled topology cycle](../../src/runner/topology_cycle.rs) collects, compiles, and publishes through the topology engine after enrollment and activation.
- [Runtime ownership](../../src/topology_manager/runtime.rs) prevents an active topology owner from silently falling back to legacy.
- [Topology publication](../../src/topology_manager/publication.rs) commits registry state and submits the output pair to the receipt tracker.
- [Write receipts](../../src/write_receipt.rs) record intent and hashes, but replace the output files sequentially.
- [File writes](../../src/files.rs) use temporary files, sync, and rename for individual output files.
- [Apply evidence](../../src/apply_evidence/verify.rs) correlates receipts, input hashes, imported generations, and native acceptance.
- [Web jobs](../../crates/web-gateway/src/topology_jobs.rs) and [forms](../../crates/web-ui/src/topology_forms.rs) still require proposal and confirmation.

The canvas mock proves modal interaction only. It does not call these service operations.
Earlier notes that say the service never selects topology automatically predate the current scheduled topology path.
Source inspection does not establish which binary or owner is active in production.

## Feature-to-code map

Paths below identify existing owners. Each row defines the change and the proof needed for completion.

| Feature | Frontend and contract | Backend reuse or change | Required proof |
|---|---|---|---|
| Infinite workspace | `crates/web-ui/src/topology.rs`, `topology_model.rs`; new focused canvas component | Persist layout separately from shaping policy | Pan, zoom, keyboard selection, undo, and reload preserve positions without shaping writes |
| FAB dock and modals | `topology.rs`, `topology_forms.rs`; scoped styles | Keep current settings ownership and typed service actions | Every settings editor opens in canvas, with focus return and automatic save |
| Existing settings and plans | Modal sections reuse the current typed settings and shared plan records | `settings.rs`, `plans.rs`, `source_rate.rs`, `dhcp_overrides.rs`, `edits.rs` | Factors, defaults, fallback, DHCP overrides, and plan membership retain units, scope, and inheritance |
| Selective discovery | `crates/web-contract/src/topology.rs`, `crates/web-gateway/src/topology_jobs.rs` | `src/topology_manager/wan.rs`, `reader.rs`, `coordinator.rs`, `source_policy.rs` | Only requested enabled routers/categories are read; discovery never attaches nodes |
| Source attachment | Inventory selection and modal source filters | `catalog.rs`, `edits.rs`, `normalize.rs`, `identity.rs` | PPP profile/server selectors stay distinct; uncertain membership cannot authorize export |
| Shared multi-WAN groups | Typed edges and effective-placement display | `catalog.rs`, `layout.rs`, `resolver.rs`, `aggregate.rs`, `shared_static.rs` | One effective parent and circuit identity, with no duplicated shared bandwidth budget |
| Automatic edits | Replace review/confirm controls; expose request and apply status | New typed automatic edit command through `src/api/dispatch.rs`, `runner.rs`, `edits.rs`, `web_receipts.rs` | Exactly-once policy effect for the same actor/key/payload, stale revision rejection, no human gate |
| Apply scheduling | Saved revision and active revision remain distinct | `src/runner/topology_cycle.rs`, `src/runner.rs` | Edits schedule bounded reconciliation; active topology runs without legacy generation |
| Count/capacity/quality allocation | Allocation modal, per-group scope, pin controls | `resolver.rs`, `rebalance.rs`, `rates.rs`; add count and measured quality policies | Pins remain eligible; pinned demand counts; quality data have explicit freshness and failure rules |
| Site/AP location | Location modal and full decimal text in contract | `catalog.rs`, `edits.rs`, `publication.rs`; versioned registry migration | Exact saved text, numeric JSON export, correct Site/AP types, runtime/map metadata |
| Optional preview | Read-only preview modal | Reuse `preview.rs` and compiler output | No mutation, writer claim, or approval dependency |
| Publication integrity | Show write ID, policy revision, and apply evidence | `publication.rs`, `compiler.rs`, `write_receipt.rs`, `files.rs`, `runtime.rs` | Coordinated generation, recovery, and no false Applied state |
| Runtime feedback | `TopologyView`, `Publication`, operation polling/status | `web_view.rs`, `apply_evidence/monitor.rs`, `apply_evidence/verify.rs` | Evidence belongs to the exact write and generation, not an older successful cycle |

All backend paths in this table, except explicitly prefixed ones, are under `src/topology_manager/`.
New fields need typed contract, registry, cache, UI, and migration changes together.
Do not display a working control until its service operation exists.

## Automatic edit contract

Implement one authenticated automatic edit operation. The following interface is proposed, not an existing route.

`POST /api/v1/topology/edits` accepts `{ key, expected_revision, edit }`.
The server derives the actor from the authenticated session. The payload cannot select another actor.
The edit uses a typed, bounded enum. Unknown fields, invalid source scope, and stale revisions fail before policy commit.
The gateway submits one service command, proposed as `topology.edit.apply`.

Reuse the current validation and durable web-receipt logic inside that command.
Internal preparation can reuse proposal logic, but the browser does not drive a proposal/confirm sequence.
Keep old clients compatible during the rollout. Do not remove their routes before the new client ships.

Use the same key when retrying the same payload. A changed payload requires a new key.
After an unknown outcome, query the existing receipt before another mutation.
Serialize edits per revision. Coalesce only unsent local field changes, never distinct accepted operations.
Treat latitude and longitude as one field update so intermediate pairs cannot apply.
Keep layout revisions independent so a card drag cannot supersede or publish a policy edit.

The service returns a durable operation identity and the saved policy revision.
Link later compilation, write receipt, import, and native evidence to that operation or its superseding desired revision.
A saved policy revision is not an applied revision.
An older completion cannot replace newer status in the browser.

## Required writer integrity

| Invariant | Existing foundation | Gap or evidence to add |
|---|---|---|
| One active producer | Instance lease and durable runtime owner | Prove competing process, replaced lease, unknown owner, and restart behavior |
| Correct candidate | Identity, eligibility, source freshness, static ownership, capacity, controlled compiler | Add canvas edges, Site/AP metadata, and new allocation modes to these checks |
| Consistent output pair | Persisted intent, hashes, per-file atomic replacement | Prove importer cannot accept mixed generations during replacement or recovery |
| Durable policy and output relationship | Registry commit precedes file intent | Recover and display saved-but-not-published revisions without claiming success |
| Correct runtime acceptance | Import evidence plus native ACK and coverage | Bind status to exact receipt, file hashes, configuration, and generation |
| Failed apply preserves service | Runtime last-good behavior and visible errors | Distinguish active runtime retention from a partially replaced on-disk pair |
| Retry safety | Actor-bound web receipts and generation checks | Cover disconnect, lost reply, crash, superseded edits, and repeated apply requests |

Two atomic file renames do not form one atomic pair transaction.
The existing migration journal recovers registry/static-source installation, not every live pair replacement.
First prove whether the installed importer and its generation gate already exclude mixed pairs.
If proof fails, implement a recoverable pair protocol in the existing publisher and coordinate the reader before cutover.
Do not assume a generation marker is effective unless the actual reader enforces it.
Inject a failure between replacements and during receipt persistence to test the complete protocol.
Recovery must restore a verified matching pair or complete the intended matching pair before import proceeds.
Keep runtime last-good queues separate from on-disk recovery claims.

## Transition stages

| Stage | Active live producer | Exit evidence |
|---|---|---|
| 0. Baseline | Current owner, unchanged | Inventory of active binary, service, target, source mode, owner, files, overrides, compiler, and backups |
| 1. Implement and test | Current owner, unchanged | New schema, command, canvas, and receipt checks pass without live writes |
| 2. Shadow | Current owner only | Topology compiles from the same frozen authorized observations in scratch space |
| 3. Controlled cutover | Topology only after exclusive transfer | Complete candidate passes invariants and first publication has matching runtime acceptance |
| 4. Observation and rollback proof | Topology only | Repeated publication, no-op, restart, source outage, edit, and recovery cases pass |
| 5. Legacy retirement | Topology only | Evidence bundle passes all gates; supported configurations no longer require the legacy producer |

Shadow output must never target production paths or trigger import.
Compare semantics by stable identities, endpoints, parent resolution, rates, static records, and capacities.
Document intended differences such as approved hierarchy changes and legacy rounding.
An unexpected missing subscriber, static branch, or address is a failure, not an ignored difference.
New canvas policies need invariant checks even when they have no legacy equivalent.

Existing live topology adoption preserves its records and ownership. Empty initialization must not erase it.
Before transfer, stop or quiesce the previous producer and prove exclusive lease ownership.
Do not allow concurrent legacy and topology writes during a comparison or rollback.
Use automated admission checks for the first topology write, then ordinary canvas edits remain automatic.
Release acceptance gates do not add Preview or Approve buttons to the normal edit workflow.

## Rollback and retirement

Keep the previous binary, compatible configuration, registry backup, and verified complete file pair through the compatibility window.
Before ownership transfer, a failed gate leaves the current owner unchanged.
After transfer, an error retains topology ownership and last-good runtime state. It never silently runs legacy.

A controlled rollback must quiesce topology, record the failed operation, and recover pending writes before changing ownership.
Restore a matching supported state/configuration/pair under the exclusive lease and confirm import plus native acceptance.
Do not reset ownership by deleting `runtime-owner.json` or restore only one output file.
If inverse migration is unsupported, restore the verified backup rather than open the new schema with the old binary.

Retirement requires a separately reviewed deletion patch after the acceptance evidence exists.
Remove the legacy generation/dispatch path and obsolete configuration branches, not shared infrastructure.
Keep useful CSV/JSON serializers, file replacement, receipts, compiler admission, import checks, and migration readers.
Importing a legacy snapshot for adoption does not make legacy the continuing producer.
The current user request authorizes this map and transition plan, not immediate live cutover or legacy deletion.

## Acceptance bundle

Record source revision, binary/compiler fingerprints, config digest, policy revision, observation generation, output hashes, and write ID.
Include circuit/device counts, protected-static inventory, semantic differences, import generation, native ACK, and coverage.
Include failure injection results, restart recovery, rollback result, and timestamps in Asia/Manila.
Predeclare the observation window and required events before the live run. A short idle interval cannot prove reliability.
Missing or failed evidence blocks retirement. Synthetic tests cannot substitute for installed-release runtime acceptance.

The current [live runbook](../operations/TOPOLOGY_LIVE_TEST.md) is a starting point, not proof of the new canvas workflow.
Update it for the new publisher protocol and rollback before running a live window.
No live window ran as part of this documentation task.
