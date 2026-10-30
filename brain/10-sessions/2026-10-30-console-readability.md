---
type: session
created: 2026-10-30
tags: [lqosync-v3, console, layout]
status: complete
---

# Console readability and card layout

Updated: 2026-10-30T14:02:38+08:00 (Asia/Shanghai).
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`.

The operator requested larger fonts and two or three columns for small cards.
Primary text, controls, and table values use 14px fonts. Secondary labels use at least 12px.
Table rows and controls have more vertical space. Summary cards use up to three columns.
Settings, WAN mappings, core mappings, and source policy cards share a responsive grid.
The grid uses three columns above 1300px, two above 900px, and one on narrower screens.
Tables and the topology workspace retain their existing layout. Native configuration dialogs retain their behavior.

Initial synthetic browser checks passed on all 12 routes at three widths.
Font and card grid checks passed at five widths. The WAN page was inspected visually at 1440px.
Final checks passed: cargo fmt --check, cargo test --locked (196 passed, zero failures, 12 ignored), and cargo clippy --locked --all-targets -- -D warnings.
The locked binary build passed. Both browser scripts passed against the final packaged binary and isolated synthetic data.
Browser checks covered modal layering, keyboard focus, validation, retries, responsive grids, and topology edits.
Diff review found no behavior or performance changes outside layout and typography. No dependency was added.
The live console remains on the previous build. No service restart, router write, or production apply is part of this change.
