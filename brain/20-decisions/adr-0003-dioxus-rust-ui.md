---
type: decision
created: 2026-10-30
tags: [lqosync-v3, frontend, dioxus, rust-ui]
status: accepted
related: ["[[10-sessions/2026-10-30-frontend-skills-plan]]"]
---

# ADR 0003: Select Dioxus and Rust/UI

## Context

The current v3 console uses Maud templates, custom CSS, and vanilla JavaScript on Axum.
The operator requested Dioxus and Rust/UI as the frontend stack.
The operator limited this task to skills, Tandem, and a migration plan.

## Decision

Use Dioxus for the future browser interface and selected source components from Rust/UI.
Keep Axum and existing domain operations on the server.
Plan a separate browser crate at `v3/ui/`, with a same-origin preview under `/ui/`.
Keep Maud available during migration and verify page parity before the default switch.

Use `ui-ux` for interaction and accessibility guidance.
Use `frontend-design` for visual direction within the compact NOC console context.
The selected framework takes precedence over skill database presets.

## Alternatives

Keep Maud permanently: this does not meet the operator's selected frontend direction.
Replace the backend with a new fullstack application: this adds unnecessary migration scope.
Install Rust/UI as one component crate: the selected project distributes component source through a registry.

## Consequences

The target adds a WebAssembly and Tailwind build to the existing Rust build.
Exact dependency versions, registry revision, licenses, and Rust compatibility need verification before implementation.
Keep credential material out of browser contracts. Preserve server checks and publication behavior.
This decision does not authorize deployment, service restart, router writes, or production shaping.

See the [design](../../docs/superpowers/specs/2026-10-30-dioxus-rust-ui-design.md)
and [plan](../../docs/superpowers/plans/2026-10-30-dioxus-rust-ui.md).
