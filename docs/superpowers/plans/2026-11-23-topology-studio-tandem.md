# Topology Studio Tandem Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task. The owner assigns Codex to frontend and Claude Code to backend. Preserve this method. Check each task before the next task.

**Goal:** Connect the approved Topology Studio UI to the current service through one typed API contract.

**Architecture:** Keep Leptos CSR and the current topology engine. Use the gateway and private broker for service access. The core owns policy decisions and publication.

**Tech Stack:** Rust 2024, Rust 1.89, Leptos `=0.8.23`, Axum `=0.8.9`, Wasm through Trunk, existing browser fixture.

**Spec:** [Product requirements](../../frontend/ROUTER_SOURCE_CONFIGURATION.md), [API contract](../../frontend/TOPOLOGY_STUDIO_API.md), [accepted SVG](../../frontend/prototypes/topology-studio-v2/01-sankey-canvas.svg).

## Global constraints

- Start from `d38113e`, then recheck current source and the worktree.

- Keep the accepted cards and Sankey composition.

- Keep every Studio action inside the canvas. Use icons, accessible names, and tooltips.

- Keep the workflow behind the information icon.

- Reuse Light, Dark, and System state. Keep values and focus during theme changes.

- Keep ordinary valid edits automatic. Preview remains optional.

- Keep one authoritative publisher. Do not change live writer ownership in this work.

- Preserve stable circuit identity, complete-read grace rules, and shared capacity budgets.

- Add no UI framework or dependency for this feature.

- Use short STE technical prose. Keep technical names exact.

- Stage explicit paths. Never commit another worker's unfinished files.

## Work ownership

| Owner | Files and responsibilities |
| --- | --- |
| Codex: frontend | `crates/web-ui/src/topology*.rs`, UI-only changes in `crates/web-ui/src/router_form.rs` and `crates/web-ui/src/router_form_model.rs`, UI module registration, `crates/web-ui/style.css`, new `tools/frontend/studio-*-check.cjs` |
| Claude Code: backend | `crates/web-contract/**`, `crates/web-gateway/**`, `src/api/**`, `src/runner.rs`, `src/runner/**`, `src/topology_manager/**`, `src/router_edit.rs`, `src/config.rs`, backend tests and migrations |
| Claude Code: shared fixture | `crates/web-gateway/examples/browser_fixture.rs`, shared fixture data under `crates/web-contract/tests/fixtures/studio/`, `tools/integration/studio-service-smoke.sh` |
| Claude Code: shared build files | Workspace manifests and `Cargo.lock`, only if required |
| Integration owner: Codex | Integration commits, combined checks, screenshots, and the acceptance report |
| Each worker | Its own dated session note and lane handoff file |

The backend owner publishes shared types. The frontend owner requests contract changes through the lane handoff.
Do not edit the shared fixture or a shared Rust type concurrently.
B0 owns mechanical consumer updates before frontend edits start.

## Parallel work and dependency gates

| Gate | Backend work | Frontend work | Exit condition |
| --- | --- | --- | --- |
| G0 | B0: types and fixture | Review contract and SVG | Contract compiles and fixture examples agree |
| G1 | B1 and B2 | F0 | Canvas shell passes browser checks without new service features |
| G2 | B3 and B4 | F1 | Modal uses the shared fixture and current request types |
| G3 | B5 | F2 and F3 | Real service and UI agree on preview, save, and error states |
| G4 | Assist integration | I0 | Combined evidence passes on one integration commit |

Create separate worktrees only when implementation starts.
Use `codex/topology-studio-ui` and `claude/topology-studio-backend` from the same G0 contract commit.
Use separate working directories under `/home/pi/LQoSync3.0-worktrees/`.

Keep `lqosync3` as the integration branch. One integration owner applies each lane's accepted commits.
Do not rewrite the other lane's branch or force-push it.

Both workers record their base commit, owned files, task, and checks in their lane handoff.
A contract change requires a new shared contract commit before either consumer relies on it.
Reserve browser fixture port `19581` for one check run at a time. Do not run the shared full-check script concurrently.

Keep separate Cargo target directories. Use at most two build jobs per worker on this host.

## Review focus

| Risk | Required test owner |
| --- | --- |
| Old response replaces a newer router draft | F2: `old_preview_cannot_replace_new_draft` |
| A route trap looks like an empty successful read | B2: `route_failure_keeps_last_evidence` |
| Credential save succeeds but policy save conflicts | B4/F3: `config_save_is_separate_from_policy_save` |
| Preview changes IDs, grace state, routing state, or outputs | B3: `preview_preserves_durable_state` |
| A large or expired result loses blockers or shows false success | B3/F2: `paged_issues_keep_complete_counts` and `expired_candidate_keeps_draft` |

