# Topology TUI read-only views implementation plan (Phase 2b-1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the Topology route to `lqosync3 tui` with read-only Builder, Assignments, Policies, and Changes views, fed by an on-demand preview through the service API.

**Architecture:** The service builds view models. Preparation keeps a view struct and returns conflicts as a typed error. A preview path skips writer-only checks and caches its result. API handlers become async. The TUI talks to the API on its own thread and only draws.

**Tech Stack:** Rust 2024, Tokio, ratatui (existing), serde_json. No new dependencies. MSRV 1.89.

**Spec:** `docs/superpowers/specs/2026-11-18-topology-tui-readonly-design.md` (layout baseline: `docs/design/tui/TOPOLOGY_MANAGER_LAYOUT.md`)

## Global Constraints

- Do not stop, restart, deploy, or reconfigure `lqosync3.service`. Do not touch `/etc/lqosync3`, `/var/lib/lqosync3`, `/opt/libreqos/src`, or routers.
- No new dependencies. Keep `cargo +1.89 build --locked` working.
- Gate for every task: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`. Run cargo with `nice -n 19`, `CARGO_BUILD_JOBS=2`.
- Separate worktree, branch `topology-tui-readonly` from `lqosync3`. Do not edit `docs/OPERATIONS.md`.
- Persisted text follows ASD-STE100.
- Publication keeps every existing gate. Only the preview path skips writer-only checks.
- Preview source age limit: 30 s. TUI auto refresh on route open only when the cached preview is older than 300 s or absent.
- The TUI never blocks its draw loop on an API request.
- Follow the approved layout: Topology entry after Routers, views Builder / Assignments / Policies / Changes, three Builder panels at wide sizes, one focused panel at 80 columns, a strip on every Topology view.

## Review Focus

1. A preview while the router read fails or times out: the API answers an error, the cached preview stays, and the TUI strip shows the error with the old preview age. Test in Task 3 and Task 6.
2. A preview with identity conflicts (for example a duplicate DHCP MAC): `state: "conflicts"`, the list names each account, no nodes or subscribers. Test in Task 1 and Task 3.
3. The service stops while the TUI shows the Topology route: a clear "service does not answer" message, the rest of the TUI stays usable. Test in Task 6.
4. A preview refresh during the wait before a cycle: the next cycle starts after the refresh completes, never twice, and never skipped. Test in Task 4.
5. A selection in Assignments survives a preview refresh when the subscriber still exists, and moves to the nearest row when it does not. Test in Task 8.

---

### Task 1: Conflicts as data

**Files:**
- Modify: `src/topology_manager/identity.rs` (`Authorization::reconcile` near line 77), `src/topology_manager/resolver.rs` (conflict check near line 31), `src/topology_manager/publication.rs` (aggregate check near line 199)
- Create: `src/topology_manager/conflicts.rs` (register in `mod.rs`)
- Test: `src/topology_manager/conflicts.rs` tests, plus one test in `identity_tests.rs`

**Interfaces:**
- Produces: `#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct ConflictView { pub kind: &'static str, pub subject: String, pub detail: String }` where `kind` is `"identity"`, `"placement"`, or `"aggregate"`.
- Produces: `#[derive(Debug)] pub struct PreparationConflicts(pub Vec<ConflictView>)` implementing `Display` (first conflict plus count) and `std::error::Error`.
- The three checks `bail!(PreparationConflicts(..))` instead of a text error. Identity subjects use `scope.source_id` and `row_ref`; aggregate subjects use `node_id`.

- [ ] **Step 1: Write the failing tests:** `identity_conflicts_reach_the_caller_as_data` (duplicate DHCP MAC, downcast gives two `identity` entries), `aggregate_conflicts_reach_the_caller_as_data`, `display_names_the_first_conflict_and_the_count`.
- [ ] **Step 2: Run** `cargo test --locked topology_manager::` — Expected: FAIL (type missing).
- [ ] **Step 3: Implement.** Keep the existing message text inside `detail`.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS (existing publish tests still pass).
- [ ] **Step 5: Commit** `feat(topology): return preparation conflicts as data`.

### Task 2: Preview view struct

**Files:**
- Modify: `src/topology_manager/publication.rs` (`PreparedPair`, `prepare_with_static`)
- Create: `src/topology_manager/preview.rs` (register in `mod.rs`)
- Test: `src/topology_manager/preview.rs` tests using `policy_draft::tests::{scenario, candidate}`

**Interfaces:**
- Produces: `#[derive(Debug, Clone, PartialEq, Serialize)] pub struct NodeView { pub id: String, pub label: String, pub parent: Option<String>, pub kind: String, pub source: Option<String>, pub calculated_mbps: (f64, f64), pub exported_mbps: (f64, f64), pub subscribers: u64 }`
- Produces: `#[derive(Debug, Clone, PartialEq, Serialize)] pub struct SubscriberView { pub id: String, pub account_ref: String, pub scope: String, pub ips: Vec<String>, pub macs: Vec<String>, pub node: String, pub pin: Option<String>, pub profile: Option<String>, pub min_mbps: (f64, f64), pub max_mbps: (f64, f64) }`
- Produces: `#[derive(Debug, Clone, PartialEq, Serialize)] pub struct PreviewView { pub nodes: Vec<NodeView>, pub subscribers: Vec<SubscriberView>, pub inventory: Vec<SourceView> }` with `pub struct SourceView { pub id: String, pub label: String, pub scope: String, pub kind: String, pub members: u64, pub instances: Vec<String> }`.
- Produces: `PreparedPair::view(&self) -> &PreviewView`. Built inside `prepare_with_static` from the evaluation, placements, active endpoints, and candidate subscriber policies.

