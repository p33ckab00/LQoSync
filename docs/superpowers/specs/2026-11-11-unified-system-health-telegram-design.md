# Unified system health and Telegram update reports

Date: 2026-11-11. Branch: `lqosync3`. Status: local implementation and verification. Production remains unchanged.

## Intent and confirmed requirements

The owner requested a review, research, and design discussion for unified system health.
Each actual managed-file update must produce a Telegram health report.
The owner selected an immediate WRITTEN/PENDING report and a later APPLIED or FAILED follow-up.

Treat `ShapedDevices.csv` and `network.json` as one logical update, with a result for each file.
One file can change while the other remains unchanged. A cycle with identical output is not a new write event.
Keep subscriber collection on the existing sync cycle. Preserve RouterOS speed-profile resolution.

This proposal covers the current LibreQoS v3 pair target first. Preserve existing behavior for other targets until we define their evidence contracts.
No production file, configuration, service, permission, or Telegram recipient changed during this review.
This review sent no live Telegram test message.

## Approaches considered

| Message flow | Benefit | Limitation | Decision |
| --- | --- | --- | --- |
| Immediate WRITTEN/PENDING report, then terminal follow-up | Shows each write promptly and records its later outcome | Usually two logical reports per update | Selected by the owner |
| Report only after confirmation or failure | Less chat traffic | Hides a pending update until its terminal result | Not selected |
| Edit the initial Telegram message | Compact chat view | Needs stored message IDs and makes historical transitions less visible | Not selected |

Recommendation: reuse the existing Rust notifier and health samplers. Add no external monitoring service for this feature.
Use one shared health model for Telegram, status output, and TUI presentation.

## Implementation progress

The owner authorized task execution after the performance review.
[Task 1](../../operations/UNIFIED_HEALTH_TASK1.md) adds strict normalized import evidence and a read-only native acknowledgement probe.
The findings below describe the original reviewed baseline. The input-comparison gap now has a local fix.
The shared health model, durable write receipts, pending checker, Telegram worker, and TUI projection are implemented locally.
The operator's read-only native policy check passed. It also found a separate allocation mismatch: 64 native slots versus 1000 generated slots.
APPLIED describes matching subscriber policy. Allocation mismatch remains DEGRADED in the shared health report.
The producer's policy content key excludes allocation capacity. No bridge setting was changed.
Production remains unchanged. See the linked operations document for limits and recovery procedures.

## Current implementation and review findings

References describe the checked source. They do not describe proposed behavior as already implemented.

1. `src/notify.rs`, `Notifier::message`, already checks `notify_on_changes && files_changed`.
   Reuse this behavior rather than add a second independent file-change notifier.
2. The current title can say `lqosync3 update applied` while `status.reload` still reports a pending LibreQoS import.
   File replacement and runtime acceptance need different events and labels.
3. Import confirmation alone is not an existing notification trigger.
   A later cycle with unchanged files can confirm the import without producing an APPLIED message.
4. `include_telemetry` currently adds cycle and router statistics. It does not join the newer aggregate, health, host, or NIC reports.
5. `src/main.rs` awaits `send_all` before the next sleep and signal select.
   Telegram latency therefore delays the control loop, although the telemetry tasks remain independent.
6. The notifier advances its event memory before delivery. Failed sends are intentionally not retried under the original Telegram design.
   A durable delivery requirement needs a new policy, not only more message text.
7. `post_json` currently accepts an HTTP 2xx response without retaining the Bot API success envelope or message ID.
   The new worker must validate the API result and store a delivery receipt.
8. The pair writer replaces files in order, with `network.json` first. Each file replacement is atomic; the pair is not one filesystem transaction.
   A second-file failure can leave a partial update. The current outer error path reports `changed = false`.
   Health reports need per-file outcomes rather than infer them from that boolean.
9. `files::imported` checks circuit/device IDs, parent, maximum rates, and network JSON.
   It does not currently compare all policy-critical fields, including subscriber addresses and minimum rates.
