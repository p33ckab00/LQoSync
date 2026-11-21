---
type: session
created: 2026-11-21
tags: [frontend, routers, modal]
status: complete
related: ["[[2026-11-21-routers-screen-planning]]"]
---

# Router toolbar and registration modal

The owner requested icon buttons for Add Router and Sync, plus a modal for registration.
The toolbar uses a plus and circular arrows. Each button has a label and a 44px target.
Sync feedback appears below the header. It does not move the buttons.

The native dialog preserves the Connection, Sources, and Review steps.
The `/routers/new` route opens the same modal above the inventory.
The inventory retains its filters. Close returns focus to the Add Router button.
New routers remain disabled after the save. Backend behavior does not change.

Vercel frontend-design and Google Modern Web Guidance supplied design and dialog guidance.
Tandem, UI/UX, Ponytail, Brain, and verification guidance also applied.
The graph query returned older TUI concepts. Current web source supplied the implementation evidence.
The optional UI/UX inspector scripts are absent. Playwright supplied screenshots and interaction checks.

Checks passed: 26 UI tests, release Wasm, Wasm Clippy, `cargo fmt`, whitespace, and three router browser suites.
Browser checks cover modal focus, keyboard access, close controls, field validation, save, filters, Sync, and narrow layouts.
All browser data is synthetic. No production configuration, service, or router changed.

Evidence: `docs/frontend/evidence/router-toolbar/README.md`.
The web distribution is ready for installation through the normal frontend workflow.

## Connection actions

The owner also requested inline icon buttons for Test Connection and Edit Connection.
The Connection tab now places the plug and pencil icons in one row.
Hover and keyboard focus show a label. Escape dismisses the label without a focus change.
Disabled routers retain the test explanation and a hover label.
The test result and edit/save behavior remain available.

The three router browser suites passed again with checks for these interactions and 320px layout.
The release build, 26 UI tests, Wasm Clippy, `cargo fmt`, and whitespace checks passed.
The web distribution is prepared. Production services did not change.
