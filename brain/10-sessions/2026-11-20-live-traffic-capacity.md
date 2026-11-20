---
type: session
created: 2026-11-20
tags: [frontend, dashboard, traffic, capacity]
status: verified
related: ["[[2026-11-20-dashboard-reference-alignment]]", "[[2026-11-20-frontend-first-slice]]"]
---

# Live traffic and WAN capacity

Timestamp: 2026-11-20T20:42:24+08:00 (Asia/Shanghai).
Agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
Previous commit: `61b1dc9`.

## Request and decisions

The owner requested LibreQoS-style live updates and a vertical rainbow chart.
Red must mark the configured combined WAN capacity.
Keep separate download and upload sums. Never add the two directions together.
Use true WAN base rates from the validated catalog. Exclude shaping factors and subscriber plan totals.
Configured capacity does not prove live link availability, packet routing, or applied shaping.

The LibreQoS upstream reference is `15be4ca69d07691813260ed620498fb1f6c9aae6`.
Its ticker runs once per second. Its live graph uses a timestamped ring buffer.
LQoSync uses server-sent events for this one-way stream through the existing Rust gateway.
The browser uses Rust and Leptos. Application code does not use handwritten JavaScript.

## Changes

- The core publishes a safe WAN capacity summary in `status.json`.
- The native sampler runs once per second. All three histories keep one hour, with a 3601-point ceiling.
- The public worker cache remains bounded at 2 MiB and uses atomic replacement on tmpfs where available.
- The broker refreshes traffic each second and other overview fields every five seconds.
- The traffic decoder skips unrelated JSON trees. Browser traffic remains bounded at 300 points.
- The gateway permits 16 live streams. It checks the session and account before each event.
- The browser closes hidden or disposed streams. Fallback reads preserve monitoring and account checks.
- SVG gradients use shared rate coordinates and separate directional red limits.
- Rate gaps break the geometry. Traffic above capacity remains visible.
- Theme colors, line patterns, numeric values, and the semantic sample table support chart access.
- The Nginx example disables response buffering. The Traefik route stays the same.

## Checks and evidence

Core, gateway, and chart tests: 1590 passed, 0 failed, 12 existing ignored tests.
Core, web services, browser fixture, and Wasm release builds passed.
Gateway Clippy passed with warnings denied. Browser smoke and pulse checks passed.
Browser checks cover themes, 320/768px layouts, stale recovery, stream lifecycle, thresholds, and traffic above capacity.
The source history test keeps a full hour at one-second intervals.
The cache stress tests preserve basic worker rows and all three histories.

Gateway and Wasm Clippy passed with warnings denied. Rust 1.89 checks passed for both web crates.
The full-hour source fixture produced 300 browser points and 19,511 bytes in approximately 27 ms in debug mode.
See [design and source links](../../docs/frontend/LIVE_TRAFFIC.md).
See [browser evidence](../../docs/frontend/evidence/live-traffic/README.md).
See [deployment](../../docs/frontend/DEPLOYMENT.md).

## Remaining work

Install the prepared core and web builds with root privileges.
This session does not replace installed binaries or restart production services.
The web-only installer cannot install the updated core.
An older core shows no WAN capacity summary. The chart must then show capacity as unavailable.
Per-WAN traffic, link health, longer browser windows, and broader F06 performance gates remain open.
The TUI screens and controls stay unchanged.
Preserve unrelated November 16 notes and the existing dataplane report.