10. The runner checks import confirmation during normal cycles. Its existing failure limit is 40 minutes.
    A prompt follow-up needs a lightweight pending-update checker, not a faster RouterOS collection cycle.

## Health model

Keep these domains separate. A fresh telemetry response alone does not prove subscriber service availability.

| Domain | Inputs | Required distinction |
| --- | --- | --- |
| Service and config | Process identity, active config digest, cycle progress, config errors | Running with the old valid config versus accepting a new config |
| Collection | Same-cycle router/source outcomes and last-good fallback | Complete, failed, retained, stale, or intentionally disabled |
| Validation and write | Validation result, guard holds, file receipts | Blocked, unchanged, complete, partial, or failed |
| Apply | Desired write identity, import evidence, runtime readiness, native acknowledgement | Pending versus verified acceptance |
| Dataplane | Producer-reported bridge status, topology tier, classification state, source epoch | Healthy, degraded, unavailable, or recovering |
| Telemetry | Aggregate, bridge health, NIC, and host sampler metadata | Fresh, stale, warming, unavailable, or not applicable |
| Notification delivery | Queue state, retry time, last API receipt, permanent errors | Queued, retrying, accepted, failed, or disabled |

Each component carries its state, reason code, observation time, source epoch, applicability, and last successful observation.
Use structured reasons for deduplication. Router names and NIC port IDs remain part of identity.
Do not remove all digits from errors to identify distinct components.

Use separate update and health fields in the public report:

- `update_phase`: WRITTEN/PENDING, IMPORT_CONFIRMED, APPLIED, WRITE_FAILED, APPLY_FAILED, VERIFY_TIMEOUT, or SUPERSEDED.
- Domain health: HEALTHY, DEGRADED, FAILED, UNKNOWN, WARMING, or NOT_APPLICABLE.
- Overall presentation: HEALTHY, UPDATING, DEGRADED, FAILED, or UNKNOWN, with named reasons.

A normal pending update can show UPDATING while the previous dataplane generation remains healthy.
An apply-verification timeout can show a failed update and a healthy previous dataplane. It does not prove an Internet outage.
Missing required evidence prevents an all-healthy report. Disabled or inapplicable features do not create failures.
A disabled Hotspot source is not an offline Hotspot service.
The optional native-schema capability probe must not make a working pair target unhealthy.

Preserve existing source freshness limits: aggregate six seconds; bridge health, NIC, and host observations fifteen seconds.
Collection freshness follows its own sync-cycle budget.
A process change or uptime regression starts a new source epoch. Do not compare its counters with the previous epoch.
Idle traffic and a measured zero are valid observations. A polling worker at high OS CPU usage is not, by itself, a failure.

## Write and apply lifecycle

1. Allocate a durable `write_id` for a new logical update. Record expected file hashes and its cycle/config identity before replacement.
2. Record the outcome of each file: WRITTEN, UNCHANGED, or FAILED. Retain a partial result if only one replacement succeeds.
3. After a complete logical write, persist WRITTEN/PENDING with a health snapshot and enqueue its Telegram report immediately.
4. Request import through the existing scheduler mechanism. An enqueue or scheduler-request failure becomes explicit evidence.
5. Check pending acceptance outside RouterOS collection. Proposed cadence: two seconds while an update remains pending.
6. Publish a terminal receipt only from evidence for that exact `write_id`. Persist and enqueue its follow-up.

A durable identity must survive a LQoSync restart. `cycle_seq` alone is insufficient because it resets at process start.
A content hash alone is also insufficient: updates A to B to A are three distinct write operations.
Use hashes for desired-content comparison and a persisted operation identity for event tracking.

Every file-change event remains eligible even if overall health does not change.
Do not emit duplicate write events for unchanged periodic cycles.
A force request does not imply a physical write when the current file contents already match.

