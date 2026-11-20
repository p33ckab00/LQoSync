---
type: session
created: 2026-11-20
tags: [frontend, dpdk, telemetry]
---

# DPDK Dashboard card

The owner requested a Dashboard card with the same DPDK data sources as the TUI.
The card uses the published worker-utilization and NIC telemetry caches.
The TUI, core collectors, and CLI account policy remain unchanged.

The card includes worker busy/idle bars, a recent mean-busy chart, pressure gauges, bridge health, and NIC diagnostics.
The worker total and mean use all validated workers.
The response includes sixteen busiest workers, eight ports, and sixteen xstats per port.
Optional row details have a sixteen-KiB budget. Source health and pressure verdicts remain available.

Worker analytics expire after six seconds.
Bridge health and NIC telemetry have independent fifteen-second limits.
Source ages use server time and elapsed time after receipt.
A browser test uses a one-hour device clock difference.
Unknown values, reported zero, stale evidence, reset counters, and degraded health stay distinct.

The session chart keeps sixty distinct fresh samples.
It preserves gaps and clears history on a backward source timestamp.
Longer retention and a full paged Workers inspector need additional backend contracts.
See [design and atomic plan](../../docs/frontend/DPDK_DASHBOARD.md).

Vercel frontend-design and Google modern-web-guidance supplied the interface guidance.
The existing authenticated stream carries the optional DPDK projection.
The broker reuses the one-second worker read and reads NIC evidence during the five-second full projection.

Eight projection tests, six native UI tests, and the existing gateway tests passed.
The DPDK browser test, existing Dashboard/sign-in smoke, and ticker regressions passed.
Native web and Wasm release builds, Clippy, Rust 1.89, formatting, and whitespace checks passed.
See [screenshots and evidence](../../docs/frontend/evidence/dpdk-dashboard/README.md).

A read-only projection of the host caches found one worker and two ports.
It reported live worker/NIC data and healthy bridge status at that check.
One debug projection took approximately 24 ms. This is not a production benchmark.

The release binaries and assets are prepared.
Install with `sudo tools/frontend/install.sh`.
Production files and services were not changed in this session.
