---
type: session
created: 2026-11-21
tags: [frontend, routers, layout]
status: verified
related: ["[[2026-11-21-router-toolbar-modal]]"]
---

# Router reference layout

Updated: 2026-11-21T16:24:27+08:00 (Asia/Manila).

The owner requested the layout from `docs/frontend/images/02-routers.png`.
The implemented inventory now opens cached details beside the selected row.
Inspect uses a stable router ID. Close and Escape restore focus.
Filtering closes a selection that no longer matches.
The row name and summary actions keep the existing detail routes.
Add, Sync, registration, and Connection actions retain their behavior.

Changed files: `crates/web-ui/src/routers.rs`, `crates/web-ui/style.css`, and `tools/frontend/routers-layout-check.cjs`.
Plan: `docs/frontend/plans/2026-11-21-router-reference-layout.md`.
Evidence: `docs/frontend/evidence/router-reference/README.md` and five screenshots.
Vercel frontend-design and Google responsive-table guidance informed the composition.
The table retains all columns in a named local scroll region.
Details stack below the inventory under 1200px.
The UI does not invent collection duration, interval, or message fields.

Checks: 32 UI tests, release Wasm, Wasm Clippy, formatting, whitespace, and five router browser suites passed.
Chromium checks cover keyboard focus, themes, routes, states, and 320–1536px layouts.
No production service or router changed. Production installation remains pending.
Next action: review the screenshots and use the standard web installation workflow when requested.
