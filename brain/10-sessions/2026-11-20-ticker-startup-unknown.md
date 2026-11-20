---
type: session
created: 2026-11-20
tags: [frontend, diagnostics, regression]
---

# Ticker startup placeholders

The owner reported that every moving status showed Unknown.
The core, broker, and gateway services were active.
The status cache had mode, target, and cycle duration values.
The health cache separately reported degraded health.

The moving queue captured placeholders before the first monitoring response.
The browser regression reproduced ten Unknown entries while the static list already had valid values.
The queue now waits for a timestamped snapshot.
The banner shows “Waiting for service status…” until then.
Both moving status sets start with the source values.

Vercel frontend-design and Google modern-web-guidance supplied the presentation guidance.
The change preserves continuous movement, activity append, reduced motion, and the owner's no-pause requirement.
It does not replace genuine unknown or degraded source values.

The delayed-response regression and existing ticker continuity checks passed.
Existing sign-in/Dashboard smoke, Wasm release, Clippy, Rust 1.89, formatting, and whitespace checks passed.
No core, backend, or TUI code changed. The core suite was not repeated.

See [evidence](../../docs/frontend/evidence/diagnostics-warmup/README.md).
Install the prepared web assets with `sudo tools/frontend/install.sh`.
Production files and services were not changed.
