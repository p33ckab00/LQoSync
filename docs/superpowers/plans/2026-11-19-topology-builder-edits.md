# Builder edits implementation plan (Phase 2b-3b)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the operator build WAN, intermediate, and group instance nodes, move and remove them, and edit names and capacities, with one capacity-share rebalance committed in the same revision, through `propose` and `confirm` service operations and the Builder TUI.

**Architecture:** A pure `topology_manager::edits` module applies one edit to a registry copy, splits group minimums, and names the affected sources. A share objective replaces the `SourcePool` rebalance sweep. The runner proposes (edit, read, resolve, rebalance per affected source, aggregate check), keeps one proposal, and commits it on confirm. A new `topology.tree` operation feeds the Builder Tree. The TUI adds Inventory selection, forms, a parent picker, and a proposal popup.

**Tech Stack:** Rust 2024, Tokio, ratatui, serde_json. No new dependencies. MSRV 1.89.

**Spec:** `docs/superpowers/specs/2026-11-19-topology-builder-edits-design.md`

## Global Constraints

- Do not stop, restart, deploy, or reconfigure `lqosync3.service`. Do not touch `/etc/lqosync3`, `/var/lib/lqosync3`, `/opt/libreqos/src`, or routers.
- No new dependencies. Keep `cargo +1.89 build --locked` working. No registry schema change.
- Gate for every task: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`. Run cargo with `nice -n 19`, `CARGO_BUILD_JOBS=2`.
- Separate worktree, branch `topology-builder-edits` from `lqosync3`. Do not edit `docs/OPERATIONS.md`.
- Persisted text follows ASD-STE100.
- Edits need export ownership: "enroll first: the registry has no export ownership".
- Proposal expiry: 120 s. One proposal at a time. Stale or expired confirm: `conflict` "the proposal is stale, propose again".
- Rates: whole Mbps 1 to 1,000,000 (`RateBps` = Mbps × 1,000,000). Percent: whole 1 to 100 (`Fraction { numerator: p, denominator: 100 }`). Names: 1 to 64 characters, no outer spaces.
- New IDs only through `ids::reserve_ids` (`IdKind::Node`, `IdKind::Source`), reserved before the reconciliation allocates subscriber IDs.
- Exact refusal texts (spec): "move or remove the children first", "unpin the subscribers first", "this is the last instance of the source", "the export name comes from the legacy node", "an enrolled node keeps its legacy export name", "resolve the placement conflicts first".

## Review Focus

1. A remove of an instance whose subscribers are partly offline: online ones move by the rebalance, offline ones lose the assignment, and no `InvalidAssignment` conflict appears when they return. Test in Task 2 and Task 3.
2. Two operators: A proposes, B commits another edit, A confirms: `conflict`, and nothing of A's proposal commits. Test in Task 3.
3. An edit whose rebalance puts group minimums above a WAN ceiling: `invalid` with the aggregate conflict text and no kept proposal. Test in Task 3.
4. A distro router that is down: edits without affected sources still work; edits with affected sources answer the read error. Test in Task 3.
5. A WAN candidate that a second `wan.create` names again, or a candidate missing from the last discovery: refused. Test in Task 2.

---

### Task 1: Capacity-share objective for source pools

**Files:**
- Modify: `src/topology_manager/rebalance.rs`, `src/topology_manager/resolver.rs` (visibility only, if needed)
- Test: `src/topology_manager/rebalance_tests.rs`

**Interfaces:**
- Produces: `pub(crate) fn instance_weights(catalog: &TopologyCatalog, source_id: &str) -> anyhow::Result<Vec<(String, RateBps)>>` in `rebalance.rs`: each instance node ID of the source with the effective capacity of the first node in its target `independent_path`, or `RateBps { down: 1, up: 1 }` when the path is empty. Node order: catalog order.
- `rebalance(.., RebalanceScope::SourcePool(id))` uses the share objective (spec "Share objective"). `RebalanceScope::Branch` keeps the current sweep. Signature does not change.

- [ ] **Step 1: Write the failing tests:**
  - `equal_wans_split_one_group_five_and_five`: 10 equal-plan members all on the WAN1 instance; result 5 and 5.
  - `a_two_to_one_capacity_splits_two_to_one_within_one_subscriber`: 9 members give 6 and 3.
  - `two_groups_each_split_by_capacity_and_wan_totals_follow`: two sources with different group factors.
  - `pins_stay_and_count_in_their_instance_share`.
  - `a_balanced_group_does_not_move`.
  - `instance_weights_use_the_nearest_independent_ancestor`.
- [ ] **Step 2: Run** `cargo test --locked --lib topology_manager::resolver::rebalance` — Expected: the new tests FAIL.
- [ ] **Step 3: Implement.** Demand is the placement `base`. Each pool member has the same group factor, so the factor does not change the split; keep `base`. Existing `SourcePool` tests whose numbers contradict the share objective get the share result, with a ledger ruling for each.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): balance source pools by capacity share`.

