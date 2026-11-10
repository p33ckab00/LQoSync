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
