# Live Source Canvas Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Topology canvas follows the routers: active PPP profiles, DHCP servers with leases, Hotspot, routers, and default-route WANs appear by themselves; the operator connects WANs, excludes sources or router nodes, and edits each card in a modal.

**Architecture:** Each cycle, the service reads the current sources with the policy proposal reader, adds missing group and router nodes to the registry (an automatic commit), and records which nodes are active in a runtime file with a grace period. Two registry flags (schema 9) exclude a source from reading or a node from the exported tree. The web UI gets a framework-independent layout engine ported from `traffic-intelligence` and renders live cards, connectors, and modals.

**Tech Stack:** Rust 2024 (Rust 1.89 compatible), Leptos 0.8.23 CSR (Wasm, Trunk), web-contract crate, Playwright browser checks.

**Spec:** `docs/superpowers/specs/2026-11-22-live-source-canvas-design.md`

## Global Constraints

- No new dependency without a reason in the task. No router writes. `[wan_writer] enabled = false` stays the default.
- Registry schema 8 today. This plan adds schema 9 with a migration arm 8→9 and a rollback note.
- Grace period: `runtime.subscriber_grace` seconds (600 on production).
- Node IDs of group nodes: `crate::topology::group_node_id(router, &GroupKey)`; names: `group_node_name`. The same source always gets the same ID.
- The default of "Exclude from network tree" for a new router node: `cfg.network.mode == NetworkMode::RootPromotion`.
- A failed, partial, or stale router read changes no activity state and adds no node.
- Persisted text in ASD-STE100. Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Shared worktree: stage explicit paths only.
- Checks for every task: `cargo test --locked --workspace --no-fail-fast`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `git diff --check`. Web tasks also: `cargo clippy --locked -p web-ui --target wasm32-unknown-unknown -- -D warnings` (read the raw log) and `tools/frontend/check-all.sh` with `BROWSER_TEST_PASSWORD`, `TRUNK`, `PLAYWRIGHT_MODULE`, `CHROMIUM_PATH` set as in earlier plans.

## Review Focus

1. A router reboot: every lease and session disappears for some minutes, then comes back. No card disappears and no node is added again (Task 3 test `a_short_outage_hides_no_card`).
2. The activity file is missing or damaged after a restart: every existing node counts as active until the first good read; nothing disappears at once (Task 3 test `a_missing_activity_file_keeps_every_node_visible`).
3. Two routers with the same PPP profile name get two groups with different IDs (Task 2 test `the_same_profile_on_two_routers_gives_two_groups`).
4. A web edit and the automatic commit at the same time: the automatic commit uses the expected revision, and on a conflict it does nothing in this cycle (Task 2 test `an_automatic_commit_yields_to_a_newer_revision`).
5. An excluded source with online subscribers: they leave the pair, and the automatic step never adds the source again (Task 4 test `an_excluded_source_is_not_read_and_not_added_again`).

---

### Task 1: The first proposal uses active sources

**Files:**
- Modify: `src/topology_manager/normalize.rs` (`CurrentProfile`, `CurrentService`, `ppp_like` accounts loop near line 345)
- Modify: `src/topology_manager/policy_draft.rs` (`propose`, lines 173–302)
- Modify: `src/topology_manager/legacy_rate_tests.rs`, `src/topology_manager/source_policy_tests.rs` (struct literals)
- Test: `src/topology_manager/policy_draft_tests.rs`

**Interfaces:**
- Produces: `CurrentProfile.active: bool` (an active PPP or Hotspot session of an account uses the profile). It replaces `in_use` from commit `3f6491d`. `CurrentService.active: bool` (a shaped lease or active session names the service). `propose` makes a group node only for active profiles and active DHCP servers, and one Hotspot group (`GroupKey::Hotspot`) for a router with an active Hotspot session. Selections still bind every profile and every configured or discovered server.