## Check protocol

Each task uses this cycle:

1. Add the named failing tests and exact assertions.
2. Run the task command and make sure the failure concerns the missing behavior.
3. Add the smallest implementation that satisfies the contract.

4. Run the task command again and inspect the result.
5. Commit only the owned files. Record the commit and checks in the lane handoff.

The commands below are future implementation checks. This plan does not claim they passed.
Use the plan task ID in commit bodies and handoff entries.

## B0 — Publish the shared contract and fixture

**Owner:** Claude Code. **Depends on:** no implementation task.

**Files:** Create `crates/web-contract/src/source_studio.rs` and `crates/web-contract/tests/studio_contract.rs`.
Change `crates/web-contract/src/lib.rs`, `crates/web-contract/src/topology.rs`, `crates/web-contract/src/routers.rs`, and `crates/web-gateway/examples/browser_fixture.rs`.
Create fixture JSON under `crates/web-contract/tests/fixtures/studio/`.

Update required `Operation` and router literal constructors across the workspace before F0 starts.
Add explicit unsupported branches for new edit variants in existing exhaustive matches, including `src/topology_manager/edits.rs`.
These branches refuse unimplemented features. B0 grants no live capability. Complete mechanical UI matches before Codex takes those files.

**Interface:** Publish the types and enum variants in `TOPOLOGY_STUDIO_API.md`.
The fixture implements the new HTTP shapes with synthetic data only.
No production endpoint advertises a feature before its implementation passes.

- [ ] Add `old_operation_deserializes_without_source_result` with `source_result == None`.

- [ ] Add `studio_requests_reject_unknown_fields` and round-trip tests for every job and policy variant.

- [ ] Add fixture cases `normal`, `partial_read`, `revision_conflict`, `expired_candidate`, `lost_reply`, and `legacy_backend`.

Expose `POST /__fixture/studio/scenario` with `{ "case": "normal" }` in the signed-in loopback fixture only.
The browser checks use this fixture control. It must not exist in the production router.

- [ ] Run `cargo test --locked -p web-contract --test studio_contract` before and after implementation.

- [ ] Run `cargo check --locked --workspace --all-targets`. Record the G0 commit.

## F0 — Put the approved Studio shell in the application

**Owner:** Codex. **Depends on:** B0.

**Files:** Change `crates/web-ui/src/topology_canvas.rs`, `crates/web-ui/src/topology_canvas_model.rs`, `crates/web-ui/src/topology.rs`, `crates/web-ui/src/main.rs`, and `crates/web-ui/style.css`.
Create `crates/web-ui/src/topology_studio_help.rs` and `tools/frontend/studio-shell-check.cjs`.

**Interface:** Keep `Canvas` and current `TopologySummary`, `Flow`, and layout requests.
Use the existing theme state. Do not copy the HTML prototype into the application.

- [x] Add `tools_stay_inside_canvas`, `modal_stays_inside_canvas`, and `card_data_survives_shell_change` to the browser check.

- [x] Assert icon controls, accessible names, focus return, tooltip access, and no permanent workflow strip.

- [x] Implement fixed viewport docks. Pan and zoom change the world layer only.

- [x] Reuse current card IDs, layout saves, and service counts. Keep unavailable backend controls visibly unavailable.

- [x] Run `CHECKS=studio-shell-check STAGES=browser bash tools/frontend/check-all.sh`. Check all three themes and widths 1440, 768, and 390.

## B1 — Add access profiles and explicit draft tests

**Owner:** Claude Code. **Depends on:** B0.

**Files:** Change `src/config.rs`, `src/router_edit.rs`, `src/api/dispatch.rs`, `src/runner.rs`, `crates/web-gateway/src/http.rs`, `crates/web-gateway/src/broker.rs`, and `crates/web-gateway/src/topology_jobs.rs`.
Create `src/topology_manager/source_studio/mod.rs`, `src/topology_manager/source_studio/connection.rs`, and `crates/web-gateway/src/source_studio.rs`. Register the module in `src/topology_manager/mod.rs`.
Add tests in `connection.rs` and the existing router-edit test module.

**Interface:** `StudioJob::DraftTest` returns `DraftTestReport` through `Operation.source_result`.
Access profile updates use `RouterEdit` and its config digest. Old credentials resolve as profile `default`.

