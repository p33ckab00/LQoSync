# Subscriber plans implementation plan (Phase 2b-2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the operator create LQoSync speed plans and attach one to a DHCP/IPoE client or as an override to a PPP subscriber, through the service API and the Topology TUI.

**Architecture:** Pure plan functions edit a `RegistryDocument` copy and return typed rule errors. Runner methods add the lock, enrollment, preview, and override checks, and commit through the revision-checked store. The TUI adds a plan picker in Assignments and a Plans section with a form in Policies.

**Tech Stack:** Rust 2024, Tokio, ratatui, serde_json. No new dependencies. MSRV 1.89.

**Spec:** `docs/superpowers/specs/2026-11-18-topology-subscriber-plans-design.md`

## Global Constraints

- Do not stop, restart, deploy, or reconfigure `lqosync3.service`. Do not touch `/etc/lqosync3`, `/var/lib/lqosync3`, `/opt/libreqos/src`, or routers.
- No new dependencies. Keep `cargo +1.89 build --locked` working. No registry schema change.
- Gate for every task: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`. Run cargo with `nice -n 19`, `CARGO_BUILD_JOBS=2`.
- Separate worktree, branch `topology-subscriber-plans` from `lqosync3`. Do not edit `docs/OPERATIONS.md`.
- Persisted text follows ASD-STE100.
- Plan: profile ID `plan-<n>` (highest existing plus one), `base: Local(rate)`, `speed: SpeedOverrides::default()`, `eligible_targets: None`. Name unique among plans, 1 to 64 characters. Rates positive whole Mbps.
- Exact error texts: "enroll first: the registry has no export ownership", "remove the DHCP client override first", "the plan is in use", "the account is not in the latest preview".
- Edits need export ownership. A stale preview revision gives `conflict`.

## Review Focus

1. Two operators attach different plans to one subscriber at the same time: the second gets `conflict`, and the TUI refreshes and shows the first result. Test in Task 3 and Task 5.
2. A plan is deleted while a subscriber uses it: `invalid` "the plan is in use", and nothing changes. Test in Task 2.
3. A DHCP client whose MAC changes (new CPE) after a plan was attached: the old record keeps the plan, the new MAC is a new account without a plan, and the preview shows both correctly. Test in Task 2.
4. The plan form gets "0", "abc", "1.5", or a name that already exists: the TUI shows a field error and sends nothing. Test in Task 6.
5. A plan attached to a PPP subscriber whose secret is renamed: the renamed secret is a new identity without the plan; the old record stays. Test in Task 2.

---

### Task 1: Full identities in the preview

**Files:**
- Modify: `src/topology_manager/preview.rs`
- Test: same file

**Interfaces:**
- Produces: `#[derive(Debug, Clone, PartialEq, Serialize)] pub struct IdentityView { pub router_id: String, pub source_id: String, pub kind: String, pub account_ref: String }` and `SubscriberView.identities: Vec<IdentityView>` (sorted, unique). Keep `account_ref` and `scope` for the current TUI.

- [ ] **Step 1: Write the failing test:** `each_subscriber_lists_full_identities` (g1: one identity, kind `Ppp`, `router_id` of Hub-1).
- [ ] **Step 2: Run** `cargo test --locked topology_manager::preview` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): list full identities in the preview`.

### Task 2: Plan rules on a registry document

**Files:**
- Create: `src/topology_manager/plans.rs` (register in `mod.rs`)
- Test: same file, using `policy_draft::tests::{scenario, candidate}` for the resolver checks

**Interfaces:**
- Produces: `pub fn is_plan(p: &SubscriberProfile) -> bool` (prefix `plan-` and `Local` base).
- Produces: `pub fn create(doc: &RegistryDocument, name: &str, down_mbps: u64, up_mbps: u64) -> anyhow::Result<(RegistryDocument, String)>`.
- Produces: `pub fn update(doc: &RegistryDocument, plan_id: &str, name: Option<&str>, down_mbps: Option<u64>, up_mbps: Option<u64>) -> anyhow::Result<RegistryDocument>`.
- Produces: `pub fn delete(doc: &RegistryDocument, plan_id: &str) -> anyhow::Result<RegistryDocument>`.
- Produces: `pub fn attach(doc: &RegistryDocument, identity: &IdentityRef, plan_id: Option<&str>, in_preview: bool) -> anyhow::Result<(RegistryDocument, String)>` returning the subscriber ID. It finds the record by identity, or creates one from `next_id` when `in_preview` is true.
- All rule failures are `InvalidPolicy` with the exact texts from Global Constraints. Each function requires export ownership. `attach` refuses an identity that a `dhcp_overrides` binding uses.

- [ ] **Step 1: Write the failing tests:** `create_adds_a_local_plan_with_the_next_number`, `names_must_be_unique_and_rates_positive`, `update_and_delete_touch_only_plans`, `deleting_a_used_plan_is_refused_and_changes_nothing` (Review Focus 2), `edits_before_enrollment_are_refused`, `attach_creates_a_record_from_the_preview_with_the_next_id`, `attach_refuses_an_account_missing_from_the_preview`, `attach_refuses_a_dhcp_override_binding`, `a_dhcp_client_with_a_plan_gets_the_plan_rate_and_detach_restores_the_server_rate`, `a_ppp_override_wins_over_the_router_speed`, `a_new_mac_or_renamed_secret_is_a_new_identity_without_the_plan` (Review Focus 3 and 5).
- [ ] **Step 2: Run** `cargo test --locked topology_manager::plans` — Expected: FAIL.
- [ ] **Step 3: Implement.** Allocate subscriber IDs as `tm-sub-{:016x}` from `next_id`, then increment it, as `observation.rs` does.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): add subscriber plan rules`.

