# Topology Manager Web Implementation Plan

> For agentic workers: use `superpowers:executing-plans` to implement one task at a time. Do not start parallel writers without explicit coordination.

**Goal:** Deliver the current Topology Manager workflow through a reliable, typed Rust web interface.

**Architecture:** The current service owns policy, collection, evaluation, and publication. A bounded core projection feeds the authenticated gateway through the private broker. Durable core receipts protect web mutations.

**Tech stack:** Rust, Leptos 0.8.23, WebAssembly, Axum, current broker IPC, core registry, SQLite web state, and semantic CSS.

**Spec:** [Feature and layout design](TOPOLOGY_DESIGN.md), [connections and integrity](TOPOLOGY_CONNECTIONS.md), and [mockup guide](evidence/topology-manager/README.md).

Source checkpoint: `76563ee`, 2026-11-21, Asia/Shanghai.
This is an implementation plan. Its unchecked tasks are not completed application features.
The static mockup files are documentation assets, not production frontend modules.

## Global constraints


- Preserve the current Leptos stack, theme selector, brand, router controls, and independent TUI.

- Keep one Runner and one registry/publication owner. Do not write registry or shaping files from web code.

- Keep GET, route entry, polling, and theme changes free of router I/O and mutations.

- Make refresh and discovery explicit POST jobs. Respect disabled-router I/O barriers.

- Preserve source identity, instance identity, exact numeric rules, revision checks, and static ownership.

- Implement TQ-01 through TQ-15 from the design. Do not replace evidence with a green generic status.

- Use STE for labels, procedures, errors, notes, and review text. Keep technical identifiers exact.

- Follow the requested Vercel and Google guidance. Retrieve current browser guides again during implementation.

- Do not enable schedules, activation, enrollment, router routing writes, or unsupported policy editors through placeholder handlers.

- Keep unrelated work out of each commit. Never stage with `git add .` in the shared worktree.

- Before each commit/push, follow the latest owner-approved work split. Check ownership, Git activity, and explicit staged paths.

## Dependency order and release cuts

```text
T00 -> T01 -> T02 -> T03        Read-only release cut
T00 -> T04 -> T05              Mutation reliability gate
T02 + T05 -> T06 -> T07        Builder and discovery
T03 + T05 -> T08 -> T09        Assignments and policies
T04 + T06 + T08 + T09 -> T10 -> T11
```

Each task must pass its tests and review before its dependents start.
A read-only release can stop after T03 with writes explicitly unavailable.
No task may skip T04/T05 through a proxy for unrestricted core operations.
The final release requires T11. Screenshot checks alone do not permit production writes.

## File ownership map

The file map proposes the paths marked New. They do not exist at this checkpoint.
Keep shared-file edits small and coordinate them with any other worker first.

| Area | Proposed files and responsibility |
|---|---|
| Core view | New `src/topology_manager/web_view.rs`: coherent safe snapshot, indexes, paging, evidence correlation |
| Core dispatch | New `src/api/topology_web.rs` and edit `src/api/dispatch.rs`, `src/runner.rs`, module declarations |
| Core receipts | New `src/topology_manager/web_receipts.rs`: durable request-key result and recovery contract |
| DTOs | New `crates/web-contract/src/topology.rs`: safe typed views, allowed commands, errors, operation results |
| Broker/HTTP | New `crates/web-gateway/src/topology.rs` and `topology_jobs.rs` and extend `broker.rs`, `http.rs`, `sessions.rs` only as needed |
| Browser workspace | New `crates/web-ui/src/topology.rs`, `topology_model.rs`, `topology_builder.rs`, `topology_assignments.rs`, `topology_policies.rs`, `topology_changes.rs`, `topology_forms.rs` |
| Route and shell | Edit `route_model.rs`, `route.rs`, `app.rs`, `main.rs`, and `style.css` under `crates/web-ui` |
| Proof | New core integration tests, gateway topology tests, and `tools/frontend/topology-check.cjs` |
| Release record | New implementation evidence beside this design's static mockups and update deployment instructions after gates pass |

The file map sets responsibilities. Do not create empty modules or abstractions before a delivery task needs them.

## T00 — Coherent bounded core view