### Task 2: Edit rules on a registry copy

**Files:**
- Create: `src/topology_manager/edits.rs`, `src/topology_manager/edits_tests.rs` (register in `mod.rs`)
- Test: `src/topology_manager/edits_tests.rs`

**Interfaces:**
- Consumes: Task 1 `instance_weights`; `ids::reserve_ids`; `wan::answer` shape for the discovery check.
- Produces:
  - `#[derive(Debug, Clone, Deserialize, Serialize)] #[serde(tag = "kind", deny_unknown_fields)] pub enum Edit` with variants renamed `wan.create`, `node.create`, `instance.add`, `node.move`, `node.update`, `node.remove` and the spec fields.
  - `#[serde(tag = "mode", rename_all = "snake_case")] pub enum CapacityInput { Inherit, Fixed { down_mbps, up_mbps, factor_percent: Option<[u64; 2]> }, WanPercent { down_percent, up_percent }, Automatic }`.
  - `pub struct Applied { pub candidate: RegistryDocument, pub affected: Vec<String>, pub cleared: Vec<String> }`.
  - `pub fn apply(doc: &RegistryDocument, edit: &Edit, discovery: Option<&serde_json::Value>, cfg: &Config) -> anyhow::Result<Applied>`. It checks export ownership, applies the edit, splits the minimums of each affected source (Task 1 weights), runs `validate`, and returns sorted unique affected source IDs. Rule failures are `InvalidPolicy` with the exact texts.
  - `pub fn tree(doc: &RegistryDocument) -> serde_json::Value`: the `topology.tree` answer (spec).

- [ ] **Step 1: Write the failing tests:**
  - `wan_create_adds_source_node_router_and_role` (core router absent from the catalog, present in `cfg`).
  - `wan_create_refuses_a_candidate_missing_from_discovery_or_already_used` (Review Focus 5).
  - `node_create_and_update_respect_capacity_modes_and_limits` (WAN fixed only, factor 1 to 100, intermediate no automatic, group automatic or fixed, rate and name limits).
  - `instance_add_accepts_only_a_selected_source_and_splits_the_minimum`: minimum 30 over equal WANs gives 15 and 15; 2:1 gives 20 and 10; a remainder keeps the exact total.
  - `move_keeps_id_capacity_and_refuses_cycles_wan_children_and_duplicate_instances`.
  - `remove_refuses_children_pins_last_instance_and_aliased_nodes`.
  - `remove_clears_offline_assignments_and_removes_an_unused_wan_source` (Review Focus 1, rule half).
  - `rename_of_an_aliased_node_is_refused`.
  - `affected_sources_follow_the_spec` (no affected source for a new WAN, intermediate, or rename).
  - `edits_before_enrollment_are_refused`.
  - `tree_lists_every_catalog_node_with_capacity_and_alias`.
