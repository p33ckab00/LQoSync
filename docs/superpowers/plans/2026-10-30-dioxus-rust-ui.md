# Dioxus and Rust/UI implementation plan

> **For agentic workers:** Use `superpowers:executing-plans` for inline execution. Use subagents only with separate delegation authorization.

**Goal:** Replace the v3 console frontend with Dioxus and Rust/UI and preserve server behavior.

**Architecture:** Add a browser-only crate under `v3/ui/` and serve its preview under `/ui/` from Axum.
Use explicit JSON contracts and existing server operations. Keep Maud during the migration.

**Tech stack:** Rust, Axum, Dioxus web, WebAssembly, selected Rust/UI source components, and compiled Tailwind CSS v4.

**Spec:** [Dioxus and Rust/UI design](../specs/2026-10-30-dioxus-rust-ui-design.md).

Status: Plan only. No implementation task below has run.

## Global constraints

- The current task authorizes skills, Tandem, and a plan only.
- Keep Axum and domain operations on the server. No backend crates enter the WebAssembly dependency tree.
- Pin compatible versions before implementation. Verify the current Rust 1.88 floor.
- Preserve CSRF, revision checks, loopback access, ownership, protected entries, and publication gates.
- Preserve candidate, staged, and applied labels. Preserve inventory freshness labels.
- Use explicit Save actions. Keep drafts after rejected saves.
- Keep current routes available during preview. Use synthetic data on `127.0.0.1:19304`.
- Do not change live settings, router configuration, production output, or services.
- Read current files again before execution. Other authorized work can change the integration baseline.

## Task 1: Prove the build and one component

**Create:** `v3/ui/{Cargo.toml,Cargo.lock,Dioxus.toml,README.md,UPSTREAM.md}`,
`v3/ui/src/main.rs`, `v3/ui/src/components/ui/button.rs`, and `v3/ui/assets/tailwind.css`.
Add only the Tailwind source and lock files needed by the selected build method.

**Consumes:** Official Dioxus documentation and a pinned Rust/UI Dioxus source revision.
**Produces:** A repeatable browser build with one working Button and local styles.

- [ ] Inspect installed Rust, `dx`, browser test tools, and Tailwind tooling. Record exact versions and missing tools.
- [ ] Resolve compatible Dioxus crate/CLI versions and the registry commit. Check licenses and the Rust version requirement.
- [ ] If Rust 1.88 is insufficient, record the required floor and its effect before changing a toolchain.
- [ ] Create a separate crate with Dioxus web features. Copy the pinned Button with its required helpers only.
- [ ] Build with `dx build --platform web --release` from `v3/ui/` using the pinned CLI.
- [ ] Open the built page under production-style CSP. Confirm the Button receives keyboard focus and responds once.
- [ ] Document exact build commands, asset output paths, source attribution, and tool requirements in the new UI README.

Acceptance: Rust/UI source compiles with the pinned Dioxus release. The built page loads without CDN access or CSP errors.
This task resolves the compatibility choices before later tasks depend on them.

## Task 2: Add an isolated read-only preview

**Create:** `v3/src/ui_api.rs`, `v3/tests/ui_api.rs`, `v3/ui/src/{api.rs,app.rs}`.
**Modify:** `v3/src/{lib.rs,web.rs}`, `v3/ui/src/main.rs`.

**Consumes:** `web::Shared` and the build output from Task 1.
**Produces:** `/ui/` plus a safe, read-only overview response at `/api/ui/overview`.

- [ ] Add an integration test: `/api/ui/overview` rejects an untrusted Host and never mutates the generation pointer.
- [ ] Add tests that response fields contain no credentials and include `Cache-Control: no-store`.
- [ ] Run `cargo test --locked --test ui_api` from `v3/` and confirm the new route cases fail before implementation.
- [ ] Implement a narrow DTO built from the existing overview state. Include status, generation ID, and clearly named totals.
- [ ] Serve preview assets under `/ui/`. Restrict fallback to browser paths and keep `/api/` errors as API responses.
- [ ] Add only the CSP permissions proved necessary by the release WebAssembly build.
- [ ] Fetch the overview with same-origin credentials. Render loading, empty, error, and success states.
- [ ] Repeat the API tests and load `/ui/` against isolated synthetic data. Check reload and direct navigation.

Contract rule: DTOs expose selected display fields. They must not serialize `App`, `Settings`, or raw inventory wholesale.
Keep the browser DTO definition separate from the backend crate, or use a small contract crate only if duplication becomes material.

## Task 3: Build the shared console shell

**Create:** `v3/ui/src/components/{shell.rs,status.rs}` and page modules under `v3/ui/src/pages/`.
**Modify:** `v3/ui/src/app.rs`, styles, and `v3/tests/browser/configuration.cjs` as needed.

**Consumes:** The safe overview response and preview routing.
**Produces:** Grouped navigation, route handling, density preference, status labels, and reusable controls.

- [ ] Read `ui-ux` for interaction rules and `frontend-design` for visual decisions. Keep the NOC product context.
- [ ] Build navigation for the 12 existing routes listed in `v3/src/noc_web.rs`.
- [ ] Add only the Rust/UI components needed by implemented pages. Record each source and license.
- [ ] Preserve direct links under the preview prefix, browser back, density preference, and mobile navigation.
- [ ] Add browser checks for keyboard navigation, focus return, visible errors, and explicit state labels.
- [ ] Check 1440px, 768px, 390px, and the existing five-width grid/font checks.

Acceptance: The shell retains readable text and current navigation behavior. Unimplemented pages link to the existing Maud route explicitly.