- [ ] **Step 1: Write the failing tests:** `scenario_view_lists_every_node_with_calculated_and_exported_ceilings` (Gold calculated 52.5, exported 52), `scenario_view_lists_each_subscriber_with_identity_endpoints_and_node`, `inventory_counts_current_members_per_source`.
- [ ] **Step 2: Run** `cargo test --locked topology_manager::preview` — Expected: FAIL.
- [ ] **Step 3: Implement.** Mbps values come from `crate::speed::Rate::try_from(RateBps)`.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): keep a preview view in prepared pairs`.

### Task 3: Runner preview and policies

**Files:**
- Modify: `src/runner.rs`
- Test: `src/runner/tests/api.rs`

**Interfaces:**
- Consumes: Task 1 `PreparationConflicts`, Task 2 `PreviewView`.
- Produces: `pub async fn api_preview_refresh(&mut self) -> anyhow::Result<serde_json::Value>`: lock, installed policy required (else `InvalidPolicy("no installed topology policy")`), `topology_sources.prepare(cfg, doc, last_topology_generation, 30 s)` without the writer-only checks, then cache `{collected_at, registry_revision, generation, state, nodes, subscribers, inventory, conflicts, rounded}`. A `PreparationConflicts` error gives a cached result with `state: "conflicts"`. Any other error keeps the old cache and returns the error.
- Produces: `pub fn api_preview_get(&mut self) -> anyhow::Result<serde_json::Value>` (cached value or `null`).
- Produces: `pub fn api_policies(&mut self) -> anyhow::Result<serde_json::Value>`: defaults and each profile with `resolve_subscriber` min/max in Mbps, or `null` without a policy.
- Field: `last_preview: Option<serde_json::Value>`.

- [ ] **Step 1: Write the failing tests:** `preview_refresh_works_in_shadow_mode_before_enrollment`, `preview_get_reuses_the_cache_without_router_reads` (fake connector read count unchanged), `preview_with_a_duplicate_mac_is_a_conflict_state` (Review Focus 2), `a_failed_router_read_keeps_the_old_preview` (Review Focus 1), `preview_without_policy_is_invalid`, `policies_resolve_profile_rates`.
- [ ] **Step 2: Run** `cargo test --locked runner::tests::api` — Expected: FAIL.
- [ ] **Step 3: Implement.** Add a read counter to the test `Fake` connector if none exists.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): add an on-demand preview and policy view`.

### Task 4: Async API handlers and new operations

**Files:**
- Modify: `src/api/dispatch.rs`, `src/api/wait.rs`, `src/main.rs`
- Test: `src/api/wait.rs` tests, `src/runner/tests/api.rs`

**Interfaces:**
- Produces: `pub trait Handler { fn handle(&mut self, request: &Request) -> impl Future<Output = Result<String, LostLock>>; }` in `wait.rs`, implemented for `Runner<C>` by calling `dispatch::handle(self, request).await`.
- Changes: `dispatch::handle` becomes `async`. `serve_until(deadline, jobs, handler: &mut impl Handler)`.
- New operations in `dispatch`: `topology.preview.refresh`, `topology.preview.get`, `topology.policies`.

- [ ] **Step 1: Update the Task 5 tests of Phase 2a** to a test `Handler`, and **write the failing tests:** `an_async_job_that_outlasts_the_deadline_returns_after_it_without_a_second_wait` (Review Focus 4), `preview_operations_are_dispatched`.
- [ ] **Step 2: Run** `cargo test --locked api::wait` and `cargo test --locked runner::tests::api` — Expected: FAIL.
- [ ] **Step 3: Implement** and update `main.rs` to pass `&mut runner`.
- [ ] **Step 4: Run** the tests, `cargo build --locked`, and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(api): serve async preview operations`.

### Task 5: TUI API thread

**Files:**
- Create: `src/tui/api.rs` (register in `src/tui/mod.rs`)
- Test: `src/tui/api.rs` tests

**Interfaces:**
- Produces: `pub struct ApiLink { requests: std::sync::mpsc::Sender<(String, serde_json::Value)>, answers: std::sync::mpsc::Receiver<ApiAnswer> }` with `pub fn spawn(state_dir: PathBuf) -> ApiLink`, `pub fn send(&self, op: &str, args: Value)`, and `pub fn try_recv(&self) -> Option<ApiAnswer>`.
- Produces: `pub struct ApiAnswer { pub op: String, pub result: Result<Value, String> }`.
- The thread runs a current-thread Tokio runtime and `api::client::request` with the default timeout. One request at a time, in order.

- [ ] **Step 1: Write the failing tests:** `answers_come_back_in_order_without_blocking_the_caller`, `a_missing_socket_gives_a_not_running_answer`.
- [ ] **Step 2: Run** `cargo test --locked tui::api` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): add a non-blocking API link`.

