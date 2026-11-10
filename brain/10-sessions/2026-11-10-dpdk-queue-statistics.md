---
type: session
created: 2026-11-10
tags: [dpdk, telemetry, queues]
status: implemented-not-deployed
related: ["[[2026-11-09-worker-utilization]]", "[[2026-11-09-buffer-pressure]]"]
---

# DPDK statistics: task 1

Updated: 2026-11-10T18:07:51+08:00 (Asia/Shanghai).

The owner requested all proposed DPDK statistics, with separate task checkpoints.
The plan is `docs/superpowers/plans/2026-11-10-dpdk-statistics.md`.
Task 1 adds aggregate queue drops, AQM marks, and backlog bytes.
Tasks 2 through 6 remain pending. The next task is mapped/unmapped traffic.

## Implementation

The same snapshot poll reads optional `qdisc_totals` and publishes an additive `queue` cache field.
Counter baselines reset after errors, timestamp regression, gaps, and counter decreases.
Repeated frames do not replay increments. Backlog is a current byte gauge.
Old caches remain readable. Absent values stay unknown.
Stale display/copy retains labeled totals but suppresses current increments.
Queue values describe the whole bridge. They are not per-worker or per-WAN values.

Dashboard adds queue values inside its health panel with exact wrapped scrollbar bounds and `j`/`k` navigation.
Workers adds them inside the current detail pane and copy output. Approved panel order stays unchanged.
The Ratatui feature for rendered-line information supplies exact wrapped line counts.
This changes no runtime dependency or lockfile entry.

## Source evidence

The operator ran the read-only `examples/dpdk_queue_probe.rs` binary against the root-only socket.
All three samples contain the three fields. Producer timestamps advance.
Observed values: zero cumulative drops, 13,899 cumulative marks, backlog 95,550 → 6,526 → 0 bytes.
Evidence: `/home/pi/.local/state/lqosync3/dpdk-queue-task1/live-source.jsonl`.
No host identity or flow data appears in that evidence.

## Review and checks

Regression tests failed before implementation for cache totals and UI/copy output.
Tests cover partial fields, resets, stale values, counter limits, repeated frames, source errors, and scrolling.
The CBOR/descriptor test verifies the source fields through the actual IPC decoder.
Final checks: 560 tests passed, zero failed, seven manual tests ignored.
Formatting, all-target Clippy with warnings denied, debug build, whitespace, and PNG decoding passed.
Final check logs: `/home/pi/.local/state/lqosync3/dpdk-queue-*.log`.
Captures: `docs/design/tui/captures/dashboard-queue-v1.png` and `workers-queue-v1.png`.
The captures use synthetic data. They preserve the approved screen composition.

Performance review: three aggregate fields reuse the current request and producer rollup.
The change adds constant-size baseline state and fixed-size display text.
Draw reads memory, not sockets or router APIs. No per-flow scan or new poll task exists.
This review does not establish production latency under every workload.

No push, deployment, restart, routing change, or shaping change occurred.
The local commit excludes the pre-existing capture-index edit and local-profile discussion changes.
