# DPDK statistics delivery plan

Date: 2026-11-10. Branch: `lqosync3`.

## Task checkpoints

1. Queue totals: drops, AQM marks, and backlog bytes. Implement this checkpoint first.
2. Mapped and unmapped traffic.
3. Additional worker measurements.
4. Bridge health and uptime.
5. Subscriber traffic, RTT, and retransmissions.
6. On-demand circuit queue diagnostics.

Each checkpoint needs its own tests, performance/integrity review, documentation, and local commit.
Do not deploy or publish a release as part of checkpoint 1.

## Checkpoint 1 contract

Extend the existing two-second analytics snapshot read. Keep `limit=1`, deadlines, size bounds, and one monitor.
Read optional `qdisc_totals`: `drop_pkts`, `aqm_marks`, and `backlog_bytes`.
Keep absent fields unknown. The values describe the whole bridge, not one worker, WAN, or subscriber.
Counters show cumulative totals and increments since the previous valid sample. Backlog is a current gauge.
Reset baselines after read failure, expiry, producer timestamp regression, or counter decrease.
Repeated producer snapshots cannot replay increments. Old caches remain readable.
Do not add drops from unrelated layers or treat AQM marks as packet drops.

## Files and proof

- `src/worker_util/queue.rs`: optional totals, checked increments, labels, and focused tests.
- `src/worker_util.rs`: snapshot/cache fields and baseline lifecycle tests.
- `src/tui/overview.rs`: queue measurements within the existing health panel, with bounded scrolling.
- `src/tui/workers/approved.rs`: whole-bridge queue measurements in the existing scrollable details.
- Existing fixture constructors and capture generator: additive queue fixture values.
- `docs/OPERATIONS.md` and design contract: scope, units, missing/stale/reset behavior.

Write regression tests first. Confirm failure, then implement the narrow path.
Run focused tests, full tests, formatting, Clippy, and debug build.
Inspect synthetic terminal captures and narrow/theme rendering. Review the final diff.
Preserve pre-existing edits in `brain/Home.md`, the local-profile note, and the capture index.

## Source and live verification

The LibreQoS analytics contract at `9ebe29bc` defines `QdiscTotals` and the snapshot field `qdisc_totals`.
Source: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/lib.rs
The installed analytics socket is root-only. Non-interactive sudo requires a password.
The operator ran the read-only probe. Three samples contain all three queue fields and advancing timestamps.
Live source proof: `/home/pi/.local/state/lqosync3/dpdk-queue-task1/live-source.jsonl`.
Source verification passed. Service deployment remains separate.


## Checkpoint 1 result

Implemented and locally verified. 560 tests passed, zero failed, seven manual tests ignored.
Formatting, all-target Clippy, debug build, whitespace, and capture review passed.
The operator's live probe verified field support and producer progress.
The existing poll and bounded cache remain the only data path.
No production deployment, restart, or push occurred. Task 2 is the next checkpoint.

## Checkpoint 2: mapped and unmapped traffic

Use the writing-plans and test-first workflow for this checkpoint.
Read the optional whole-bridge `mapped` and `unmapped` byte counters from the same snapshot.
The source counts these bytes at enqueue. The Actual chart continues to use delivered bytes.
Never infer unmapped bytes by subtraction from Actual or by summing capped host/flow rows.

- Add a classification module with two bounded histories. Reuse `traffic::History` for rate calculations and resets.
- Integrate optional snapshot fields and backward-compatible cache fields in `src/worker_util.rs`.
- Verify counters and producer progress with a read-only aggregate probe.
- Add Actual/Mapped/Unmapped chart selection and scope labels in Dashboard.
- Add classification measurements to the scrollable health/worker detail panes and copy output.
- Keep counters and history independent for each class. Missing fields, stale evidence, resets, and gaps remain explicit.
- Test source decoding, reset/duplicate/failure behavior, old and invalid caches, cache size, UI controls, and stale copy output.
- Review captures, performance, integrity, and the final diff. Commit locally after checks pass.

Each class retains at most 1,801 points and one hour. The public cache remains bounded at 1 MiB.
The transport retains its 2 MiB snapshot bound, two-second deadline, `limit=1`, and one monitor.
No router read, subscriber scan, WAN attribution, service restart, deployment, or push belongs to this checkpoint.
Task 4 still owns bridge health, uptime, and degraded-subsystem diagnostics.


## Checkpoint 2 result