- [ ] **Step 2: Run** `cargo test --locked --lib topology_manager::edits` — Expected: FAIL (module absent).
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): add Builder edit rules`.

### Task 3: Propose, confirm, cancel, and tree in the service

**Files:**
- Modify: `src/topology_manager/coordinator.rs`, `src/topology_manager/observation.rs` (derive `Clone` where the chain needs it), `src/runner.rs`, `src/api/dispatch.rs`
- Test: `src/runner/tests/api.rs` (new `mod edits`), `src/topology_manager/coordinator_tests.rs`

**Interfaces:**
- Consumes: Task 2 `Edit`, `apply`, `tree`; Task 1 rebalance; `wan::answer`.
- Produces:
  - `Coordinator::reconcile_for(&mut self, cfg: &Config, registry: &RegistryDocument, minimum_generation: u64, max_age: Duration) -> anyhow::Result<Reconciliation>`: the read and authorization part of `prepare`, without publication.
  - Runner fields: `wan_discovery: Option<Value>` (set by `api_wan_discover`), `edit_proposal: Option<EditProposal>` with `id`, `base_revision`, `candidate`, `expires: Instant`.
  - `api_edit_propose(&mut self, expected_revision: u64, edit: Edit) -> anyhow::Result<Value>`, `api_edit_confirm(&mut self, proposal_id: &str) -> anyhow::Result<Value>`, `api_edit_cancel(&mut self, proposal_id: &str) -> anyhow::Result<Value>`, `api_tree(&mut self) -> anyhow::Result<Value>`.
  - Propose with affected sources: `reconcile_for` on the candidate (30 s), then `rebalance` with `SourcePool` for each affected source in ID order on the previous result. A rebalance error "rebalance requires resolved placements" becomes `invalid` "resolve the placement conflicts first". Aggregate conflicts in the evaluation become `invalid` with their text.
  - `moves`: online subscribers whose saved assignment was `Some` and changes. `assigned`: online subscribers without a saved assignment that get one. `new_records`: records added by the reconciliation. `label` from the cached preview when present.
  - Confirm commits through the revision-checked store, drops the proposal, and clears `last_preview`.
  - Dispatch: `topology.edit.propose` (`expected_revision`, `edit`), `topology.edit.confirm` (`proposal_id`), `topology.edit.cancel` (`proposal_id`), `topology.tree` (none). `deny_unknown_fields` argument structs.

- [ ] **Step 1: Write the failing tests:**
  - `reconcile_for_reads_the_candidate_sources_without_publication` (coordinator).
  - `propose_then_confirm_commits_the_kept_candidate`.
  - `an_instance_below_a_second_wan_moves_half_of_the_group`.
  - `a_remove_moves_online_members_and_clears_offline_ones` (Review Focus 1).
  - `confirm_after_another_commit_gives_conflict_and_commits_nothing` (Review Focus 2).
  - `confirm_after_expiry_gives_conflict` (paused time).
  - `a_new_propose_replaces_the_old_one_and_cancel_drops_it`.
  - `minimums_above_a_wan_ceiling_give_invalid_and_keep_no_proposal` (Review Focus 3).
  - `a_down_distro_blocks_only_edits_with_affected_sources` (Review Focus 4).
  - `tree_and_edit_operations_are_dispatched`.
- [ ] **Step 2: Run** `cargo test --locked --lib runner::tests::api::edits coordinator` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(api): propose and confirm Builder edits`.

### Task 4: Builder Tree from the catalog, Inventory selection, and the proposal popup

**Files:**
- Create: `src/tui/topology/proposal.rs`
- Modify: `src/tui/topology/mod.rs`, `src/tui/topology/builder.rs`
- Test: same files