- [ ] **Step 1: Write the failing tests** in `policy_draft_tests.rs`. Change `a_ppp_profile_without_a_secret_gets_no_group` to `a_ppp_profile_without_an_active_session_gets_no_group`: give the new `Unused` profile one secret without an active session; the group list stays equal to `before`. Add `a_dhcp_server_without_a_lease_gets_no_group`: the scenario server `EMPTY` has no lease; assert no node named `DHCP-EMPTY-Hub-1`, and assert the DHCP selection list still contains a selection with `service_names == ["EMPTY"]`. Keep `a_secret_that_moves_to_a_profile_without_a_group_keeps_the_others`. Add `an_active_hotspot_gets_one_group_per_router` with a Hotspot fixture (enable `hotspot` in the scenario config, one `hotspot_active` row); assert one node with `group_node_id("Hub-1", &GroupKey::Hotspot)`.
- [ ] **Step 2: Run them and make sure that they fail.** Run: `cargo test --locked -p lqosync3 --lib policy_draft`. Expected: the new tests fail (the `EMPTY` group exists, Hotspot refused).
- [ ] **Step 3: Implement.** In `normalize.rs`, set `active` from the accounts that this read produces (`profile_ref` and `service_refs`). In `propose`, filter `b.add` by `active`, remove the `!router.hotspot.enabled` refusal, and add the Hotspot selection and group. Update the two test struct literals.
- [ ] **Step 4: Run the tests and make sure that they pass.** Run: `cargo test --locked -p lqosync3 --lib topology_manager`. Expected: PASS. `proposed_policy_reproduces_root_promotion_network` must still pass: the legacy tree has no node for `EMPTY`.
- [ ] **Step 5: Commit** `feat(topology): propose groups only for active sources`.

### Task 2: Automatic group and router nodes in each cycle

**Files:**
- Modify: `src/topology_manager/edits.rs` (next to `auto_read`, line ~1033)
- Modify: `src/runner.rs` (`prepare_topology_for` near line 2300; legacy `cycle_inner` after the router reads)
- Modify: `src/runner/topology_cycle.rs` (`topology_cycle_publish`)
- Test: `src/topology_manager/edits_tests.rs`, `src/runner/tests/canvas_shadow.rs`

**Interfaces:**
- Consumes: Task 1 `propose`.
- Produces: `pub fn auto_groups(doc: &RegistryDocument, draft: &PolicyDraft, cfg: &Config) -> anyhow::Result<Option<RegistryDocument>>` in `edits.rs`. It adds each draft group node and its source when the node ID is absent, and a router node for each router that has an active group but no router node (`kind: NodeKind::Intermediate`, `router_id: Some(router id)`, `name`: the router name, `capacity: CapacityPolicy::Inherit`, `tree_excluded` from Global Constraints once Task 4 exists). A new router node adopts the group nodes of its router that have no parent. It never removes a node, never changes an existing node, and skips an excluded source (Task 4). `None`: nothing to add.
- Produces: `Runner::sync_live_groups(&mut self, now: u64)`: runs `propose_policy` (read), then `auto_groups`, then commits with the expected revision and one web receipt (actor `service`, change `automatic group`). It runs in both writer modes after an enrolled registry exists. A read or commit failure logs a warning and changes nothing. It also returns the set of active node IDs for Task 3: `-> Option<BTreeSet<String>>` (`None` on a failed read).

