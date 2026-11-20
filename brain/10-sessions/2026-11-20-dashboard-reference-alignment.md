---
type: session
created: 2026-11-20
tags: [frontend, dashboard, design, rust]
status: built-and-verified
related: ["[[2026-11-20-frontend-first-slice]]", "[[2026-11-20-frontend-documentation-plan]]"]
---

# Dashboard reference alignment

Checkpoint: 2026-11-20T20:03:19+08:00 (Asia/Shanghai). Agent: Codex `/root`.
Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`. Previous HEAD: `06f0536`.

## Owner direction

Build the Dashboard from its approved picture and frontend documentation.
Always use Vercel and Google frontend skills for this project.
The written corrections take priority over generated sample values.

## Result

The Dashboard now has five icon cards, gradient traffic with axes and sample times, a WAN distribution panel, and an upstream table.
The dark palette follows the documented tokens. Theme controls remain in the main header.
The sidebar identifies unavailable screens. It has no account administration route.
Service diagnostics follow the main panels.

A new safe DTO field counts bound DHCP observations for the IPoE card.
The gateway filters invalid traffic timestamps, orders samples, and checks the latest sample age.
The UI preserves open panels during polling and marks retained data stale after a request failure.
The broker still reads only four public cache names. Core service and TUI source files did not change.

See [alignment contract](../../docs/frontend/DASHBOARD_ALIGNMENT.md) and [evidence](../../docs/frontend/evidence/dashboard-alignment/README.md).
The evidence has Light, Dark, mobile, and expanded-detail pictures with hashes.

## Checks and open work

Eleven web tests passed. Gateway and Wasm UI Clippy passed with warnings denied.
Native and Wasm release builds passed. Rust 1.89 gateway and UI checks passed.
Browser checks covered reference sections, IPoE counts, themes, sessions, refresh, failure recovery, and 320px width.

WAN attribution, capacity, and health need a safe published adapter. Longer history needs retention support.
The UI labels these gaps unavailable. F06 remains partial.

The source and build artifacts are ready. Root installation of this update is pending.
The knowledge graph was not regenerated for this checkpoint.