- [ ] Add `draft_test_never_saves_or_enables` and assert no config, registry, or router write occurs.

- [ ] Add `disabled_registered_router_stays_no_io`, `secret_is_absent_from_reports`, and `shared_profile_does_not_enable_writes`.

- [ ] Implement the bounded one-shot test and current reauthentication rules for credential replacement.

- [ ] Run `cargo test --locked -p lqosync3 --lib source_studio` and `cargo test --locked -p lqosync3 --lib router_edit` before and after.

- [ ] Run `cargo test --locked -p web-gateway`. Record tested features before capability advertisement.

## B2 — Add complete snapshot evidence and WAN decisions

**Owner:** Claude Code. **Depends on:** B1.

**Files:** Create `src/topology_manager/source_studio/snapshots.rs`.
Change `src/topology_manager/reader.rs`, `src/topology_manager/wan.rs`, `src/topology_manager/wan_tests.rs`, `src/runner.rs`, and `crates/web-gateway/src/source_studio.rs`.
Reuse the current WAN/source collection code and router exclusion gate.

**Interface:** `StudioJob::Collect` returns `SnapshotRef` with per-table status.
`RouterSourceConfig` exposes loaded config digest, snapshot metadata, and source capabilities.
WAN evidence keeps route table, resolved interface, time, and existing decisions. B4 adds durable new decision edits.

- [ ] Add `route_failure_keeps_last_evidence` and assert failed differs from complete-empty.

- [ ] Add `collection_limit_is_partial`, `disabled_router_is_not_collected`, and `snapshot_digest_matches_loaded_config`.

- [ ] Implement allowlisted read plans, explicit bounds, source timestamps, and last-good WAN evidence.

- [ ] Run `cargo test --locked -p lqosync3 --lib source_studio` and `cargo test --locked -p lqosync3 --lib wan` before and after.

- [ ] Run gateway tests for GET-without-router-I/O and authenticated POST collection.

## F1 — Connect the router modal and source editor

**Owner:** Codex. **Depends on:** B0 and F0. Use B0 fixtures while B1/B2 run.

**Files:** Create `crates/web-ui/src/topology_studio.rs`, `topology_studio_model.rs`, and `topology_source_editor.rs`.
Change `crates/web-ui/src/topology_canvas.rs`, `crates/web-ui/src/router_form.rs`, `crates/web-ui/src/router_form_model.rs`, `crates/web-ui/src/main.rs`, and `crates/web-ui/style.css` only where integration needs them.
Create `tools/frontend/studio-router-check.cjs`.

**Interface:** The modal reads `RouterSourceConfig` and `StudioCapabilities`.
Reuse `NewRouter`, `EditableSection`, and the current router-edit transport.
Extend the shared frontend form components when required. Do not create a second connection/credential implementation.

- [x] Add pure tests `unsupported_field_is_not_selectable`, `source_switch_keeps_each_draft`, and `unenrolled_router_keeps_connection_editor` in `topology_studio_model.rs`.

- [x] Add browser tests for source icons, access profiles, explicit test, typed query fields, group selection, and rate precedence.

- [x] Implement a `StudioDraft` state with router ID, base revisions, policy drafts, and local epoch. Keep secret inputs separate.

- [x] Run `cargo test --locked -p web-ui topology_studio_model` before and after implementation.

- [x] Run `CHECKS=studio-router-check STAGES=browser bash tools/frontend/check-all.sh` against the fixture.

## B3 — Evaluate policies without durable side effects

**Owner:** Claude Code. **Depends on:** B2.

**Files:** Create `src/topology_manager/source_studio/evaluate.rs` and `src/topology_manager/source_studio/candidate_cache.rs`.
Change `src/topology_manager/normalize.rs`, `src/topology_manager/source_policy.rs`, `src/topology_manager/source_rate.rs`, and `src/topology_manager/preview.rs` only for required reuse.
Add candidate GETs in `crates/web-gateway/src/source_studio.rs`, `crates/web-gateway/src/http.rs`, and `crates/web-gateway/src/broker.rs`. Keep raw snapshots inside the service.

**Interface:** `StudioJob::Evaluate` returns `CandidateRef`. Candidate GETs return the contract summary and pages.
Evaluate the draft within a clone of the complete current topology, with unrelated router policy intact.

- [ ] Add `preview_preserves_durable_state` and compare registry bytes, next IDs, grace state, routing state, and output hashes.