### Task 6: Topology route, states, and strip

**Files:**
- Create: `src/tui/topology/mod.rs` (state and key handling), `src/tui/topology/strip.rs`
- Modify: `src/tui/app.rs` (`Tab::Topology` after `Routers`, `ALL` size), `src/tui/shell.rs`
- Test: `src/tui/topology/mod.rs` tests with `ratatui::backend::TestBackend` (follow `src/tui/app.rs:1369`)

**Interfaces:**
- Consumes: Task 5 `ApiLink`.
- Produces: `pub struct TopologyView` with `pub fn on_answer(&mut self, answer: ApiAnswer)`, `pub fn on_key(&mut self, key) -> bool`, `pub fn draw(&self, frame, area)`, and `pub enum Screen { NotEnrolled, PolicyInstalled, Builder, Assignments, Policies, Changes }`.
- Route open: send `status`, then `topology.preview.get`; when absent or older than 300 s, send `topology.preview.refresh` once. A refresh key sends refresh. View keys switch the four views (choose keys within existing global shortcut rules).

- [ ] **Step 1: Write the failing tests:** `the_topology_entry_sits_after_routers`, `null_revision_shows_not_enrolled_with_runbook_commands`, `a_registry_without_ownership_shows_policy_installed`, `an_old_cached_preview_triggers_one_refresh`, `the_strip_shows_preview_age_revision_and_legacy_writer`, `a_waiting_request_shows_waiting_for_the_service`, `a_not_running_answer_keeps_the_tui_usable` (Review Focus 3), `a_failed_refresh_keeps_the_old_preview_and_shows_the_error` (Review Focus 1).
- [ ] **Step 2: Run** `cargo test --locked tui::topology` and `cargo test --locked tui::app` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): add the Topology route and its strip`.

### Task 7: Builder view (read-only)

**Files:**
- Create: `src/tui/topology/builder.rs`
- Test: same file

**Interfaces:**
- Consumes: preview `nodes` and `inventory`.
- Produces: `pub fn draw_builder(view: &TopologyView, frame, area)`. Wide (≥ 120 columns): Inventory, Tree, Details panels. Narrow: one focused panel with Inventory / Tree / Details selectors. Tree rows are indexed once for each preview, and only the viewport is drawn.

- [ ] **Step 1: Write the failing tests:** `wide_builder_has_three_panels`, `narrow_builder_has_one_focused_panel_and_selectors`, `tree_shows_labels_and_exported_and_calculated_mbps`, `details_show_the_selected_node`.
- [ ] **Step 2: Run** `cargo test --locked tui::topology::builder` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): add the read-only Builder view`.

### Task 8: Assignments view (read-only)

**Files:**
- Create: `src/tui/topology/assignments.rs`
- Test: same file

**Interfaces:**
- Consumes: preview `subscribers`.
- Produces: `pub fn draw_assignments(...)` and selection state keyed by subscriber ID. Columns: identity, IPs, MAC, node, plan (profile), pin, min/max Mbps.

- [ ] **Step 1: Write the failing tests:** `rows_show_identity_endpoints_node_plan_and_pin`, `selection_survives_a_refresh_when_the_subscriber_remains` and `selection_moves_to_the_nearest_row_when_the_subscriber_is_gone` (Review Focus 5), `only_the_viewport_is_rendered_for_1000_subscribers`.
- [ ] **Step 2: Run** `cargo test --locked tui::topology::assignments` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): add the read-only Assignments view`.

### Task 9: Policies and Changes views (read-only)

**Files:**
- Create: `src/tui/topology/policies.rs`, `src/tui/topology/changes.rs`
- Test: same files

**Interfaces:**
- Policies consumes `topology.policies`: defaults and profiles with resolved min/max Mbps.
- Changes shows registry revision, preview generation and age, rounded nodes, and conflicts with their subject.

- [ ] **Step 1: Write the failing tests:** `policies_show_defaults_and_each_profile`, `changes_list_rounded_nodes_and_conflicts`, `changes_show_no_conflicts_when_the_preview_is_ok`.
- [ ] **Step 2: Run** `cargo test --locked tui::topology` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): add the read-only Policies and Changes views`.

### Task 10: Records and final verification

**Files:**
- Modify: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`, `docs/design/tui/TOPOLOGY_MANAGER_LAYOUT.md` (record the implemented baseline), `brain/30-knowledge/topology-manager-status.md`
- Create: `brain/10-sessions/2026-11-18-topology-tui-readonly.md`

- [ ] **Step 1: Run** `cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked && git diff --check` — Expected: all pass.
- [ ] **Step 2: Update** the records with results and the 2b-2 next step.
- [ ] **Step 3: Commit** `docs(tui): record the read-only Topology views`.
