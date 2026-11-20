---
type: session
created: 2026-11-21
tags: [frontend, dashboard, nic, telemetry]
status: implemented
related: ["[[2026-11-21-nic-summary-placement]]", "[[2026-11-20-aggregate-traffic]]"]
---

# Live NIC diagnostics

The owner requested RX/TX updates alongside Actual, Mapped, and Unmapped traffic.
The owner also approved the hover/tap glass diagnostics panel from the supplied reference.

The core now targets one NIC counter sample each second.
The broker reads NIC evidence during the same one-second refresh as traffic.
Each port keeps its own activity counters. Independent producers keep their capture timestamps.

Hardware identity refreshes every thirty seconds. Failed NIC reads retry after ten seconds.
The collector captures counter timestamps before optional hardware probes to protect rate intervals.

The panel supports hover, keyboard, and tap. Escape works even when hover has no keyboard focus.
It shows measured-interval RX/TX bytes, packets, rates, recent trends, and health counters.
Counter details persist across source updates.
Unknown composition remains explicit. Driver shares remain approximate.

The panel uses the current theme, a translucent surface, and a solid fallback.
The TUI keeps its current presentation. The browser starts no extra telemetry poll.

The broker regression failed with the old NIC timestamp, then passed after the refresh change.
The browser check found a clock-anchor race in the new trend. The sample tracker now waits for the clock anchor.
Native checks cover the same race and hardware-probe timestamp delays.

Eighteen core NIC tests, nine native UI tests, and the web integration suite passed.
The HUD, NIC, DPDK, aggregate traffic, and sign-in/Dashboard browser checks passed.
Core/web releases, Wasm Clippy, Rust 1.89, formatting, whitespace, and image checks passed.
The STE checker remains heuristic. Technical terms received manual review.

See [the feature contract](../../docs/frontend/NIC_LIVE_DIAGNOSTICS.md).
See [pictures and checks](../../docs/frontend/evidence/nic-live-hud/README.md).
See [the atomic plan](../../docs/frontend/plans/2026-11-21-nic-live-hud.md).

Install the core and web artifacts with the documented NIC update procedure.
Production binaries, services, routing, and NIC bindings did not change during implementation.
The host's actual connector identity still needs verification through its installed collector.
