# Live NIC diagnostics plan

1. Read NIC counters in the one-second web refresh. Prove that a changed cache reaches the same overview as traffic.
2. Poll NIC counters each second. Keep a ten-second failure retry and a thirty-second identity refresh.
3. Add a glass diagnostics panel for hover, keyboard, and tap. Use the measured interval for rates.
4. Verify source independence, stale/reset states, responsive layout, release builds, and browser behavior. Save evidence and commit.

Keep physical RX/TX separate from aggregate delivered and classification enqueue measurements.
Use the same transport cadence. Do not claim identical capture timestamps across independent producers.
Keep the current Rust application, palette, typography, and adjacent physical ports.
Use a translucent panel with a thin border, subtle blur, and a solid fallback.
The panel must fit the viewport. Show missing counters as unknown.