Implemented and locally verified. 568 tests passed, zero failed, seven manual tests ignored.
Formatting, all-target Clippy, debug build, whitespace, and terminal capture review passed.
The operator's aggregate probe verified both source classes and producer progress.
The maximum-cache fixture remains below 1 MiB with three full histories and 4,096 workers.
No push, deployment, or restart occurred. Task 3 is next. Bridge health remains Task 4.

## Checkpoint 3: worker details

Keep one analytics snapshot poll. Read optional `idle_ppm`, `laps_per_s`, `busy_laps`,
`cycles_per_busy_lap`, and `phase_cycles` from each worker row.
The source declares ten cumulative phase counters in this order:
rx, dissect, select, charge, leaf, tx, begin_cycle, drop_overdue, handoff, empty_rx.
Do not label laps/s as packets/s or cumulative phase totals as current interval utilization.
Keep derived idle from busy separate from reported idle. Absent fields stay unknown.
Sanitize an inconsistent reported idle or an unknown phase-vector length without hiding valid busy/backlog data.

- Add optional row fields and detail formatting in the existing worker telemetry path.
- Add source/cache tests before implementation, then verify the live aggregate worker rows with a read-only probe.
- Add metrics to the scrollable Workers detail pane and copy output. Retain the approved panel order.
- Keep basic telemetry and histories within the 1 MiB public cache bound.
  If extended rows exceed that budget, publish basic rows with an explicit omission notice.
  Preserve all extended values in memory. Never publish a cache that old readers must reject for size.
- Test unsupported fields, bad idle/phase data, duplicate samples, stale labels, copy, and cache fallback.
- Verify mouse focus and keyboard scrolling for the longer detail pane.
- Run the full suite, Clippy, formatting, build, captures, performance/integrity review, and local commit.

No health endpoint, new poll, dependency, runtime key, router write, deployment, restart, or push belongs to this checkpoint.
Task 4 owns bridge health, uptime, and degraded subsystems.


## Checkpoint 3 result

Implemented and locally verified. 576 tests passed, zero failed, seven manual tests ignored.
Formatting, all-target Clippy, debug build, and terminal capture review passed.
The operator probe verified all five optional fields, ten phases, and advancing producer timestamps.
Reported laps/s equaled busy ppm in the three samples. Keep its producer-reported label; do not infer packets/s.
Cache fallback preserves basic telemetry and histories within 1 MiB without mutating the source report.
The maximum fallback fixture measured 739,546 bytes and 154.6 ms for one debug publish/read sample.
No push, deployment, or restart occurred. Task 4 bridge health, uptime, and degraded subsystems is next.

## Checkpoint 4: bridge health

Use the read-only health request on the existing analytics socket.
Keep one monitor. Request health every ten seconds, with the existing two-second transport deadline.
Run due health and snapshot requests concurrently. A failed health request cannot invalidate valid traffic or workers.
Health has its own update time, producer-uptime progress check, and fifteen-second expiry.
Do not infer bridge health or uptime from a snapshot timestamp or operating-system CPU load.

- Reuse the bounded CBOR/descriptor transport. Require the correct health reply kind.
- Keep aggregate health only: status, version, uptime, topology tier, classification state, named degraded subsystems, and worker rollup/AQM summary.
- Preserve missing fields as unknown and zero as a valid value. Bound strings, lists, and worker counts.
- Test decoding, old reports, unknown statuses, malformed values, duplicates, failures, expiry, restart, and cache compatibility.
- Verify three live aggregate reads through an operator-run root probe.
- Add health to the existing scrollable Dashboard and Workers panes and copy output.
- Keep approved composition, mouse scrolling, and independently labeled health freshness.
- Update captures, review performance/integrity, run full checks, and commit locally.

No supervisor action, restart policy, flow scan, router read, runtime key, push, deployment, or restart belongs to this checkpoint.

## Checkpoint 4 result

Implemented and locally verified. The operator's three read-only samples reported healthy with advancing uptime.
No degraded subsystems or AQM-degraded workers were reported. The source topology tier was empty.
The UI retains unknown topology, independent freshness, and last-known states.
585 tests passed, zero failed, seven manual tests ignored. Formatting, Clippy, build, and terminal capture review passed.
The maximum cache fixture measured 742,194 bytes after fallback with full histories and bounded health.
One debug publish/read sample took 155.3 ms; this is not a production guarantee.
No push, deployment, or restart occurred. Task 5 subscriber traffic, RTT, and retransmissions is next.