### Acceptance evidence

Strengthen import comparison to cover all fields that affect shaping and classification, including addresses and minimum rates.
Normalize those fields using established LibreQoS semantics. Preserve configured subscriber and node identities.

The installed runtime status exposes `source_generation`, `shaping_generation`, `shaping_content_generation`, and `ready`.
The checked host's transaction generation matched its runtime `shaping_generation`.
The native analytics state contract exposes the last acknowledged generation and the applied transaction.
These are different identifiers with different meanings. Do not treat them as interchangeable hashes.

APPLIED requires a stable correlation between this write's imported inputs and the native applied-state acknowledgement.
Check that input hashes and source identities did not change during the read.
Compare identities across the read and reject a bridge restart or a superseding update.
A generation string is evidence, not a locking or fencing primitive.

The read-only correlation contract passed against the installed producer. The verifier requires current input hashes, stable artifacts, producer validation, and matching native policy operations.
Do not infer APPLIED from `ready = true`, a generated transaction file, scheduler request success, or traffic volume alone.
If the required proof is unavailable, retain IMPORT_CONFIRMED or PENDING and expose the verification gap.

Do not call scheduler readiness helpers from every health poll without checking their effects.
The installed `verify_published_shaping_generation` path participates in world construction. This review did not execute it.

Retain the existing 40-minute import failure limit initially. The new checker must not silently shorten it or change shaping retry behavior.
A timeout follow-up must say that verification failed, not claim that the dataplane necessarily rejected the files.

### Concurrent updates and interruption

Do not block a newer shaping update while waiting for an older acknowledgement.
If a newer update replaces an unconfirmed one, close the older operation as SUPERSEDED and reference the new `write_id`.
Never mark the older operation APPLIED using the newer operation's acknowledgement.
A superseded update is an exceptional follow-up, not a false apply failure.

After restart, reconcile interrupted receipts with actual file hashes and current applied evidence.
A crash between rename and receipt persistence must not produce a fabricated success.
An unresolved interrupted operation stays UNKNOWN until evidence resolves it.

Shadow writes remain candidate updates. They must never produce a live APPLIED report.

## Event and delivery architecture

Separate latest health state from durable update events.
A latest-state channel may combine frequent telemetry updates. It must not discard distinct file-write events.
Health producers should publish small metadata receipts to an in-process aggregator.
Do not repeatedly parse complete host inventories merely to determine source freshness.
Publish one bounded public `system-health.json` summary for the TUI and status tools.
The renderer must not perform network requests or reimplement health decisions.

A single service-owned Telegram worker consumes a private, bounded disk outbox.
Persist events before delivery. Store message IDs and acknowledged parts after the Bot API accepts them.
A Telegram timeout or outage must not delay collection, shaping writes, or apply verification.
No notification failure may roll back a successful shaping update.

Preserve the existing subscriber-change detail options. Keep the health summary first and compact.
If a report needs parts, track delivery per part and resume only unacknowledged parts.
Keep each part valid HTML and within a conservative Telegram text budget. Test Unicode and escaped names.

Follow Telegram rate limits and the server's `retry_after` value.
Use bounded retry/backoff for transient failures. Authentication or permission failures need a visible delivery error and paused retries.
Keep a per-write predecessor order: its terminal report must not overtake its WRITTEN/PENDING report.
Record event time, health observation times, queue delay, and send attempt time. Mark delayed reports as historical observations.

Do not promise exactly-once chat delivery. A network timeout can occur after Telegram accepts a message but before the client receives its receipt.
Persist event identities and show the same `write_id` on retries so any duplicate remains recognizable.
A successful API receipt is not proof that a human read the message.

Implementation reserves 256 events and 8 MiB each for write events and general notices. One worker uses three-second request pacing.
At capacity, keep shaping active, record a visible notification-overflow fault, and report the delivery gap locally.
Do not claim unlimited retention or silently discard messages as delivered.
Paused or disabled Telegram routing must remain explicit. Do not automatically forward an old queue to a newly configured recipient.
Persist no bot token in report payloads. Keep payloads private because existing optional subscriber details can contain names and addresses.

