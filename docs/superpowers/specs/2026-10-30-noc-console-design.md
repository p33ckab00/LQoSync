# Compact NOC console

The user selected the supplied HTML as the visual design. Follow its layout, colors, spacing, navigation, and controls.
The reference title is `LQoSync 3 · Full NOC Console · UI Prototype`.

## Approach

Use the supplied CSS with the current Axum/Maud pages. Keep the Rust forms and their validation.
Do not replace the application with the reference's synthetic data or preview actions.
A static copy loses real controls. A separate frontend framework adds an unnecessary build system.

## Scope


- Use the 206px sidebar, grouped navigation, sticky top bar, teal accents, compact tables, and responsive layout.

- Provide working navigation and density controls. Use a same-origin script with no inline event handlers.

- Show actual local state on Overview. Distinguish snapshot counts from online sessions.

- Show router rows and a selected-router inspector with the current configuration and inventory controls.

- Show discovered inventory, the configured hierarchy, and a selected-node inspector in Topology.

- Keep all current routes, form fields, tokens, ownership checks, and publication gates.

- Use the reference styles for the other console pages. Keep their actual forms and diagnostic results.

- Do not invent utilization, validation passes, online counts, a core gateway, or applied state.

## Limits

This change implements the supplied visual design. It does not change assignment policy or add router writes.

The earlier requested reconnect policy remains future work: count online clients and reassess automatic assignments on a new session.
Topology and core membership must eventually share that assignment. Manual pins keep priority.

A failed inventory read must not count as a disconnect. Protected manual core entries still require migration review.
The current topology schema and source-selection rules remain in force during this visual change.

## Verification

Check every console route, navigation state, script policy, escaped names, and the topology inspector.
Run Rust formatting, tests, and strict Clippy. Check desktop and mobile layouts in an isolated browser session.
Use temporary application data. Do not read private credentials or change the running service.
