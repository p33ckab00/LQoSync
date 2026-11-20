---
type: session
created: 2026-11-20
tags: [frontend, dashboard, diagnostics]
---

# Rolling diagnostics strip

The owner requested a rolling banner without a card or pause control.
The strip has no heading or subtitle.
Movement continues on hover.
The device reduced-motion setting shows a static list.

The five permanent fields and temporary activity rules remain.
The full static diagnostics remain available below the main panels.
No backend, core, or TUI change was needed.

Vercel frontend-design and Google modern-web-guidance supplied the design guidance.
The explicit owner requirement overrides Google's visible pause control recommendation.
The design document records this accessibility limit.

Release Wasm, Clippy, Rust 1.89, formatting, and whitespace checks passed.
Focused browser checks and the existing sign-in/Dashboard smoke check passed.
See [design](../../docs/frontend/DIAGNOSTICS_TICKER.md) and [screenshots](../../docs/frontend/evidence/diagnostics-strip/README.md).

The web build is prepared. Run `sudo tools/frontend/install.sh` to install it.
Production assets and services were not changed.