**Interfaces:**
- Consumes: Task 3 operations and answer shapes.
- State: `tree_data: Option<Value>` from `topology.tree` (sent with every reload and on route open), `inventory_selected: usize`, `proposal: Option<Value>`.
- The Tree draws `topology.tree` nodes in depth-first order, joined by node ID with preview counts and rates when present ("no preview" marks otherwise).
- Inventory lines are selectable (subscriber sources, then WAN candidates; the section headers and "other interfaces" lines are not).
- A `topology.edit.propose` answer opens the proposal popup: the edit kind, the number of moves, up to 8 moves (`subscriber  from -> to`), the `cleared`, `assigned`, and `new_records` counts, and `[Enter] confirm  [Esc] cancel`. Enter sends `topology.edit.confirm`, Esc sends `topology.edit.cancel`. The popup owns all keys. A confirm answer reloads status, preview, policies, and tree. `conflict` shows "changed by another operator, try again" and reloads.

- [ ] **Step 1: Write the failing tests:**
  - `the_tree_shows_catalog_nodes_without_a_preview`.
  - `an_empty_instance_stays_visible`.
  - `up_and_down_select_inventory_sources_and_wan_candidates`.
  - `a_proposal_answer_opens_the_popup_with_moves_and_counts`.
  - `enter_confirms_and_esc_cancels_the_proposal`.
  - `a_stale_confirm_shows_the_conflict_message_and_reloads`.
  - `the_proposal_popup_fits_80_columns`.
- [ ] **Step 2: Run** `cargo test --locked --lib tui::topology` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): Builder tree from the catalog and edit proposals`.

### Task 5: Builder edit forms and actions

**Files:**
- Create: `src/tui/topology/node_form.rs`, `src/tui/topology/parent_picker.rs`
- Modify: `src/tui/topology/mod.rs`, `src/tui/app.rs` (Builder hint)
- Test: same files

**Interfaces:**
- Consumes: Task 4 state; `plan_form.rs` field and error style.
- Node form fields: name; capacity mode (cycles through the modes that the node kind allows); down and up Mbps for `fixed`; factor down and up percent for a WAN (100 by default); down and up percent for `wan_percent`. Validation before send uses the spec limits; field errors show next to the field.
- WAN form: the node form for a WAN with the interface name as default name; sends `wan.create` with `router_id` and `endpoint_ref` of the selected candidate.
- Parent picker: "root" and each valid parent in tree order. Valid: not the node itself or its descendants; a WAN node gets no picker; demote lists only siblings.
- Keys, Builder screen only (exact keys are the builder's choice): Enter on an Inventory source (parent picker, `instance.add`), Enter on a WAN candidate (WAN form), new intermediate node, move, promote, demote, edit, remove (sends `node.remove` propose directly).
- Every action sends `topology.edit.propose` with `expected_revision`.

- [ ] **Step 1: Write the failing tests:**
  - `the_wan_form_defaults_to_the_interface_name_and_full_factors`.
  - `bad_rates_percent_and_names_show_field_errors_and_send_nothing`.
  - `the_mode_cycles_only_through_modes_the_kind_allows`.
  - `the_parent_picker_leaves_out_the_node_and_its_descendants`.
  - `promote_proposes_the_grandparent_and_demote_lists_siblings`.
  - `enter_on_a_source_proposes_instance_add_below_the_chosen_parent`.
  - `remove_proposes_node_remove`.
  - `forms_and_picker_fit_80_columns`.
- [ ] **Step 2: Run** `cargo test --locked --lib tui::topology` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): create, move, edit, and remove Builder nodes`.

### Task 6: Records and final verification

**Files:**
- Modify: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`, `brain/30-knowledge/topology-manager-status.md`
- Create: `brain/10-sessions/2026-11-19-topology-builder-edits.md`

- [ ] **Step 1: Run** `cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked && git diff --check` — Expected: all pass.
- [ ] **Step 2: Update** the records: the edit operations, the share objective, the minimum split, the designer calls from the spec, and the 2b-3c next step.
- [ ] **Step 3: Commit** `docs(topology): record Builder edits`.