## Notification policy

| Event | Telegram behavior |
| --- | --- |
| Complete managed-file update | Immediate WRITTEN/PENDING report with current health and changed-file list |
| Matching applied-state acceptance | APPLIED follow-up for the same write ID |
| Explicit write/apply failure | FAILED report with the failing stage and retained prior state |
| Verification deadline reached | Verification-failed follow-up; actual runtime acceptance remains unproven |
| Newer update replaces a pending update | SUPERSEDED follow-up naming the successor |
| No file changes, no health transition | No repeated healthy report |
| Sustained health fault without a file write | Recommended independent incident report |
| Recovery from an incident | Recommended recovery report with duration |

The owner selected the first two lifecycle reports. Independent incident/recovery reports are a recommendation for the unified-health feature.
Suppress repeated copies of the same incident. Use stable state transitions and a bounded reminder interval.
Group dependent symptoms when the evidence supports their shared cause. Keep independent sources visible.
Never suppress the required write or terminal reports because another health alert is active.

## Example reports

These examples are illustrative. They are not messages sent to the production chat.

```text
LQoSync3 — WRITTEN / PENDING
Write ID: W-042 | Mode: APPLY | Target: v3_pair
Files: ShapedDevices.csv WRITTEN; network.json WRITTEN
System: UPDATING
Collection: complete
Dataplane: healthy, previous acknowledged generation
Telemetry: aggregate LIVE; bridge LIVE; NIC LIVE; hosts LIVE
Apply: awaiting matching LibreQoS acknowledgement
Health as of: event observation time
```

```text
LQoSync3 — APPLIED
Write ID: W-042
Evidence: inputs matched; native applied-state acknowledgement matched
System: HEALTHY
Collection: complete | Dataplane: healthy
NIC: both ports UP; basic error increments 0 in the observed interval
Write-to-acceptance duration: 13 seconds
```

```text
LQoSync3 — APPLY VERIFICATION FAILED
Write ID: W-042
Files: written
Reason: matching acknowledgement unavailable before the deadline
Last verified applied write: W-041
Dataplane: healthy on the last observed generation
System: DEGRADED; latest update acceptance is unproven
```

## Performance and failure checks

Keep existing collector and telemetry schedules. No extra RouterOS reads are required for health aggregation.
Use the following performance rules in the implementation:

| Work | Proposed optimization | Required integrity condition |
| --- | --- | --- |
| Health aggregation | Reuse small in-memory receipts from existing samplers. Evaluate on state changes and scheduled freshness deadlines. | A stopped producer must become stale without another producer event. Do not read full subscriber inventories on each health tick. |
| Apply verification | Check only while an update is pending, at the proposed two-second interval. Stop after a terminal or superseded result. | Keep the existing forty-minute deadline initially. Use read-only acknowledgement evidence; do not call scheduler paths that construct a world. |
| Output comparison | Normalize expected values once per write from the already-rendered output strings. Reuse content hashes where available. | Include all policy-critical fields. Do not remove checks to reduce comparison cost. |
| Import parsing | Cache parsed evidence for the relevant file identity and generation. Invalidate on source, process, configuration, or update changes. | File metadata alone is not proof of matching content or acknowledgement. Reject mixed or stale evidence. |
| Health publication | Publish a bounded summary on meaningful changes. Readers calculate source age from observation timestamps. | Keep freshness deadlines active. Avoid rewriting a cache solely to advance a display countdown. |
| Telegram delivery | Use one worker. Prepare the event payload once and reuse it for retries. Persist delivery progress separately. | Keep distinct write events and per-part acknowledgements. A slow Telegram request must not hold the sync loop. |
| Queue and I/O | Bound pending work, disk usage, message size, and reason lists. Use one owner for persistent queue changes. | Expose capacity failures. Do not silently drop required write reports or create an unbounded task per sample. |