- [ ] **Step 1: Write the failing tests.** In `edits_tests.rs`: `auto_groups_adds_a_new_active_profile_once` (second call returns `None`); `auto_groups_keeps_an_edited_node` (a changed parent or capacity stays); `the_same_profile_on_two_routers_gives_two_groups`; `a_new_router_node_adopts_only_parentless_groups`. In `canvas_shadow.rs`: `a_cycle_adds_the_group_of_a_new_profile` (add an active session on a new profile to the fake router; after one legacy cycle the registry has its node and one receipt by `service`); `an_automatic_commit_yields_to_a_newer_revision` (a web edit commits between read and commit; the automatic commit does nothing and the next cycle adds the node).
- [ ] **Step 2: Run them and make sure that they fail.** Run: `cargo test --locked -p lqosync3 --lib -- auto_groups a_cycle_adds an_automatic_commit the_same_profile a_new_router_node`. Expected: compile errors.
- [ ] **Step 3: Implement** `auto_groups` and `sync_live_groups`. Call `sync_live_groups` once per cycle in the legacy cycle and in `topology_cycle_publish` before `prepare_topology`. Remove the `auto_read` call in `prepare_topology_for` only if `auto_groups` covers it (it must still add missing protocol selections); else keep both.
- [ ] **Step 4: Run the tests and make sure that they pass**, then the full checks. Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(topology): add group and router nodes for active sources`.

### Task 3: Activity with a grace period

**Files:**
- Create: `src/topology_manager/activity.rs` (declare in `src/topology_manager/mod.rs`)
- Modify: `src/runner.rs` (field `activity`, `publish_web_view`), `src/topology_manager/web_view.rs` (`project`, `Input`)
- Modify: `crates/web-contract/src/topology.rs` (`Node`)
- Test: `src/topology_manager/activity.rs` (unit tests), `src/runner/tests/canvas_shadow.rs`

**Interfaces:**
- Consumes: Task 2 `sync_live_groups -> Option<BTreeSet<String>>`.
- Produces: `pub struct SourceActivity { last_active: BTreeMap<String, u64> }` with `pub const FILE: &str = "source-activity.json"`, `pub fn load(dir: &Path) -> SourceActivity` (missing or damaged file: empty, and `is_active` then treats a node as active until its first observation), `pub fn observe(&mut self, now: u64, active: &BTreeSet<String>)`, `pub fn is_active(&self, node_id: &str, now: u64, grace: u64) -> bool`, `pub fn save(&self, dir: &Path) -> anyhow::Result<()>` (atomic, mode 0600). Router nodes are active when one of their group nodes is active. WAN nodes and nodes without a source are always active.
- Produces: `web_contract::topology::Node.active: bool` (`#[serde(default = "yes")]`).

- [ ] **Step 1: Write the failing tests:** `a_node_becomes_inactive_only_after_the_grace` (observed at 1_000, not at 1_500: active at 1_599 with grace 600, inactive at 1_601); `a_short_outage_hides_no_card` (empty observations for 300 s, then the node again: active the whole time); `a_failed_read_changes_nothing` (`observe` is not called on `None`); `a_missing_activity_file_keeps_every_node_visible`; in `canvas_shadow.rs` `the_view_marks_an_inactive_group` (the view `Node.active` is `false` after the grace).
- [ ] **Step 2: Run them and make sure that they fail.** Run: `cargo test --locked -p lqosync3 --lib activity`. Expected: compile errors.
- [ ] **Step 3: Implement** the module; in the cycle, call `observe` with the set from `sync_live_groups` when it is `Some`, then `save`; `project` sets `active`.
- [ ] **Step 4: Run the tests and the full checks.** Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(topology): track active sources with a grace period`.

### Task 4: Schema 9: source exclusion and tree exclusion

**Files:**
- Modify: `src/topology_manager/catalog.rs` (`CatalogSource`, `CatalogNode`, `TopologyCatalog::layout` line 430 / `index`)
- Modify: `src/topology_manager/model.rs` (`SCHEMA = 9`), `src/topology_manager/store.rs` (migration arm near line 641)
- Modify: `src/topology_manager/edits.rs` (`Edit`, `apply`), `crates/web-contract/src/topology.rs` (`BuilderEdit`, `Source`, `Node`)
- Modify: the source selection used for preparation (`topology_sources.prepare` input) and placement (R3–R5 targets)
- Modify: `docs/frontend/DEPLOYMENT.md` (rollback: schema 9 needs the schema-8 backup)
- Test: `src/topology_manager/catalog_tests.rs`, `src/topology_manager/edits_tests.rs`, `src/topology_manager/store_recovery_tests.rs` (migration), `src/runner/tests/canvas_shadow.rs`

**Interfaces:**
- Produces: `CatalogSource.excluded: bool` and `CatalogNode.tree_excluded: bool`, both `#[serde(default, skip_serializing_if = "is_false")]`. `BuilderEdit::SourceExclude { source_id: String, excluded: bool }` (`"source.exclude"`) and `BuilderEdit::NodeTreeExclude { node_id: String, excluded: bool }` (`"node.tree_exclude"`), with matching `Edit` arms. Web view: `Source.excluded: bool`, `Node.tree_excluded: bool`.
- Rules: an excluded source is not read for shaping, its subscribers leave the pair, and `auto_groups` (Task 2) skips it. `layout()` omits a tree-excluded node and gives each of its children the nearest ancestor that is not excluded. Placement never targets a tree-excluded node. `NodeTreeExclude` is allowed only on a router node.

