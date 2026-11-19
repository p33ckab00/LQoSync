# Explicit rebalance and subscriber pins implementation plan (Phase 2b-3c)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add explicit group rebalance and subscriber pin and unpin as Builder edit kinds, with the existing propose and confirm flow, and the TUI keys and views for them.

**Architecture:** `edits::apply` gets two edit kinds and tells the runner which subscribers must be online and whether offline assignments in the rebalanced groups are cleared. The runner applies these two rules around the existing rebalance chain. `topology.tree` lists pins. The TUI adds rebalance keys in the Builder, a WAN picker in Assignments, offline pinned rows, and popup texts.

**Tech Stack:** Rust 2024, Tokio, ratatui, serde_json. No new dependencies. MSRV 1.89.

**Spec:** `docs/superpowers/specs/2026-11-19-topology-rebalance-and-pins-design.md`

## Global Constraints

- Do not stop, restart, deploy, or reconfigure `lqosync3.service`. Do not touch `/etc/lqosync3`, `/var/lib/lqosync3`, `/opt/libreqos/src`, or routers.
- No new dependencies. Keep `cargo +1.89 build --locked` working. No registry schema change.
- Gate for every task: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`. Run cargo with `nice -n 19`, `CARGO_BUILD_JOBS=2`.
- Separate worktree, branch `topology-rebalance-pins` from `lqosync3`. Do not edit `docs/OPERATIONS.md`.
- Persisted text follows ASD-STE100.
- Exact refusal texts (spec): "the group has only one instance", "no group has two or more instances", "confirm an edit or rebalance first, so the account gets a record", "the subscriber has no placement yet", "the node is not a WAN", "the group has no instance below this WAN", "the group has more than one instance below this WAN", "the subscriber has no pin", "the subscriber must be online to pin".
- The `Branch` rebalance is not offered. Pins never move in a rebalance.

## Review Focus

1. A pinned subscriber goes offline and its pin blocks `node.remove`: the operator unpins it by `subscriber_id` from the offline row. Test in Task 2 and Task 3.
2. A rebalance right after many new subscribers landed on one WAN: the group returns to the capacity split, and returning offline subscribers do not pull it off again. Test in Task 2.
3. A pin to a WAN where the group has no instance: refused, nothing kept. Test in Task 1.
4. A re-pin from WAN1 to WAN2 of an already pinned subscriber: the pin and the assignment move to the WAN2 instance. Test in Task 1.
5. `rebalance all` with one group that has a single instance and one with two: the single-instance group is skipped, the other is balanced. Test in Task 1.

---

### Task 1: Rebalance and pin edit kinds

**Files:**
- Modify: `src/topology_manager/edits.rs`
- Test: `src/topology_manager/edits_tests.rs`

**Interfaces:**
- Produces:
  - `Edit::Rebalance { source_id: Option<String> }` (`kind: "rebalance"`).
  - `Edit::SubscriberPin { identity: Option<IdentityInput>, subscriber_id: Option<String>, wan_node_id: Option<String> }` (`kind: "subscriber.pin"`), where `IdentityInput { router_id, source_id, kind: SourceKind, account_ref }` has the `subscriber.plan` shape. A pin needs `identity`; an unpin takes `identity` or `subscriber_id` (exactly one, else "give identity or subscriber_id").
  - `Applied` adds `require_online: Vec<String>` (subscriber IDs that the read must contain) and `clear_offline: bool` (true for `rebalance`).
  - `tree(doc)` adds `pins: [{subscriber_id, node_id}]`.

- [ ] **Step 1: Write the failing tests:**
  - `rebalance_one_group_marks_it_and_refuses_a_single_instance`.
  - `rebalance_all_skips_single_instance_groups_and_refuses_when_none_is_left` (Review Focus 5).
  - `pin_sets_pin_and_assignment_to_the_group_instance_below_the_wan` (sets `require_online` to the record ID and marks the group).
  - `a_repin_moves_the_pin_to_the_other_wan` (Review Focus 4).
  - `pin_refusals_follow_the_spec` (no record, no assignment, not a WAN, no instance, two instances; Review Focus 3).
  - `unpin_by_identity_or_subscriber_id_keeps_the_assignment_and_marks_no_source`.
  - `unpin_without_a_pin_is_refused`.
  - `tree_lists_pins`.
- [ ] **Step 2: Run** `cargo test --locked --lib topology_manager::edits` — Expected: FAIL.
- [ ] **Step 3: Implement.** The group of a subscriber is the `source_id` of its saved assignment node. The instance below a WAN is a node with that `source_id` whose ancestor chain contains the WAN node.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): add rebalance and pin edits`.

