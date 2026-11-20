---
type: session
created: 2026-11-20
tags: [frontend, dashboard, diagnostics, accessibility]
status: verified
related: ["[[2026-11-20-live-traffic-capacity]]", "[[2026-11-20-dashboard-reference-alignment]]"]
---

# Service diagnostics banner

Timestamp: 2026-11-20T21:02:39+08:00 (Asia/Shanghai).
Agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
Previous commit: `45dad14`.

The owner requested a stock-exchange-style banner for synchronization and component health.
A Rust component now shows the current overview values on a stable CSS transform track.
The existing stream supplies all data. This change adds no network request or backend operation.

The banner has a keyboard Pause/Resume control. Hover and focus also pause movement.
Source values continue to update while motion pauses.
Reduced motion gives a static wrapped list and hides the duplicate loop.
The duplicate also has `aria-hidden="true"` in motion mode.
Full static diagnostics remain available under View diagnostics.
Component keys include position and name. Repeated labels keep separate states.

Checks: Wasm release build, Wasm Clippy with warnings denied, and Rust 1.89 check passed.
Browser checks cover motion, stable nodes, keyboard/hover pause, reduced motion, repeated names, long labels, and stale states.
Light/Dark and 768/320px layouts passed. The existing Dashboard and sign-in smoke check passed.
No core, gateway, or TUI code changed. The core test suite was not repeated.

Vercel frontend-design keeps the current visual design.
Google modern-web-guidance `motion` guides transform animation, the pause control, and reduced motion.
See [design and plan](../../docs/frontend/DIAGNOSTICS_TICKER.md).
See [browser evidence](../../docs/frontend/evidence/diagnostics-ticker/README.md).

Runtime step: run `sudo tools/frontend/install.sh` from the repository.
The assets are built but are not installed by this session.
Preserve unrelated November 16 notes and the existing dataplane report.
