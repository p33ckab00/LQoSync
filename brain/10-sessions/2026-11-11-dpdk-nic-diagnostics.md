---
type: session
created: 2026-11-11
tags: [dpdk, nic, telemetry, tui]
status: implemented-not-deployed
related: ["[[2026-11-11-mirrored-dpdk-chart]]", "[[HANDOFF]]"]
---

# Native DPDK NIC diagnostics

Updated: 2026-11-11T01:03:51+08:00 (Asia/Shanghai).

The owner prioritized NIC diagnostics before chart inspection.
The operator ran the read-only Python probe because the runtime directory requires root access.
The probe found one telemetry v2 socket at `/run/dpdk/rte/dpdk_telemetry.v2`.
The peer was `bridge_rs`, with DPDK 25.11.0. Ports 0 and 1 identify PCI devices `0000:01:00.0` and `0000:01:00.1`.
Each port reported UP, 10,000 Mbps, full duplex, and 78 extended statistics in three samples.
Basic RX/TX errors, missed packets, and no-mbuf counters were zero in those samples.
This establishes the sampled source shape. It does not establish long-term health.

## Implementation

`src/nic_stats.rs` owns the model, baseline, bounded public cache, and separate ten-second task.
`src/nic_stats/ipc.rs` reads native SOCK_SEQPACKET JSON with a three-second deadline and 64 KiB reply limit.
Port IDs and device names are checked again before publication. Peer PID and port identity guard interval baselines.
The report keeps at most eight ports and 256 extended statistics per port. The cache limit is 1 MiB.
The configured socket is `libreqos.v3_dpdk_telemetry_socket`. Its default matches the verified host.
The service does not scan runtime instances during normal collection.

Basic counters retain raw totals and safe interval increments. Failures, gaps, resets, process changes, and identity changes rebase them.
Extended statistics retain raw values only. Vendor gauges can decrease without resetting basic counters.
Missing values remain unknown. Stale data cannot supply current link states or increments.
NIC RX/TX is separate from subscriber traffic. Link speed is separate from WAN capacity.

Dashboard Dataplane health shows summary rows. Workers details and Copy include bounded extended statistics.
Existing keyboard, mouse wheel, and scrollbar behavior remains in use. The traffic chart and approved panel order remain unchanged.
Fresh DOWN links and positive basic error increments produce Operational alerts.
Unavailable NIC data stays explicit in Dataplane health without duplicate default alerts.

The service monitor owns and aborts the NIC task. Pending obsolete reads cannot publish after reload.
Analytics and NIC settings comparisons are separate. A NIC-only setting change preserves analytics histories and subscriber baselines.
The TUI reuses parsed reports by file stamp. Frame rendering performs no source reads.
No dependency, permission change, packet capture, router change, or packet-path work was added.

## Evidence and review

632 tests passed, with zero failed and seven manual tests ignored. Formatting, all-target Clippy, build, and whitespace checks passed.
Tests include source replay, malformed/oversized replies, optional endpoints, caps, identity changes, resets, gauges, cache bounds, reload, cadence, shutdown, copy, and cache reuse.
A compact Dashboard regression initially failed when a duplicate NIC-unavailable alert displaced router outcomes.
The duplicate alert was removed. NIC unavailability stays in Dataplane health. The regression and full suite now pass.

The maximum cache fixture uses eight ports and 256 long statistic names each. Its cache size was 312,567 bytes.
One hundred debug detail formats took 90.7 ms. This does not guarantee terminal frame rate.
Logs: `/home/pi/.local/state/lqosync3/dpdk-nic-*.log`.
Live source and probe checks: `/home/pi/.local/state/lqosync3/dpdk-nic-task7/`.
Repository replay fixture: `tests/fixtures/dpdk-nic-source.json`. It contains numeric counters and PCI identity, without subscriber data.

Actual terminal captures use isolated synthetic records: `dashboard-nic-v1.png` and `workers-nic-v1.png`.
Visual and source contract: `docs/design/tui/NIC_DIAGNOSTICS.md`.

## Deployment state

The feature is local and uncommitted. Production remains `3.0.172+gb6778cb`.
No push, deployment, service restart, or configuration change occurred for this task.
Preserve the unrelated local-profile Knowledge note and the pre-existing `brain/Home.md` change.