- [ ] Add `missing_ppp_join_is_a_blocker`, `dynamic_dhcp_keeps_identity`, and `hotspot_authority_is_not_invented`.

- [ ] Add `explicit_rate_precedes_profile_rate` with a 25 Mbps override and a 15 Mbps profile. Assert 25 Mbps and override provenance.

- [ ] Add `paged_issues_keep_complete_counts`, `foreign_actor_cannot_read_candidate`, and `restart_expires_candidate`.

- [ ] Reuse current identity, rate, parent, capacity, and compiler admission. Use temporary references for proposed nodes.

- [ ] Run `cargo test --locked -p lqosync3 --lib source_studio` and `cargo test --locked -p web-gateway` before and after.

## F2 — Render the candidate and its validation

**Owner:** Codex. **Depends on:** F1 and B0. Integrate B3 at G3.

**Files:** Create `crates/web-ui/src/topology_studio_api.rs` and `topology_studio_preview.rs`.
Change the Studio model, canvas projection, `crates/web-ui/src/app.rs`, and module registration.
Create `tools/frontend/studio-preview-check.cjs`.

**Interface:** Use `StudioJobRequest`, existing `Operation`, and candidate page types directly from `web-contract`.
Add a typed source-job client without changing the old endpoint error shape.

- [x] Add `old_preview_cannot_replace_new_draft` and assert an old epoch leaves the newer router and policy draft untouched.

- [x] Add `expired_candidate_keeps_draft`, `load_saved_sends_no_mutation`, and `secret_input_is_not_in_draft_state`.

- [x] Add `different_actor_cannot_recover_draft`. Reauthentication can recover only that actor's non-secret draft in the current tab.

- [x] Overlay candidate nodes and diff markers on the same canvas. Keep counts distinct from throughput.

- [x] Add paged issue/record details, source provenance, freshness, explicit refresh, and request-key recovery.

- [x] Run `cargo test --locked -p web-ui topology_studio_model` and `CHECKS=studio-preview-check STAGES=browser bash tools/frontend/check-all.sh`.

## B4 — Save source policy through the current writer path

**Owner:** Claude Code. **Depends on:** B3.

**Files:** Change `src/topology_manager/catalog.rs`, `src/topology_manager/model.rs`, `src/topology_manager/store.rs`, `src/topology_manager/edits.rs`, `src/topology_manager/web_receipts.rs`, `src/runner.rs`, and `src/runner/topology_cycle.rs`.
Change `src/sources.rs` only when the periodic read plan needs policy authority.
Create `src/topology_manager/source_studio/policy.rs` for stored policy validation and conversion.

Extend existing migration, receipt, source-policy, and publication tests.

**Interface:** `BuilderEdit::RouterSourcesSet` validates both revisions and commits the router policy set once.
`CatalogRouter.source_policies: Option<Vec<StoredSourcePolicy>>` distinguishes legacy behavior from an explicit empty policy set.
`BuilderEdit::WanCandidateSet` stores pin/ignore/clear decisions in `CatalogRouter.wan_decisions`, without router writes.

At baseline the registry schema is 9. Add the tested 9→10 migration unless a new shared baseline already owns version 10.

- [ ] Add `same_key_same_payload_saves_once`, `changed_payload_same_key_is_refused`, and `stale_config_digest_refuses_policy`.

- [ ] Add `uninitialized_registry_refuses_policy_without_side_effects`. Keep current initialization separate from activation.

- [ ] Add `config_save_is_separate_from_policy_save` and assert a policy conflict keeps the separately saved credential state.

- [ ] Add `some_empty_policies_disables_sources`, `none_keeps_current_sources`, and `partial_read_never_removes_circuits`.

- [ ] Add `wan_decision_survives_read_failure` and assert no route or WAN-list mutation.

- [ ] Integrate saved policy in periodic collection, normalization, grouping, rate resolution, and the current compiler.

- [ ] Run `cargo test --locked -p lqosync3 --lib source_studio`, existing migration/receipt tests, and the workspace gate before enabling `policy_save`.

## B5 — Complete transport and release evidence

**Owner:** Claude Code. **Depends on:** B1–B4.

**Files:** Change gateway source routes and job mapping, `src/api/dispatch.rs`, `src/runner.rs`, the shared fixture, and backend tests.
Complete `crates/web-gateway/src/source_studio.rs` for bounded cached reads and typed request adapters.
Update service command documentation with the final names.
Create `tools/integration/studio-service-smoke.sh` for HTTP → real broker → real core tests in temporary state.

