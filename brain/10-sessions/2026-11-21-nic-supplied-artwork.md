---
type: session
created: 2026-11-21
tags: [frontend, nic, svg]
status: implemented
related: ["[[2026-11-21-nic-live-diagnostics]]"]
---

# Supplied NIC artwork

The owner supplied `docs/frontend/images/nic-ports.svg` and `nic-svg-kit.zip`.
The archive SVG matches the standalone file byte for byte.
The UI now uses the supplied metallic RJ45, SFP, and QSFP shapes and gradients.
The original source files remain unchanged.

One static SVG fragment supplies the shared symbols. Each port uses its detected connector family.
The fragment excludes demo styles, instances, background, and the unused glow filter.
Unique ID prefixes avoid collisions. Shape classes support forced colors.

Unknown hardware keeps the generic connector.
Live link colors, finite RX/TX pulses, and the diagnostics panel keep their current behavior.

Vercel frontend-design and Google modern-web-guidance supplied the icon integration guidance.
A source comparison verifies the geometry and colors of all three connector families.
The NIC and diagnostics browser checks passed.
The web release build, Wasm Clippy, Rust 1.89 check, formatting, and whitespace checks passed.
See [screenshots and source details](../../docs/frontend/evidence/nic-svg-kit/README.md).

The operator can install the prepared web artifacts with `sudo tools/frontend/install.sh`.
This artwork update does not need a core binary replacement.
The earlier core update still supplies the one-second sampling and hardware identity.
Production binaries and services did not change during these checks.