- [ ] **Step 1: Write the failing tests:** `schema_8_migrates_to_9_without_change`; `a_tree_excluded_router_node_passes_its_groups_to_its_parent` (WAN → router (excluded) → group: the layout has WAN → group); `an_included_router_node_is_exported`; `an_excluded_source_is_not_read_and_not_added_again`; `tree_exclusion_is_only_for_router_nodes`; `the_first_tree_with_excluded_router_nodes_equals_the_legacy_tree` (scenario: after Task 2 adds the router node with `tree_excluded = true`, `compare_legacy` gives no issue).
- [ ] **Step 2: Run them and make sure that they fail.** Expected: compile errors.
- [ ] **Step 3: Implement** the fields, the migration arm, the edits, the layout pass-through, the placement rule, and the preparation filter. Set `tree_excluded` in Task 2 `auto_groups` from Global Constraints.
- [ ] **Step 4: Run the tests and the full checks**, also the Wasm clippy. Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(topology): exclude sources and router nodes (schema 9)`.

### Task 5: Connector rules for router branches

**Files:**
- Modify: `src/topology_manager/edits.rs` (`NodeMove`, `InstanceAdd`)
- Test: `src/topology_manager/edits_tests.rs`

**Interfaces:**
- Rule: a WAN parent for a group node whose parent is a router node is refused with the text `connect the WAN to the router card`. A router node can have two or more WAN parents through instances (`InstanceAdd`) like a group source; allocation follows its group mode. If the existing instance code supports only source nodes, add router-node instances in this task.

- [ ] **Step 1: Write the failing tests:** `a_wan_connector_to_a_group_below_a_router_card_is_refused`; `a_router_card_can_have_two_wan_parents` (two instances; the export puts its groups below both WANs with the Count split).
- [ ] **Step 2: Run them and make sure that they fail.**
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run the tests and the full checks.** Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(topology): connect WANs to router cards`.

### Task 6: Default-route WAN candidates

**Files:**
- Modify: `src/topology_manager/wan.rs` (`read_all`, `answer`), `src/runner.rs` (`api_wan_discover`, cycle, `publish_web_view`)
- Modify: `crates/web-contract/src/topology.rs` (`WanCandidate`, `TopologyView`)
- Test: `src/topology_manager/wan_tests.rs`, `src/runner/tests/canvas_shadow.rs`

**Interfaces:**
- Produces: `pub fn default_route_interfaces(routes: &[RosRow]) -> BTreeSet<String>`: the interfaces named by `gateway` or `immediate-gw` of an active `/ip/route` row with `dst-address=0.0.0.0/0`, in any routing table. `read_all` also reads `/ip/route` (`dst-address,gateway,immediate-gw,active,routing-table`). `WanCandidate.default_route: bool`. `TopologyView.wan_candidates: Vec<WanCandidate>` (`#[serde(default)]`): default-route interfaces of routers with the WAN role or without a subscriber protocol, that no WAN node uses yet.
- The cycle runs the WAN read once every 10 cycles and at start, and keeps the last answer in `topology-manager/wan-discovery.json` (0600), so that the cards stay after a restart.

