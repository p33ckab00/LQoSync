---
type: session
created: 2026-11-21
tags: [frontend, dashboard, nic]
status: implemented
related: ["[[2026-11-20-nic-faceplate]]"]
---

# NIC summary placement

Move the NIC faceplate into the service summary, beside PPPoE and LibreQoS.
The faceplate appears once, before the traffic charts.
DPDK diagnostics keep workers, pressure, raw counters, and their source labels.
Both sections share one snapshot and one server-time clock.

The layout keeps the current artwork, colors, port identity, and RX/TX pulse behavior.
Vercel frontend-design and Google modern-web-guidance supplied layout guidance.
CSS Grid adapts the summary to six, three, or two columns.
The NIC card spans both columns at widths of 1200px or less.
The cadence description remains accessible to screen readers.

The NIC and DPDK browser checks passed.
They check clock differences and independent source states.
The sign-in/Dashboard and aggregate chart checks passed.
The web release build, Wasm Clippy, Rust 1.89 check, formatting, and whitespace checks passed.
The 1024px check found wrapped ports. A wider two-column breakpoint corrected the layout.
See [screenshots and checks](../../docs/frontend/evidence/nic-summary/README.md).

Production services did not change. Install the prepared web artifacts with `sudo tools/frontend/install.sh`.
Full hardware detection still needs the earlier core collector update if it was not installed.
The project graph predates this UI work. A targeted query supplied only older NIC and TUI relationships.
Current source files and browser checks establish this change.