## Task 4: Migrate read-only operational views

**Create:** `v3/ui/src/pages/{overview.rs,subscribers.rs,core_review.rs,validation.rs}`.
**Modify:** `v3/src/ui_api.rs`, `v3/tests/ui_api.rs`, and the corresponding existing backend view modules only where needed.

**Consumes:** Existing normalization, validation, assignment, and core analysis results.
**Produces:** Typed read-only endpoints and Dioxus views for Overview, Subscribers, Core review, and Validation.

- [ ] Create synthetic API cases for missing data, saved inventory, stale evidence, incomplete evidence, and a valid staged generation.
- [ ] Confirm the new cases fail before implementing each endpoint.
- [ ] Build explicit DTOs from existing analysis results. Keep review actions read-only.
- [ ] Add tables, filters, empty states, and error states using the selected components.
- [ ] Compare visible totals and findings against the equivalent Maud fixture pages.
- [ ] Check large synthetic tables. Add pagination only if measured render cost requires it.

Acceptance: The UI shows the same findings and provenance. It never labels staged output as applied or snapshot totals as active sessions.

## Task 5: Migrate configuration through shared operations

**Create:** `v3/ui/src/pages/{routers.rs,wans.rs,sources.rs,assignments.rs,speed_rules.rs,core_lists.rs,settings.rs}`.
Create `v3/src/ui_commands.rs` only for operations that must serve both form and JSON handlers.
**Modify:** `v3/src/{ui_api.rs,web.rs,wan_web.rs,sources_web.rs,assignment_web.rs,speed_web.rs,dhcp_speed_web.rs,core_web.rs}`,
`v3/tests/ui_api.rs`, and browser configuration checks.

**Consumes:** Existing form operation behavior and revision rules.
**Produces:** JSON reads and explicit save actions for the seven configuration views and router-specific speed view.

Proposed API response contract, to implement in this task:

```text
200: successful save with refreshed page data
403: invalid token or rejected origin
409: revision conflict; keep the local draft
422: validation errors; keep the local draft
5xx or network failure: unknown result; refresh evidence before another write
```

- [ ] For each operation, add tests for a valid save, invalid token, invalid data, and stale revision where the operation supports revisions.
- [ ] Add a test that one accepted mutation triggers the existing reconciliation/publication sequence at most once.
- [ ] Extract the smallest shared operation from the form handler. Preserve checks and persistence order.
- [ ] Route JSON handlers through the same operation. Keep form response behavior intact.
- [ ] Return only the CSRF token required by the same-origin UI session. Keep it out of logs and persistent browser storage.
- [ ] Connect dialogs to explicit saves. Disable duplicate submissions and keep drafts on failures.
- [ ] Test delayed responses, rejected tokens, expired revisions, retry after refresh, and dialog focus return.
- [ ] Compare saved synthetic settings and staged outputs with existing form-handler results.

Acceptance: Both frontends use the same authoritative behavior. No new production write endpoint or publication bypass exists.

## Task 6: Migrate the topology workspace

**Create:** `v3/ui/src/pages/topology.rs` and focused components for the tree, inspector, and node dialog.
**Modify:** `v3/src/{ui_api.rs,dynamic_web.rs}`, `v3/tests/ui_api.rs`, and `v3/tests/browser/dynamic-topology.cjs`.

**Consumes:** Existing dynamic edit operations, revision evidence, dependency counts, and destination policies.
**Produces:** The topology view and its existing creation, promotion, demotion, replacement, and removal actions.

- [ ] Extend synthetic API cases for stale revisions, cycles, missing parents, and unresolved replacement dependencies.
- [ ] Reuse Task 5's operation boundary for dynamic edits. Keep validation on the server.
- [ ] Render discovered inventory, the configured hierarchy, and the selected-node inspector.
- [ ] Keep routing WAN identity separate from exported ancestry. Display impacts before destructive topology changes.
- [ ] Adapt the existing topology browser script to the Dioxus preview without weakening its assertions.
- [ ] Verify all topology flows, rejected edits, retained drafts, and responsive inspector behavior.

Acceptance: Existing topology constraints and output semantics remain intact.

## Task 7: Verify parity and prepare the default switch

**Modify:** `v3/tests/browser/{configuration.cjs,dynamic-topology.cjs,README.md}`,
`v3/{README.md,OPERATIONS.md}`, and frontend routing/build configuration.

**Consumes:** All migrated preview routes and the existing synthetic fixtures.
**Produces:** A release build, parity evidence, a documented fallback, and a reviewable default-switch change.

- [ ] Run from `v3/`: `cargo fmt --check`, `cargo test --locked`, and `cargo clippy --locked --all-targets -- -D warnings`.
- [ ] Run equivalent format, locked tests, and strict Clippy checks for the separate browser crate on its supported target.
- [ ] Build the production web bundle with the pinned Task 1 commands.
- [ ] Run both browser suites against isolated synthetic data. Cover all 12 navigation routes and router-specific speeds.
- [ ] Test direct links, reload, history, focus, modal errors, density, reduced motion, and 200 percent zoom.
- [ ] Check production CSP, safe asset paths, missing assets, offline runtime assets, token rejection, and stale revisions.
- [ ] Verify the Maud fallback with the current data format. Rollback must select the old UI without restoring settings.
- [ ] Prepare the default-switch diff and operations steps. Keep live deployment and restart separate from implementation checks.

Stop condition: All pages meet parity and all required checks pass. Record exact versions, test results, and remaining limits.
Keep Maud removal as a separate accepted cleanup task after the first rollout succeeds.
