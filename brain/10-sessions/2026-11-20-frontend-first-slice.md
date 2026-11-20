---
type: session
created: 2026-11-20
tags: [frontend, implementation, rust, authentication]
status: deployment-pending
related: ["[[2026-11-20-frontend-documentation-plan]]", "[[2026-11-20-frontend-blueprint-research]]"]
---

# Sign-in and Dashboard implementation

Checkpoint: 2026-11-20T19:00:50+08:00 (Asia/Shanghai). Agent: Codex `/root`.
Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Previous HEAD: `df5738d`.

## Request and result

Start with sign-in and Dashboard. Use port `9581` and `lqosync.fsehub.qzz.io`.
Support the current Traefik deployment and provide an Nginx example.

Added Rust contracts, a Leptos CSR application, a gateway, a private broker, and a root-only account CLI.
The web application supports Light, Dark, and System themes.
The Dashboard reads allowlisted backend caches. It starts no collector.
Account changes remain CLI-only. The web has no signup or account administration route.
The core service and TUI source files did not change.

See [implementation](../../docs/frontend/FIRST_SLICE.md), [deployment](../../docs/frontend/DEPLOYMENT.md),
and [evidence](../../docs/frontend/evidence/FIRST_SLICE.md) for files, commands, screenshots, and limits.

## Checks

Core tests: 1,571 passed, zero failed, 12 ignored. Web tests: nine passed.
Gateway Clippy, formatting, release builds, and Rust 1.89 checks passed.
Chromium checked themes, forms, sessions, logout, and the 320px layout.
The safe projection also read the current public caches. It returned 31 subscribers and one router at that check.
The evidence document distinguishes synthetic browser data from this backend observation.

## Open work

Live installation requires root. Passwordless sudo is unavailable in this coding session.
The operator must create the first account through the hidden CLI prompt.
Merge only the new Traefik entries. Verify DNS, cloudflared ingress, and TLS.
The complete F00–F06 task cards remain partial. Other screens and configuration actions remain planned.
The knowledge graph was not regenerated for this implementation checkpoint.
