# Dioxus and Rust/UI console migration

Date: 2026-10-30 (Asia/Shanghai).
Status: Stack selected. Implementation pending.

## Scope

The operator selected Dioxus and Rust/UI for the v3 frontend.
The current task installs design skills, updates Tandem, and prepares this migration plan.
It does not implement or deploy the migration.

The current application uses Axum, Maud, custom CSS, and vanilla JavaScript.
The target uses Dioxus for the browser interface and selected Rust/UI components.
Keep Axum and the existing Rust domain modules on the server.
Target web browsers only. Desktop and mobile application bundles are outside this migration.

## Architecture

Create a separate browser crate at `v3/ui/`.
Use Dioxus web rendering with WebAssembly. Keep backend dependencies out of the browser crate.
The initial preview lives under `/ui/`. Existing Maud routes remain available during migration.

Serve built assets and JSON endpoints from the same loopback Axum origin.
Use `/api/ui/` for explicit browser data contracts.
Do not expose application state, credentials, or raw inventory through generic serialization.

Keep assignment, rate, topology, ownership, and publication decisions in existing server modules.
Extract shared application operations only where the JSON and form handlers need the same operation.
Preserve each operation's token check, revision check, validation, persistence, and publication sequence.
Do not create a second scheduler or a second publication call after an API mutation.

Read-only UI requests must not start collection or publication.

## Component and build policy

Rust/UI is a source registry, not a single component crate.
Its repository identifies Dioxus 0.7 and Tailwind CSS v4 as its stack.
The installation page still contains Leptos instructions. Prefer the selected Dioxus source and compile evidence.

Before implementation, pin a compatible Dioxus release, CLI version, Tailwind tool, and registry commit.
Verify the Rust 1.88 compatibility claim against those dependencies. Record any required toolchain change first.

Copy only required components into `v3/ui/src/components/ui/`.
Start with Button, Input, Label, Card, Table, Badge, Select, and Dialog as pages require them.
Check each component's dependencies, browser behavior, and license before adoption.
Keep source URLs, commit IDs, and license notices in `v3/ui/UPSTREAM.md`.

Compile Tailwind assets during the build. Serve all runtime assets locally.
Do not add a CDN, React, or Leptos to satisfy a design preset.

The current build needs no Node toolchain. The target adds a frontend asset build.
Select and document the exact Tailwind build method during the compatibility task.
Do not install global tools or change service units during this planning task.

## Interface requirements

Keep the compact dark NOC console, grouped navigation, dense tables, and explicit Save actions.
Keep the topology workspace and selected-node inspector.
Preserve direct links, browser back behavior, filters, density preference, and modal forms.
Failed saves keep the user's draft and show useful errors. Successful saves update visible state once.
Use labels and text for status. Color alone must not carry status information.

Keep candidate, staged, and applied output distinct.
Keep live, saved, stale, and incomplete inventory labels distinct.
Snapshot totals must not appear as online session counts.

Current primary text uses 14px. Current secondary text uses at least 12px.
Use at least 13px for new readable secondary text, as specified by the selected UI/UX guidance.

Keep existing three/two/one card columns above 1300px, above 900px, and on narrower screens.
Use visible focus, keyboard access, accessible dialogs, reduced motion, and WCAG 2.2 AA contrast targets.
Browser evidence must confirm these properties. A component name does not prove accessibility.

## Security and operation boundaries

Keep loopback access checks, same-origin requests, no-store responses, and explicit CSRF checks for mutations.
Keep revision conflicts visible. Do not silently retry a mutation after an unknown result.
Send no passwords, private credential paths, or raw router rows to browser DTOs.

Allow WebAssembly through the narrow CSP directive required by a verified production build.
Keep scripts and connections on the same origin. Do not add a general `unsafe-eval` exception.
Keep `frame-ancestors 'none'`, `base-uri 'none'`, and `form-action 'self'`.

Preserve protected manual RouterOS entries and existing ownership checks.
The frontend migration does not enable router writes, production shaping, or new scheduler operations.
Deployment and service restart remain separate tasks with explicit authorization.

## Rollout and acceptance

Migrate the preview in route groups. Keep equivalent Maud pages available until each group passes checks.
Use isolated synthetic data on `127.0.0.1:19304`. Do not use production settings for browser tests.
Cover all 12 navigation routes and the router-specific speed page.
Test validation errors, stale revisions, invalid tokens, draft retention, reload, and browser back navigation.

Test the existing topology creation, promotion, demotion, replacement, and removal flows.
Check desktop, tablet, narrow screens, and 200 percent zoom.
Confirm backend tests and fixture outputs remain equivalent after shared operation extraction.

Switch the default UI only after parity, build, security, and browser checks pass.
Keep a tested Maud fallback for the first rollout. Remove it in a separate change after acceptance.
Rollback changes the frontend selection. It must not restore old settings over current user data.

## Sources

- [Rust/UI repository](https://github.com/rust-ui/ui): component model, Dioxus 0.7, and Tailwind CSS v4.
- [Rust/UI installation page](https://rust-ui.com/docs/components/installation): verify framework-specific instructions against source.
- [Dioxus getting started](https://dioxuslabs.com/learn/0.7/getting_started/): official build tooling.
- [Current v3 application](../../../v3/README.md).
- [Synthetic browser checks](../../../v3/tests/browser/README.md).
