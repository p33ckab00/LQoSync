---
type: session
created: 2026-11-10
tags: [dpdk, telemetry, classification]
status: implemented-not-deployed
related: ["[[2026-11-10-dpdk-queue-statistics]]", "[[HANDOFF]]"]
---

# DPDK statistics: task 2

Updated: 2026-11-10T18:27:59+08:00 (Asia/Shanghai).

The owner approved the next checkpoint and asked about bridge health.
Bridge health, uptime, and degraded subsystems remain Task 4. They were not omitted.
Task 2 adds mapped and unmapped traffic to the local implementation.
Task 1 remains in local commit `64528e9`. Neither task changed the running service.

## Source and semantics

The source defines mapped/unmapped whole-bridge byte counters at enqueue, before capped host-row limits.
Actual remains delivered traffic. Class totals never come from Actual subtraction or host/flow sums.
Mapped means a circuit resolved for the subscriber IP. Unmapped means best effort, not proof of unauthorized use.
Source: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/lib.rs

The operator ran the read-only aggregate probe. Both counters exist and producer timestamps advance.
Measured download intervals: mapped 98.683 and 96.296 Mbps, unmapped 0.055 and 0.031 Mbps.
Evidence: `/home/pi/.local/state/lqosync3/dpdk-mapping-task2/live-source.jsonl`.
The probe prints no subscriber or flow identities.

## Changes

`src/worker_util/classification.rs` owns two independent bounded histories and class labels.
The current poll/cache reads optional counters. The transport, deadline, request limit, and cadence stay unchanged.
Each class reuses `traffic::History` for gaps, timestamps, resets, and rates.
Duplicate producer frames cannot replay values. Absent fields stay unavailable.
A source failure clears rate baselines and retains labeled last-known totals.
Corrupt histories do not hide valid workers or the other class.

Dashboard has Actual/Mapped/Unmapped buttons and `v` navigation.
Time windows follow the selected source. Low-rate charts use a sub-Mbps scale.
Health/worker detail panes and copy output retain scope and freshness labels.
Draw uses cached values and only the selected history. No WAN attribution or per-flow scan exists.

## Verification and review

Initial regression tests failed before the collector/UI implementation.
Final checks: 568 tests passed, zero failed, seven manual tests ignored.
Formatting, all-target Clippy with warnings denied, debug build, whitespace, and PNG checks passed.
The CBOR test retains `limit=1` and verifies both class counters through the real decoder.
Tests cover independence from Actual/host rows, missing classes, zeros, stale copy, resets, errors, history bounds, and UI controls.
Theme/size checks cover three themes and compact layouts. Mouse targets stay within the visible panel.

The maximum-cache fixture has 4,096 workers and three 1,801-point histories.
It measured 740,161 bytes, 35.9 ms encode, and 26.1 ms read/decode in one debug run.
These synthetic results are not a production latency or capacity guarantee.
The change adds no dependency, router scan, poll task, or runtime key.

Captures: `docs/design/tui/captures/dashboard-mapped-v1.png`, `dashboard-unmapped-v1.png`, and `workers-classification-v1.png`.
They use isolated synthetic records and preserve the approved panel order.
Logs: `/home/pi/.local/state/lqosync3/dpdk-mapping-*.log`.

No push, deployment, restart, or production configuration change occurred.
Next: Task 3 worker measurements, then Task 4 bridge health.
Preserve the unrelated local-profile discussion and the capture-index edit.