### Task 3: Runner operations and dispatch

**Files:**
- Modify: `src/runner.rs`, `src/api/dispatch.rs`
- Test: `src/runner/tests/api.rs` (an `enrolled` helper that runs the Phase 1 flow with the scenario)

**Interfaces:**
- Consumes: Task 2 functions.
- Produces: `api_plan_create(expected_revision, name, down_mbps, up_mbps) -> anyhow::Result<Value>` (`{revision, plan_id}`), `api_plan_update(...) -> Value` (`{revision}`), `api_plan_delete(expected_revision, plan_id) -> Value`, `api_subscriber_plan(expected_revision, identity: IdentityRef, plan_id: Option<String>) -> Value` (`{revision, subscriber_id}`).
- Each: lock, read the document, check `expected_revision` (`RevisionConflict`), apply the Task 2 function, commit. `api_subscriber_plan` sets `in_preview` from the cached preview identities, and gives `RevisionConflict` when the cached preview revision differs from the document revision and no record exists.
- Dispatch: `plan.create`, `plan.update`, `plan.delete`, `subscriber.plan` with `deny_unknown_fields` argument structs.

- [ ] **Step 1: Write the failing tests:** `plan_operations_commit_and_return_the_new_revision`, `a_stale_revision_gives_conflict` (Review Focus 1), `subscriber_plan_creates_a_record_for_a_new_account_seen_in_the_preview`, `a_stale_preview_gives_conflict_for_a_new_account`, `edits_before_enrollment_give_invalid`, `plan_operations_are_dispatched`.
- [ ] **Step 2: Run** `cargo test --locked runner::tests::api` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(api): add plan and subscriber plan operations`.

### Task 4: Plans in the policy view

**Files:**
- Modify: `src/runner.rs` (`api_policies`)
- Test: `src/runner/tests/api.rs`

**Interfaces:**
- `topology.policies` adds `plans: [{id, name, down_mbps, up_mbps, subscribers}]` where `subscribers` counts records that use the plan.

- [ ] **Step 1: Write the failing test:** `policies_list_plans_with_their_use_count`.
- [ ] **Step 2: Run** `cargo test --locked runner::tests::api` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): list plans in the policy view`.

### Task 5: Plan picker in Assignments

**Files:**
- Create: `src/tui/topology/picker.rs`
- Modify: `src/tui/topology/mod.rs`, `src/tui/topology/assignments.rs`
- Test: `src/tui/topology/picker.rs`

**Interfaces:**
- A key on the selected subscriber opens the picker (choose a free key within the route; `Enter` attaches, `Esc` cancels, Up/Down move). Entries: "source default", then each plan from `topology.policies` with its rate.
- Sends `subscriber.plan` with `expected_revision` from the status, the first identity of the selected subscriber, and the plan ID or `null`. On success it sends `topology.preview.refresh` and `topology.policies`. On `conflict` it refreshes and shows "changed by another operator, try again".
- The strip shows "edits apply after activation".

- [ ] **Step 1: Write the failing tests:** `the_picker_lists_source_default_then_plans`, `enter_sends_subscriber_plan_with_the_first_identity`, `esc_sends_nothing`, `a_conflict_refreshes_and_shows_a_message` (Review Focus 1), `the_strip_says_edits_apply_after_activation`, `the_picker_fits_80_columns`.
- [ ] **Step 2: Run** `cargo test --locked tui::topology` — Expected: FAIL.
- [ ] **Step 3: Implement.** The picker owns all keys while open, so global keys do not leave the route.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): attach plans from Assignments`.

### Task 6: Plans section and form in Policies

**Files:**
- Create: `src/tui/topology/plan_form.rs`
- Modify: `src/tui/topology/policies.rs`, `src/tui/topology/mod.rs`
- Test: `src/tui/topology/plan_form.rs`

**Interfaces:**
- Policies lists plans with rate and use count. Keys: new, edit, delete the selected plan (delete asks y/n).
- Form fields: name, down Mbps, up Mbps. Validation before send: name 1 to 64 characters and unique among plans; rates positive whole numbers. Field errors show next to the field. The form owns all keys while open.
- Sends `plan.create`, `plan.update`, or `plan.delete`; then refreshes policies and the preview.

- [ ] **Step 1: Write the failing tests:** `the_plans_section_lists_rate_and_use_count`, `bad_rates_and_duplicate_names_show_field_errors_and_send_nothing` (Review Focus 4: "0", "abc", "1.5", duplicate), `a_valid_form_sends_plan_create`, `delete_asks_first_and_sends_plan_delete_on_y`, `an_in_use_answer_shows_the_message`.
- [ ] **Step 2: Run** `cargo test --locked tui::topology` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): create, edit, and delete plans`.

### Task 7: Records and final verification

**Files:**
- Modify: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`, `brain/30-knowledge/topology-manager-status.md`
- Create: `brain/10-sessions/2026-11-18-topology-subscriber-plans.md`

- [ ] **Step 1: Run** `cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked && git diff --check` — Expected: all pass.
- [ ] **Step 2: Update** the records with results and the 2b-3 next step.
- [ ] **Step 3: Commit** `docs(topology): record subscriber plans`.