**Interface:** Publish the full API contract through the real gateway and broker.
Use proposed service commands `source_studio.capabilities`, `source_studio.config`, `source_studio.test`, `source_studio.collect`, `source_studio.evaluate`, and `source_studio.candidate`.
Keep their browser payloads typed. The browser cannot choose an arbitrary service operation name.

- [ ] Add HTTP tests for same-origin, CSRF, actor scoping, request bounds, and the existing disabled-router rule.

- [ ] Add `large_policy_uses_scoped_body_limit` and keep unrelated routes at the current 4 KiB default.

- [ ] Add `lost_policy_reply_recovers_receipt` and `apply_failure_never_reports_applied`.

- [ ] Run `cargo test --locked -p web-gateway` and `cargo test --locked --workspace --no-fail-fast`.

- [ ] Run `bash tools/integration/studio-service-smoke.sh`. Use an allowlisted fake RouterOS server and test runtime, with real gateway/broker/core binaries.

The smoke script creates its own config, accounts, sockets, ports, registry, and output directory under one temporary root.
It must refuse any input that points outside that root. It never reads production config or contacts a real router.
Assert that preview changes no durable state. Assert that a valid save reaches the current publisher and produces a matching output pair.

Label importer/native responses as test evidence. They do not establish production acceptance.

- [ ] Publish the backend commit, active feature list, test logs, migration note, and rollback method in the backend handoff.

## F3 — Complete save, load, and runtime feedback

**Owner:** Codex. **Depends on:** F2 and the B4/B5 contract behavior.

**Files:** Change the Studio client/model/editor and `crates/web-ui/src/topology_forms.rs` only where shared transport needs the new result field.
Create `tools/frontend/studio-apply-check.cjs`.

**Interface:** Save access profiles with the config digest first. Wait for the loaded digest before policy save.
Use `EditRequest` with `router.sources_set` and the exact expected registry revision.
Reuse current operation status and publication evidence.

- [ ] Add `config_save_is_separate_from_policy_save`, `saved_is_not_applied`, and `retry_uses_the_same_key` browser scenarios.

- [ ] Add a policy save path that works without a prior preview. Keep draft values after any refusal.

- [ ] Make Load saved topology read cached saved state without policy, layout, router, or output mutation.

- [ ] Remove fixture-only assumptions. Keep unsupported capabilities unavailable on old services.

- [ ] Run `CHECKS=studio-router-check\ studio-preview-check\ studio-apply-check STAGES=browser bash tools/frontend/check-all.sh`.

## I0 — Integrate both lanes and prove the result

**Owner:** Codex, with backend support from Claude Code. **Depends on:** all B/F tasks.

**Files:** Integration commits only, then `docs/frontend/evidence/topology-studio-integration/README.md` and required screenshots/checksums.
Update the product implementation status and both lane handoffs after the checks.

- [ ] Integrate accepted lane commits onto one branch. Resolve shared-contract drift before code conflicts.

- [ ] Run the full workspace and frontend gates below sequentially on that exact commit.

- [ ] Run `bash tools/integration/studio-service-smoke.sh` against the integrated real binaries, with its isolated test router and runtime.

- [ ] Exercise 5,000 synthetic subscribers, partial reads, stale revisions, expired previews, restart recovery, and source-policy conflicts.

- [ ] Measure all Studio controls and panels against the canvas container bounds at 1440, 768, and 390 CSS pixels.

- [ ] Record matching policy revision, operation key, write receipt, and test-runtime evidence for the isolated end-to-end apply.

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo clippy --locked -p web-ui --target wasm32-unknown-unknown -- -D warnings
cargo test --locked --workspace --no-fail-fast
bash tools/frontend/check-all.sh
git diff --check
```

The browser gate requires `PLAYWRIGHT_MODULE`, `CHROMIUM_PATH`, `BROWSER_TEST_PASSWORD`, and Trunk, as documented in the script.
The script currently skips `pulse-check` by default. Report that skip separately. Do not describe it as a passed check.
Production rollout and writer ownership transfer remain separate release actions.

## Completion and handoff

The frontend is complete when every released control uses the shared contract and shows the real result.
The backend is complete when the real service supplies the contract and preserves the required invariants.
The feature is complete only after I0 proves both lanes together.

Use the [backend handoff](../../handoffs/2026-11-23-claude-topology-backend.md) to start Claude Code.
Use the [frontend handoff](../../handoffs/2026-11-23-codex-topology-frontend.md) to resume Codex.
These files do not start an external Claude Code session automatically.
