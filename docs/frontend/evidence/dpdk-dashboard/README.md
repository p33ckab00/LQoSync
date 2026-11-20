# DPDK Dashboard evidence

Date: 2026-11-20 (Asia/Shanghai). Branch: `lqosync3`. Previous commit: `ca9065e`.
See [design, sources, bounds, and implementation plan](../../DPDK_DASHBOARD.md).

## Checks

- Eight projection tests passed. They cover unknown versus zero, independent freshness, future timestamps, invalid data, corrupt caches, resets, and bounds.
- The largest fixture uses 4096 workers, eight ports, extended fields, large counters, thirty-two health subsystems, and 300 traffic points.
- It retains sixteen worker rows, bounds DPDK details to sixteen KiB, and keeps the full overview below 60000 bytes.
- Six native UI tests and the existing gateway tests passed.
- The browser verified worker bars, mean busy, the session chart, live changes, and keyboard access and retained open details/selection.
- Independent stale, degraded, link-down, reset, unavailable, and connection-failure states passed.
- A simulated one-hour client clock difference did not change source freshness.
- Light/Dark, reduced motion, forced colors, long labels, and 768/320px checks passed.
- Existing sign-in/Dashboard and rolling-ticker regression checks passed.
- Native web and Wasm release builds, Clippy with warnings denied, Rust 1.89, formatting, and whitespace checks passed.

The projection uses the same published telemetry as the TUI.
A read-only probe of this host found one worker and two NIC ports.
The source states were live analytics, healthy bridge, and live NIC telemetry at that check.
One debug projection took approximately 24 ms.
This single measurement does not establish production latency.

## Pictures

These screenshots use synthetic fixture data.
The fixture deliberately offsets the browser clock by one hour.
The source ages still follow the server timestamp.
No production subscriber records or account credentials appear.

- [Dark](dark.png)
- [Light](light.png)
- [Mobile](mobile.png)

Run `sha256sum -c SHA256SUMS` in this directory to check the images.

## Runtime and limits

The release binaries and browser assets are prepared.
Install with `sudo tools/frontend/install.sh`.
This session did not replace production files or restart services.
Core collectors, TUI code, and account administration remain unchanged.

Worker history belongs to the current browser session and holds at most sixty distinct fresh samples.
Longer retention and a full paged Workers inspector require additional backend contracts.
