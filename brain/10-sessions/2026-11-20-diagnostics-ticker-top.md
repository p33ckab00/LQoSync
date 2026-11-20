---
type: session
created: 2026-11-20
tags: [frontend, diagnostics, ticker, activity]
status: verified
related: ["[[2026-11-20-diagnostics-ticker]]"]
---

# Top ticker and conditional activity

Timestamp: 2026-11-20T21:16:53+08:00 (Asia/Shanghai).
Agent: Codex `/root`. Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.
Previous commit: `9c2e06b`.

The owner corrected the ticker placement and content.
Show it above the Dashboard heading, below the shared header.
Always show Service health, Runtime mode, Output target, Publication phase, and Last cycle.
Keep the raw publication phase, including `IMPORT_CONFIRMED`.

Component/status activity appears only on an observed change.
The first source snapshot establishes a baseline. Identical and traffic-only updates add no activity.
Keep at most eight entries for 120 seconds. Expiry leaves the five permanent fields in place.
A small local timer expires activity without another source update. Dashboard cleanup disposes the timer.
Static details remain below the main panels. Pause, reduced motion, and theme support remain available.

This is a browser observation list, not a durable backend event or job-progress feed.
Detailed orchestration events require a future authenticated backend event projection.
No core, gateway, TUI, or network polling change was needed.

Checks: five native chart/activity tests, Wasm release, Clippy with warnings denied, and Rust 1.89 passed.
Browser checks passed for top placement, five fields, raw publication phase, new activity, duplicates, expiry, and stable motion.
Keyboard/hover pause, stale evidence, reduced motion, themes, and 768/320px checks passed.
The existing sign-in and Dashboard smoke check passed. The core suite was not repeated.

See [design and backend gap](../../docs/frontend/DIAGNOSTICS_TICKER.md).
See [current screenshots](../../docs/frontend/evidence/diagnostics-ticker-top/README.md).
Vercel frontend-design and Google modern-web-guidance `motion` supplied the frontend guidance.

The web build is prepared. Run `sudo tools/frontend/install.sh` to install it.
This session did not replace installed assets or restart production services.
Preserve the unrelated November 16 notes and dataplane report.