**Files:** New core view, contract DTOs, core dispatch adapter, and `tests/topology_web_view.rs`.
**Consumes:** Current registry, cached preview, writer state, write receipts, and per-identity native evidence.
**Produces:** Proposed `topology.web.snapshot` with the envelope and bounded queries in the connection map.


- [ ] Write tests that fail for no registry, no preview, stale revision, conflict preview, unknown writer, and uncorrelated receipt.

- [ ] Write page tests with 50,000 synthetic subscriber rows and a deep hierarchy.

- [ ] Implement one Runner-owned read view with stable snapshot IDs and explicit nullable evidence.

- [ ] Filter/sort before page selection. Preserve ancestry, exact totals where available, and conflict continuation.

- [ ] Reject old cursors after snapshot expiry. Enforce bounds before JSON serialization.

- [ ] Prove that read queries cause no RouterOS request, policy commit, reload, or writer change.

Run: `cargo test --locked --test topology_web_view` after the test target exists.
Stop condition: TQ-01, TQ-02, TQ-03, and TQ-13 pass for the core view.

## T01 — Safe broker and HTTP read adapter

**Files:** New gateway topology module and `crates/web-gateway/tests/topology.rs` and extend the fixed broker enum and HTTP router.
**Consumes:** T00 snapshot schema and current session/account checks.
**Produces:** Proposed topology GET routes with stable DTO versions and bounded responses.


- [ ] Test absent/revoked sessions, wrong gateway UID, unknown DTO versions, mismatched response IDs, and oversized messages.

- [ ] Validate complete core response envelopes. Do not copy the partial validation of the current generic client.

- [ ] Add explicit broker variants and fixed operation routes. Exclude raw registry and arbitrary `op` requests.

- [ ] Sanitize errors and subject references. Keep credentials and filesystem paths out of browser data and audit.

- [ ] Test that gateway/broker restarts do not stop synchronization or create a second Runner.

Run: `cargo test --locked -p web-gateway --test topology` after the test target exists.
Stop condition: authorized cached reads work and all topology writes still return an explicit unavailable capability.

## T02 — Workspace shell and Builder reads

**Files:** New browser topology/model/Builder modules and route, shell, CSS, and browser fixture changes.
**Consumes:** T01 capabilities, source inventory, tree window, and selected-node DTOs.
**Produces:** `/topology?tab=builder`, four view links, evidence strip, three desktop panels, and compact panel selectors.


- [ ] Test route interpretation, unknown IDs, filter state, revision changes, and selection retention as pure model behavior.

- [ ] Build source and instance views with stable IDs and explicit freshness.

- [ ] Use accessible nested-list disclosure controls or implement and test the complete ARIA tree pattern.

- [ ] Preserve focus, scroll, expansion, and open drafts through polling and theme changes.

- [ ] Check 1440×900, 1024×768, 390×844, 320×640, 200 percent zoom, and both themes.

Run: native model tests, Wasm Clippy, release Wasm build, and the Builder portion of `topology-check.cjs`.
Stop condition: operators can inspect topology without any write or router-read request.

## T03 — Assignments and Policies reads

**Files:** Browser Assignments/Policies modules, pure models, gateway fixtures, and browser checks.
**Consumes:** T01 paged assignments, all subscriber identities, plans, defaults, factors, and fallback state.
**Produces:** Filtered Assignments and Policies views with honest read-only states.


- [ ] Test multi-identity subscribers, absent endpoints, offline pins, source-read rates, and inherited rules.

- [ ] Implement server-side filter/sort/page semantics and a selected-subscriber panel.

- [ ] Preserve table headers and keyboard-scroll access on narrow layouts.

- [ ] Link PPPoE/IPoE profile destinations to the same local-plan owner.

- [ ] Show unavailable schedules and unsupported policy editors with reasons.

Stop condition: TM-10 and all supported policy read fields work without mutation controls.
The read-only release cut ends here. Record browser and API evidence before release.

## T04 — Durable core mutation receipts

**Files:** Core receipt module, Runner/dispatch adapter, store recovery integration, and `tests/topology_web_receipts.rs`.
**Consumes:** Current edit/plan engines, revision store, and core recovery rules.
**Produces:** Proposed typed `topology.web.mutate` and `topology.web.operation` operations.

The mutate request uses a closed command enum, request key, payload hash, actor reference, and expected revision.
It does not accept arbitrary service operation names.
Old local TUI operations keep their current contracts. They can still invalidate web reviews through revision/proposal changes.