Coalesce replaceable telemetry snapshots, but preserve every distinct managed-file update event.
Subscriber collection must remain on the existing sync cycle.
The normal health tick should scale with the number of monitored components, rather than the subscriber count.
Subscriber-sized comparison work is still needed when relevant output or import evidence changes.
Do not add an external database, Alertmanager, or a connection pool before measurements show a need.

Measure an unchanged cycle, a changed cycle, and a Telegram outage with a restart.
Use representative subscriber and router counts, including the expected deployment maximum.
Compare cycle duration, health evaluation latency, CPU use, allocations, cache writes, queue age, and TUI render cost against the current baseline.
Check worst-case queue capacity and recovery, not only the average successful send.
Any blocking parse or durable disk operation must have bounded work; use a bounded blocking worker if profiling shows executor stalls.
These are proposed optimizations and checks. No performance gain has been measured for this unimplemented feature.

Required tests before deployment:

- File-pair change, network-only change, CSV-only change, and unchanged cycle.
- Partial replacement and scheduler-request failure.
- Immediate PENDING report and a later correlated APPLIED report without another file write.
- Import matched but native acknowledgement missing, wrong, or stale.
- Superseding writes, bridge restart, service restart, and interrupted receipt recovery.
- Disabled sources, missing optional telemetry, zero traffic, and a failed config reload that retains the active config.
- Telegram timeout, HTTP 429, API error envelope, invalid success body, permanent failure, and queue capacity.
- Partial multipart delivery, Unicode/HTML boundaries, and no secret exposure.
- Signal handling and cycle cadence while Telegram is unavailable.
- One health report projected consistently in TUI and Telegram, with independent source ages.

A local process cannot report its own host's total outage through a worker that is no longer running.
External host supervision is a later integration. Do not claim this feature provides it.
No automatic LibreQoS restart, router write, speed-profile migration, or WAN policy change is in scope.

## Suggested implementation order

1. Shared health model and write/apply receipts, including verified native acknowledgement correlation.
2. Persistent event outbox and asynchronous Telegram delivery for the selected two-stage flow.
3. TUI projection, failure-injection tests, performance checks, and actual terminal captures.

These are implementation proposals, not completed features. The present task ends with research and this reviewable design.

## Review verification

The 25 focused tests in `notify::tests` passed. They use test inputs and local mock transports.
They establish the current notifier baseline. They do not prove the proposed feature.
The writing checker produced heuristic suggestions. This document does not claim certified language compliance.
No production send, configuration edit, source-code edit, service restart, or deployment occurred.

## Research sources

- [Telegram Bot API: sendMessage](https://core.telegram.org/bots/api#sendmessage): text constraints and successful message receipts.
- [Telegram Bot API: ResponseParameters](https://core.telegram.org/bots/api#responseparameters): flood-control retry delay.
- [Telegram Bots FAQ: rate limits](https://core.telegram.org/bots/faq#my-bot-is-hitting-limits-how-do-i-avoid-this): per-chat and group limits.
- [Prometheus Alertmanager](https://prometheus.io/docs/alerting/latest/alertmanager/): grouping, deduplication, and inhibition concepts. No new service is proposed.
- [Google SRE: Monitoring Distributed Systems](https://sre.google/sre-book/monitoring-distributed-systems/): separate observed symptoms from unsupported causal claims.
- [LibreQoS applied-state contract at the reviewed revision](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/ipc.rs): acknowledged generation, applied transaction, and distinct content identities.

Local implementation references: `src/main.rs`, `src/notify.rs`, `src/runner.rs`, `src/files.rs`, `src/status.rs`, and existing sampler modules.
Installed-producer references: `/opt/libreqos/src/scheduler.py` and the schemas of the runtime/import/transaction artifacts.
