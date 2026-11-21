# Router toolbar and Add Router modal

Date: 2026-11-21 (Asia/Shanghai).

The inventory header places two 44px icon buttons beside the title.
The plus button opens Add Router. The circular arrows start Sync.
Both buttons have accessible names and title hints.
Sync feedback appears below the header and does not move the buttons.

Add Router uses a native modal dialog above the inventory.
The `/routers/new` address opens the same modal directly.
The Connection, Sources, and Review steps keep their validation and save behavior.
New routers remain disabled after the save.
Escape, Cancel, and Close return to the inventory. The Add Router button receives focus.

The inventory keeps its filters while the modal opens and closes.
The modal contains scroll gestures without a change to root overflow.
Scroll containment on non-scrollable containers remains a browser enhancement.

Guidance: [Vercel frontend-design](https://github.com/vercel-labs/open-agents/blob/main/.agents/skills/frontend-design/SKILL.md)
and [Google Modern Web Guidance](https://github.com/GoogleChrome/modern-web-guidance).
Google guide IDs: `dialog` and `prevent-page-scroll-under-modal-dialog`.
The project-specific UI/UX inspector scripts are absent. Playwright supplies browser checks and screenshots.

## Validation


- The release Wasm build passed. The distribution includes the CSP-compatible bootstrap.

- All 26 native UI tests passed. Wasm Clippy, `cargo fmt`, and whitespace checks passed.

- Three router browser suites passed: inventory/navigation, edits, and operations.

- Modal checks cover initial focus, keyboard access, Escape, Cancel, Close, filter retention, and direct navigation.

- Width checks cover 320, 390, 768, and 1440px. Screenshots cover light and dark themes.

- The toolbar retains its position after Sync feedback appears.

See [the desktop toolbar](toolbar-dark-1440.png), [the mobile toolbar](toolbar-dark-390.png),
and [the registration modal](add-modal-1440.png).
All browser evidence uses synthetic router data and temporary fixture accounts.
No production router, configuration, or service changes form part of this update.

## Connection actions

The Connection tab places Test Connection and Edit Connection in one row.
The buttons use plug and pencil icons with 44px targets.
Labels appear on hover and keyboard focus. Escape dismisses the labels without a focus change.
The labels remain visible when the pointer moves onto them.
The disabled Test button retains its label on hover and its explanation below the row.

The buttons keep their accessible names. The test result and edit/save flow keep their prior behavior.
Google guide `interest-triggered-tooltips` supplied the interaction checks.
The Rust/CSS implementation uses hover, focus, and Escape handlers without new browser API dependencies.

All three router browser suites passed after this change.
The operations suite checks icon alignment, empty button text, hover/focus labels, Escape, disabled hints, and 320px layout.
The release build, 26 UI tests, Wasm Clippy, `cargo fmt`, and whitespace checks passed.

See [the inline icons and hover label](connection-icons.png) and [the narrow layout](connection-icons-320.png).