- [ ] **Step 1: Write the failing tests:** `default_route_interfaces_reads_every_table` (FSE-CORE rows: `wan1-rt` via `pppoe-out1`, `wan2-rt` via `pppoe-out2`, a `main` default with distance 2: result `{pppoe-out1, pppoe-out2}`); `an_inactive_default_route_is_ignored`; `a_distro_router_gives_no_wan_candidate` (a router with a subscriber protocol and no WAN role); `the_view_lists_unconfigured_wan_candidates`.
- [ ] **Step 2: Run them and make sure that they fail.**
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run the tests and the full checks.** Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(topology): list default-route WAN candidates`.

### Task 7: Live test and activation with reviewed changes

**Files:**
- Modify: `src/topology_manager/parity.rs`, `src/runner/live_test.rs` (gate near line 548, `LiveTestReport`), `src/runner/topology_cycle.rs` (`activate_topology_with`), `src/cli.rs`, `src/topology_cli.rs`
- Modify: `docs/operations/TOPOLOGY_ACTIVATION.md` (steps 4–6: `--accept-changes`; prerequisite: bulk threshold 200)
- Test: `src/topology_manager/parity_tests.rs`, `src/runner/live_test_tests.rs`, `src/cli.rs` tests

**Interfaces:**
- Produces: `pub fn needs_review(issue: &ParityIssue) -> bool` (true for `AddedRow`, `ChangedRow`, `ChangedNetwork`). `Runner::live_test(import_timeout: Duration, accept_changes: bool)`. `LiveTestReport.accepted_changes: Vec<String>` and `active_sources: Option<String>` (a SHA-256 of the sorted active node IDs from Task 3). `activate_topology(report, max_age, install, accept_changes)`. CLI: `topology live-test --report <file> [--accept-changes]`, `topology activate --report <file> [--yes] [--accept-changes]`.
- Without `--accept-changes` the gates stay exact. With it, an issue that does not need review still refuses, with the existing texts.

- [ ] **Step 1: Write the failing tests:** `needs_review_is_true_only_for_reviewed_kinds`; `live_test_with_accept_changes_passes_a_wan_parent` (enrolled fixture, then a WAN node as parent of a group: report `passed`, `accepted_changes` not empty); `live_test_without_accept_changes_stays_exact`; `activate_with_accept_changes_still_refuses_a_missing_row`; the CLI parser test for both flags.
- [ ] **Step 2: Run them and make sure that they fail.**
- [ ] **Step 3: Implement**, and update the runbook (STE checker: `python3 ~/.claude/skills/asd-ste100/scripts/ste_check.py docs/operations/TOPOLOGY_ACTIVATION.md`).
- [ ] **Step 4: Run the tests and the full checks.** Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(topology): accept reviewed changes at activation`.

### Task 8: Canvas layout engine (port)

**Files:**
- Create: `crates/web-ui/src/canvas_layout.rs` (declare in `crates/web-ui/src/lib.rs`)
- Test: unit tests in the same file (native: `cargo test --locked -p web-ui canvas_layout`)

