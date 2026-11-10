---
type: session
created: 2026-11-10
tags: [dpdk, telemetry, health]
status: implemented-not-deployed
related: ["[[2026-11-10-dpdk-worker-details]]", "[[HANDOFF]]"]
---

# DPDK statistics: task 4

Updated: 2026-11-10T20:13:44+08:00 (Asia/Shanghai).

The owner approved the next checkpoint and ran the root-only health probe.
Three samples reported healthy, no degraded subsystems, and zero AQM-degraded workers.
Uptime advanced from 44,962,108 to 44,966,115 ms. The single worker had observed rollup ages of 84–93 ms.
The source topology tier was empty. The UI displays unknown. It does not select a topology tier.

Evidence: `/home/pi/.local/state/lqosync3/dpdk-health-task4/live-source.jsonl`.
Probe SHA256: `01f7b073d6e88ab8590f4dc404a82ba124d41a47721b8050b8c05ef13a20fd61`.
The probe includes aggregate health only, with no subscriber or flow identity.

## Implementation

The current monitor requests read-only health every ten seconds on the analytics socket.
Due health and snapshot requests run concurrently under separate two-second deadlines.
The snapshot cadence stays two seconds. A failure in either source does not invalidate the other source.
Health requires an uptime baseline that advances. Duplicate uptime, failed reads, and fifteen-second expiry cannot claim LIVE.
A lower uptime starts a new baseline after a possible restart. Stale values remain last known.

The display does not extrapolate uptime or worker rollup age.

`src/worker_util/health.rs` owns bounded health decoding, aggregation, freshness, and display labels.
`src/worker_util/ipc.rs` reuses one bounded descriptor transport for snapshot and health requests.
Health replies require the health kind. Snapshot compatibility retains absent reply-kind support.

The public cache stores only an aggregate health summary. It retains old-cache compatibility.
Bounds cover strings, subsystem names, worker counts, duplicate IDs, and cache size.
Missing optional fields remain unknown. Zero and an empty subsystem list remain valid observations.
Unknown status text never becomes an invented healthy verdict.

Dashboard and Workers keep the approved composition and scroll controls.
Workers copy includes independent health freshness and whole-bridge scope.
Health remains available to copy when no worker row exists.
Captures: `docs/design/tui/captures/dashboard-health-v1.png` and `workers-health-v1.png`.
The isolated fixture deliberately reports degraded `stats_summary`. It does not represent the live probe.

## Verification and review

Initial regression tests failed before implementation. Final suite: 585 passed, zero failed, seven manual tests ignored.
Formatting, all-target Clippy with warnings denied, build, whitespace, source probe, and terminal capture review passed.
Tests cover request kinds, cadence, independent failures, cache compatibility, expiry, duplicate uptime, restart, optional fields, and stale copy.
The final focused transport suite passed after the fixture began to check exact request kinds.

The maximum fixture includes 4,096 workers, three full histories, and a full bounded health summary.
Cache fallback measured 742,194 bytes. One debug publish/read sample took 155.3 ms.
This is a synthetic measurement, not a production performance guarantee.
The collector adds one health request per ten seconds, no flow scan, dependency, runtime key, or monitor task.
No push, deployment, restart, or production configuration change occurred.

Next: Task 5 subscriber traffic, RTT, and retransmissions, then Task 6 circuit queue diagnostics.
Preserve the unrelated local-profile Knowledge link and note.
Source: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/lib.rs