### Task 2: Online check, offline clearing, and pins in the tree

**Files:**
- Modify: `src/runner.rs` (`api_edit_propose`, `api_tree`), `src/api/dispatch.rs` (no new operation; the edit kinds pass through `topology.edit.propose`)
- Test: `src/runner/tests/api.rs` (`mod edits`)

**Interfaces:**
- Consumes: Task 1 `Applied.require_online`, `Applied.clear_offline`, `tree` pins.
- `api_edit_propose`: after `reconcile_for`, refuse when an ID in `require_online` is not active ("the subscriber must be online to pin"). When `clear_offline`, set `assignment = None` for each record that is not active, has no pin, and has an assignment to an instance of an affected source; add those IDs to `cleared` before the rebalance chain.
- `api_tree`: each pin gets `online: bool`, true when the record identity is in the cached preview subscribers.

- [ ] **Step 1: Write the failing tests:**
  - `rebalance_returns_a_drifted_group_to_the_capacity_split` (Review Focus 2).
  - `rebalance_clears_offline_unpinned_assignments_of_the_group`.
  - `a_pinned_subscriber_does_not_move_and_its_demand_counts`.
  - `a_pin_of_an_offline_subscriber_is_refused`.
  - `unpin_reads_no_router_and_moves_nothing`.
  - `an_offline_pin_that_blocks_remove_is_cleared_by_subscriber_id` (Review Focus 1).
  - `tree_pins_carry_the_online_state`.
- [ ] **Step 2: Run** `cargo test --locked --lib runner::tests::api::edits` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(api): online pins and offline clearing in edit proposals`.

### Task 3: TUI rebalance keys, WAN picker, pin column, and offline pins

**Files:**
- Create: `src/tui/topology/wan_picker.rs`
- Modify: `src/tui/topology/mod.rs`, `src/tui/topology/assignments.rs`, `src/tui/topology/proposal.rs`, `src/tui/app.rs` (hints)
- Test: same files

**Interfaces:**
- Consumes: Task 1 and Task 2 shapes.
- Builder keys: `g` with an Inventory source selected proposes `{"kind": "rebalance", "source_id": <id>}`; `G` proposes `{"kind": "rebalance", "source_id": null}`.
- Assignments key `P` on the selected row opens the WAN picker: each WAN node from `topology.tree` in tree order, then "unpin" when the row has a pin. Enter proposes `subscriber.pin` with the row's first identity and the WAN node ID, or `null` for unpin. Esc cancels. The picker owns all keys.
- Offline pinned records from `topology.tree` pins with `online: false` show after the online rows as "<subscriber_id>  offline  pinned <node name>". `P` on such a row opens the picker with only "unpin", which proposes `{"kind": "subscriber.pin", "subscriber_id": <id>, "wan_node_id": null}`.
- The pin column shows the pinned node name from `topology.tree` (falls back to the ID).
- Popup change lines: "rebalance <source name or id>", "rebalance all groups", "pin <account_ref or subscriber_id> to <WAN name>", "unpin <account_ref or subscriber_id>".

- [ ] **Step 1: Write the failing tests:**
  - `g_and_shift_g_propose_one_group_and_all_groups`.
  - `the_wan_picker_lists_wans_and_unpin_only_for_a_pinned_row`.
  - `enter_in_the_wan_picker_proposes_subscriber_pin_with_the_first_identity`.
  - `offline_pinned_records_show_after_online_rows_and_offer_only_unpin`.
  - `the_pin_column_shows_the_node_name`.
  - `popup_lines_name_rebalance_and_pin_changes`.
  - `the_wan_picker_fits_80_columns`.
- [ ] **Step 2: Run** `cargo test --locked --lib tui::topology` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): rebalance groups and pin subscribers`.

### Task 4: Records and final verification

**Files:**
- Modify: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`, `brain/30-knowledge/topology-manager-status.md`
- Create: `brain/10-sessions/2026-11-19-topology-rebalance-and-pins.md`

- [ ] **Step 1: Run** `cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked && git diff --check` — Expected: all pass.
- [ ] **Step 2: Update** the records: the two edit kinds, the online pin rule, offline clearing, offline unpin, and the 2b-4 next step.
- [ ] **Step 3: Commit** `docs(topology): record explicit rebalance and pins`.