**Interfaces:**
- Source: `traffic-intelligence` `src/ui/canvas.rs` (`layout`, `placed_edge_sized`, `fit`, `clamp_zoom`; owner's repository). Clone it to a temporary scratchpad folder only, read it, and delete the clone. Put one line at the top of the new file: `//! Ported from p33ckab00/traffic-intelligence src/ui/canvas.rs (same owner).`
- Produces: `pub enum Tier { Wan, Router, Group, Other }`; `pub struct CardBox { pub id: String, pub tier: Tier, pub parent: Option<String>, pub size: [f64; 2] }`; `pub fn layout(cards: &[CardBox], manual: &BTreeMap<String, [f64; 2]>) -> BTreeMap<String, [f64; 2]>` (layered: WAN row, router row, group row below the parent; manual positions kept; automatic cards never overlap any card, gap 16 px); `pub fn edge_path(a: [f64; 2], a_size: [f64; 2], b: [f64; 2], b_size: [f64; 2]) -> (String, f64, f64)` (SVG path and label point: vertical when the columns match within 1 px, else a side-to-side curve); `pub fn fit(bounds: [f64; 4], viewport: [f64; 2]) -> (f64, f64, f64)` (zoom, pan x, pan y, border 4 %); `pub fn clamp_zoom(z: f64) -> f64` (0.25 to 2.0).

- [ ] **Step 1: Write the failing tests:** `no_automatic_card_overlaps_for_1_to_200_cards` (random sizes 160–260 × 70–140, fixed seed); `a_manual_position_is_kept_and_avoided`; `an_empty_manual_map_is_the_reset_layout`; `a_group_is_below_its_parent`; `edge_path_is_vertical_in_one_column`; `fit_contains_every_card`.
- [ ] **Step 2: Run them and make sure that they fail.** Run: `cargo test --locked -p web-ui canvas_layout`. Expected: compile errors.
- [ ] **Step 3: Implement** by porting and generalizing the slot code to tiers.
- [ ] **Step 4: Run the tests**, the Wasm clippy, and the full checks. Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(web): add the canvas layout engine`.

### Task 9: Live canvas, connectors, and card modals

**Files:**
- Modify: `crates/web-ui/src/topology_canvas.rs` (use `canvas_layout::layout` in place of `positions`; cards; connectors; modals), `crates/web-ui/src/topology_model.rs` (view model: active, excluded, tree_excluded, wan_candidates), `crates/web-ui/src/topology_forms.rs` (modal forms)
- Modify: the Topology styles file of the web UI
- Test: `crates/web-ui` unit tests for the model, and the Topology browser checks under `tools/frontend/` (follow the existing Topology check files)

**Interfaces:**
- Consumes: Task 3 `Node.active`, Task 4 edits and flags, Task 5 rules, Task 6 `wan_candidates`, Task 8 engine.
- Behavior (from the approved mockup `canvas-cards.html`):
  - Cards: WAN (configured nodes and `wan_candidates` as "not configured"), router (with an "Excluded from tree" chip when `tree_excluded`), group (PPP profile, DHCP, Hotspot, with members). Hide a node with `active == false`. Compact NOC style with the existing theme tokens, monospace for IDs and numbers.
  - Connector: drag from the handle of a WAN card to a router or group card. A first WAN parent sends `NodeMove`; a second sends `InstanceAdd`. The UI disables a drop on a group below a router card (Task 5 text as the reason).
  - Modals (⚙): group: capacity (from plans or fixed), factor, minimum, division between WANs, Subscribers tab, "Do not read" (`SourceExclude`). WAN: capacity, address list, auto failback, read-only health; a candidate WAN saves with `wan.create`. Router: "Exclude from network tree" (`NodeTreeExclude`), location, link to the Routers screen.
  - "Excluded" list in the tool dock: excluded sources with a "Read again" action.
  - Warning on a card and in the review when `wan_lists` is `None` and the node has a WAN ancestor: `MikroTik routing does not follow yet (writer off)`.
  - Controls: zoom, Fit, Reset layout (clears saved positions), List view with the same cards and actions.
- [ ] **Step 1: Write the failing tests:** model tests `inactive_nodes_are_hidden`, `a_candidate_wan_is_a_card`, `the_drop_on_a_group_below_a_router_is_disabled`; browser checks: drag a WAN connector, open each modal and save, exclude and read again, list view by keyboard, light and dark themes, a 390 px wide screen.
- [ ] **Step 2: Run them and make sure that they fail.**
- [ ] **Step 3: Implement.** Apply the Vercel frontend-design and Google Modern Web guidance (owner rule): compact, fast, accessible.
- [ ] **Step 4: Run the tests, the Wasm clippy, `tools/frontend/check-all.sh`, and the full checks.** Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(web): show the live source canvas`.

### Task 10: Documents and status

**Files:**
- Modify: `docs/frontend/TOPOLOGY_CANVAS_DESIGN.md` (mark the replaced "fresh topology starts empty" and Inventory-only rules; link the new spec), `docs/operations/TOPOLOGY_ENROLLMENT.md` (the active-source rule of the first proposal), `brain/30-knowledge/topology-manager-status.md` (a live source canvas entry: done, not deployed)

- [ ] **Step 1: Update the documents.** Run the STE checker on each changed file and fix the real findings.
- [ ] **Step 2: Run the full checks.** Expected: clean.
- [ ] **Step 3: Commit** `docs(topology): describe the live source canvas`.