- [ ] Define a durable result envelope and a recoverable commit/receipt store protocol before handler implementation.

- [ ] Test the same key/payload twice, changed payload reuse, stale revision, and foreign actor access.

- [ ] Inject crashes before intent, before commit, after commit, before receipt response, and during recovery.

- [ ] Prove that a lost response resolves through receipt lookup without a second policy commit.

- [ ] Refuse activation if the registry/receipt storage layout cannot meet the recovery contract.

Run: `cargo test --locked --test topology_web_receipts` after the test target exists.
Stop condition: exactly one committed result or an explicit unresolved outcome survives every tested crash boundary.
Do not claim a shared SQLite/registry transaction unless the implementation actually supplies one.

## T05 — Web jobs, proposal lease, and session ownership

**Files:** Gateway jobs module, typed broker/HTTP mutations, web-state migration, and gateway topology tests.
**Consumes:** T04 receipt lookup, current sessions/accounts, and the core's single 120-second proposal slot.
**Produces:** Opaque principal-bound operations, idempotent HTTP commands, and one broker mutation lane.


- [ ] Test duplicate clicks, retries, changed payload keys, queue saturation, revoked sessions, and browser reload.

- [ ] Persist intent before dispatch. Return `202` and an operation URL without a browser request that waits for a full core cycle.

- [ ] Bind proposals to session/account version, actor, base revision, payload, and core expiry.

- [ ] Keep the lease until confirm/cancel/expiry. Treat a local TUI replacement as a conflict.

- [ ] Never cancel or replay a dispatched write merely because the browser disconnects.

- [ ] Test broker/gateway restart, disk-full failure, and unknown outcomes with a lost core response.

Stop condition: TQ-04 through TQ-07 and TQ-10 pass across the core/web boundary.
No mutation button can ship before this task passes.

## T06 — Builder edit forms and impact review

**Files:** Browser forms/Builder modules, core/gateway fixtures, and browser checks.
**Consumes:** T05 proposals and the current WAN/node/instance edit variants.
**Produces:** TM-04 through TM-08 through typed forms and a native review dialog.


- [ ] Test parent cycles, absent parents, invalid capacity modes, root inheritance, and protected static ownership.

- [ ] Add Create WAN, Create node, Add instance, Move, Rename/capacity, and Remove actions.

- [ ] Model Promote/Demote as explicit parent changes only where current rules accept them.

- [ ] Review affected counts, destinations, capacity differences, warnings, and omitted sample counts.

- [ ] Test proposal expiry, local replacement, concurrent revision change, focus return, draft retention, and safe cancel.

Stop condition: every successful edit has a definite Saved receipt and publication remains separately visible.

## T07 — Explicit refresh, discovery, and new sources

**Files:** Core/gateway job mappings, browser toolbar/candidate pickers, and RouterOS fake-connector tests.
**Consumes:** T05 job control and current preview/WAN/source discovery operations.
**Produces:** TM-02, TM-03, TM-09, with explicit router-I/O disclosure and per-router errors.


- [ ] Test enabled/disabled routers, partial source failure, timeout, stale candidates, and concurrent discovery.

- [ ] Run discovery only after explicit action. Deduplicate repeated in-flight requests with explicit state.

- [ ] Review source identity and parent before `source.add` and do not auto-attach all discovered candidates.

- [ ] Keep the last successful inventory visible with its own timestamp when a refresh fails.

- [ ] Test that source additions and fallback transitions preserve identity and exclude unauthorized observations.

Stop condition: entry to any topology view still causes zero RouterOS requests.

## T08 — Subscriber plans, pins, and scoped rebalance

**Files:** Assignments forms, gateway command routes, and core/browser tests.
**Consumes:** T05 receipts, complete identity DTOs, `subscriber.plan`, `subscriber.pin`, and `rebalance`.
**Produces:** TM-11 through TM-13 with explicit operation scope.


- [ ] Test multi-identity selection, moved/deleted identities, offline unpin, stale online evidence, and strict pins.

- [ ] Implement local-plan selection and clear, WAN pin and unpin, source rebalance, and explicit all-source rebalance.

- [ ] Show moved/cleared/assigned counts and make topology placement distinct from router routing.

