---
type: session
created: 2026-11-20
tags: [frontend, dashboard, diagnostics]
---

# Continuous diagnostics ticker

The owner requested continuous movement when activity updates arrive.
The previous percentage transform and variable duration changed the visible position.
The new browser regression reproduced an 83-pixel jump.

The ticker now moves one measured item at a time at 32 pixels per second.
New activity appends to the tail.
An item receives updated text after it leaves the left edge.
Expired waiting entries leave beyond the right edge.
Visible text finishes its pass before removal.
The moving queue holds at most 18 entries.

The accessible list and static diagnostics receive source changes immediately.
Reduced motion remains static. The owner still requires no pause control.
Vercel frontend-design and Google modern-web-guidance supplied the presentation guidance.

The insertion regression now passes.
A handoff test and fourteen repeated handoffs check continuity and viewport coverage.
Focused browser checks, existing smoke checks, Wasm release, Clippy, and Rust 1.89 checks passed.
No core, backend, TUI, or dependency change was needed.
The core suite was not repeated.

See [design](../../docs/frontend/DIAGNOSTICS_TICKER.md) and [evidence](../../docs/frontend/evidence/diagnostics-continuous/README.md).
The web build is prepared. Run `sudo tools/frontend/install.sh` to install it.
Production assets and services were not changed.
