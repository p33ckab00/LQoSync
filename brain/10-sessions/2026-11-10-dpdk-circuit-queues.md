---
type: session
created: 2026-11-10
tags: [dpdk, telemetry, subscribers, queues]
status: implemented-not-deployed
related: ["[[2026-11-10-dpdk-subscriber-observations]]", "[[HANDOFF]]"]
---

# DPDK statistics: task 6

Updated: 2026-11-10T21:30:00+08:00 (Asia/Shanghai).

The owner approved the next checkpoint and ran the read-only circuit probe.
Three samples returned one matching shaping slot, eight tins per direction, and fifty RTT bins with eight samples.
Sample sequences were 243961, 243971, and 243981. The epoch stayed stable.
Elapsed time and cadence reported 200 ms. Gap/reset flags were false.

These short observations do not establish long-term health or absence of congestion.

Evidence: `/home/pi/.local/state/lqosync3/dpdk-circuit-task6/live-source.jsonl`.
SHA256: `ee882dc464805eaaf91432bca8d5906dc9c5eecc2d8bab5d9df24d29b539b472`.
The probe prints aggregate values without circuit names, IDs, or IPs.

## Implementation

Subscribers adds a manual queue action with the `d` shortcut and mouse activation.
The existing detail pane and copy action show directional depth, drops, marks, tins, and a pooled RTT histogram.
Reported p99 sojourn converts from nanoseconds to milliseconds. It is separate from host RTT.
The unit of `memory_used` has no verified contract. The view labels the source value as raw.

Missing fields remain unknown. Age, gap/reset, and source clock labels remain explicit.
A persistent detail title retains queue state during scrolling. Results remain bound to the requested circuit.

The TUI owns one cancellable background worker. A two-second cooldown limits repeated requests.
The worker reads state/queue_stats/state for one nonzero slot. Generation and slot order must remain stable.
Each request keeps the two-second deadline and 2 MiB bound. The job has a seven-second deadline.

Socket access uses the current user's permissions. This host needs a root TUI for live queue reads.
The change adds no broker, permission change, automatic queue poll, cache writer, or dependency.

## Verification

606 tests passed. Zero failed. The suite ignores seven manual tests.
Formatting, all-target Clippy with warnings denied, build, whitespace, live probe, and capture review passed.
Tests cover mapping changes, slot identity, unknown values, source age, and malformed histograms.
They also cover request limits, cancellation, selection, copy, and mouse activation.

A fixture with sixteen tins per direction and fifty nonempty bins produced 5,212 bytes of detail text.
One thousand debug formats took 61.0 ms. This synthetic result is not a production guarantee.

Actual terminal captures use an isolated synthetic socket through the real worker and CBOR transport.
They preserve the approved Subscribers composition. The collection-only queue control stays disabled.
Files: `docs/design/tui/captures/subscribers-queues-default-v1.png` and `subscribers-queues-v1.png`.
Contract and review: `docs/design/tui/DPDK_STATS.md`, Task 6.
Check logs: `/home/pi/.local/state/lqosync3/dpdk-circuit-*.log`.

## Handoff

All six DPDK implementation checkpoints now exist locally on `lqosync3`.
No push, deployment, restart, or production configuration change occurred.
Preserve the unrelated local-profile Knowledge link and note.
Graph data remains an earlier reference. Current code and this checkpoint review are authoritative for Task 6.