- [ ] Add service-side aggregate preflight or a service-generated impact review for plan and subscriber-plan mutations.

- [ ] Test plan deletion in use, duplicate names, rate boundaries, exact revision checks, and lost replies.

Stop condition: no action addresses a subscriber only by current IP, row index, or visible label.

## T09 — Speed defaults, factors, and fallback

**Files:** Policies/forms modules, gateway routes, and core/browser tests.
**Consumes:** T05 proposal control, policy DTOs, `speed.defaults`, `group.factor`, and fallback edits.
**Produces:** TM-14 through TM-16 with the exact numeric contracts in the design.


- [ ] Test percentage/Mbps boundaries, decimal precision, overflow, minimum above maximum, and aggregate conflicts.

- [ ] Disclose that global speed-default edits reset profile overrides and can replace legacy rounded rules.

- [ ] Preserve instance factor overrides and show source factors separately.

- [ ] Test fallback set/clear while in use, fallback default rates, and rejected source changes.

- [ ] Keep unsupported per-profile overrides, backstops, formulas, and schedules unavailable.

Stop condition: service-generated preview values match the confirmed revision and browser display without float-based policy decisions.

## T10 — Changes, publication, and recovery display

**Files:** Changes module, core evidence projection, safe audit storage, and integration/browser tests.
**Consumes:** T00 snapshots, T04 receipts, T05 operation records, and write/import/native evidence.
**Produces:** TM-17 and TM-18, with explicit revision-to-write correlation.


- [ ] Test save-with-legacy-writer, publication hold, partial file failure, import failure, verification timeout, and late acceptance.

- [ ] Test superseded revisions, absent receipts, empty conflict previews, and unknown writer state.

- [ ] Link safe conflict subject IDs to the view that owns the affected record.

- [ ] Preserve the last good observation beside failed refresh evidence. Do not show conflict arrays as network deletion.

- [ ] Distinguish web actor history from unrecorded local CLI/TUI actors.

Stop condition: no successful HTTP save can appear as Applied without correlated native evidence.

## T11 — Release gates and operator handoff

**Files:** Browser checks, release evidence, deployment/rollback notes, and feature status documents.
**Consumes:** T00–T10 and the unchanged core publication path.
**Produces:** A reviewable release package and explicit limits for unsupported capabilities.


- [ ] Run the topology core tests, gateway integration tests, DTO tests, Wasm checks, `cargo fmt`, and relevant full-suite gates.

- [ ] Run browser checks in Chromium, Firefox, and WebKit for read and write flows.

- [ ] Check keyboard-only use, screen-reader semantics, 200 percent zoom, text scale, themes, reduced motion, and forced colors.

- [ ] Test 50,000 subscribers and 5,000 nodes. Publish payload, browser duration, and service-delay measurements with hardware details.

- [ ] Target bounded pages, no horizontal page overflow, and stable selection. Treat unmet performance targets as release findings.

- [ ] Prove web shutdown independence, restart recovery, rollback, and no second writer.

- [ ] Review screenshots against the mockup guide. Document justified deviations.

- [ ] Check the source revision did not change during the gates. Rerun affected checks after any relevant change.

- [ ] Check the latest owner handoff and process activity before commit/push. Stop if another Git writer or overlapping edit is active.

- [ ] Stage only this task's reviewed paths, inspect the staged diff, commit, push, and check the remote revision.

Release commands, after the proposed targets exist:

```sh
cargo test --locked --lib topology_manager
cargo test --locked --test topology_web_view
cargo test --locked --test topology_web_receipts
cargo test --locked -p web-gateway --test topology
cargo test --locked -p web-ui
cargo clippy --locked -p web-ui --target wasm32-unknown-unknown -- -D warnings
cargo fmt --all --check
bash tools/frontend/build.sh
node tools/frontend/topology-check.cjs
```

Browser commands need the isolated fixture and disposable account environment from the current frontend checks.
Do not use production credentials or production RouterOS endpoints as test fixtures.
A controlled live publication window remains a separate operator action.

## Explicit deferred work

Schedules, enrollment/activation controls, RouterOS routing writes, and broad profile-policy editors remain separate backend packages.
Each package needs its own contract, failure model, review, and release proof.
The initial web implementation must explain these boundaries with an explicit unavailable state.
