# Canvas Flows and Routing Balance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One card for each node on the canvas with Sankey-style flow lines, removable links, subscriber leaf labels, four layout directions, and a routing balance that gives every shaped subscriber one `via-WAN` list.

**Architecture:** The registry keeps instances. The service web view gives each node a merge key (`card_id`) and flow and balance counts; the browser draws one card for each key. A pure routing balance module assigns a WAN to each subscriber without a reachable linked WAN, with runtime state for stability; the writer and the view both use it.

**Tech Stack:** Rust 2024 (Rust 1.89 compatible), Leptos 0.8.23 CSR (Wasm, Trunk), web-contract crate, Playwright browser checks.

**Spec:** `docs/superpowers/specs/2026-11-23-canvas-flows-and-routing-balance-design.md`

## Global Constraints

- No registry schema change. `network.json` export, aggregation, and the resolver placement do not change.
- The canvas, the List view, and the card settings never show an instance, a copy of a card, or the word "instance".
- Line width: 1.5 px to 10 px, scaled by the largest flow on the canvas; 1.5 px without counts.
- Leaf labels: at most 50 shown; "+N more" opens the Assignments tab filtered on the group.
- Routing state file: `topology-manager/routing-balance.json`, mode 0600, runtime state (no registry revision).
- A WAN is a routing candidate when its WAN monitor state is not excluded (`WanState::excluded()` is false).
- Writer rules stay: owned `via-` entries only, add before remove, the removal hold.
- No new dependency. Persisted text in ASD-STE100. Stage explicit paths only. Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Checks for every task: `cargo test --locked --workspace --no-fail-fast`, workspace Clippy and Wasm Clippy with `-D warnings`, `cargo fmt --check`, `git diff --check`; web tasks also `tools/frontend/check-all.sh`.

## Review Focus

1. A subscriber whose routing WAN is still up after a restart keeps it (the state file survives; a missing or damaged file assigns by the mode once, in subscriber ID order) — Task 1 test `a_missing_state_file_assigns_once_in_id_order`.
2. Two subscribers with the same IPv4 address in one pair: one entry, never two lists — Task 1 test `one_address_gets_one_entry`.
3. A WAN node removed from the catalog: its subscribers get a new WAN by the mode, no entry in a list of a removed WAN — Task 1 test `a_removed_wan_moves_its_subscribers`.
4. A merged card with one instance whose WAN was removed: the card has no parent line and does not vanish — Task 3 test `a_card_keeps_one_card_id_after_its_wan_goes`.
5. 5,000 subscribers: the view stays bounded (leaf labels at most 50 for each card, counts only) — Task 3 test `leaf_labels_are_bounded`.

---

### Task 1: Routing balance module

**Files:**
- Create: `src/topology_manager/routing_balance.rs` (declare in `src/topology_manager/mod.rs`)
- Test: unit tests in the same file

**Interfaces:**
- Produces:
  - `pub const FILE: &str = "routing-balance.json";`
  - `pub struct RoutingState { pub assigned: BTreeMap<String, String> }` (subscriber ID → WAN node ID), with `load(dir: &Path) -> RoutingState` (missing or damaged: empty), `save(&self, dir: &Path) -> anyhow::Result<()>` (atomic, 0600).
  - `pub struct Demand { pub subscriber_id: String, pub linked: Vec<String>, pub mode: AllocationMode, pub weight: RateBps }` — `linked`: the WAN node IDs above the placement (empty for an unlinked card or a flat placement); `weight`: the base rate for Capacity and Quality.
  - `pub fn balance(demands: &[Demand], wans: &[WanCandidate], previous: &RoutingState) -> RoutingState` where `pub struct WanCandidate { pub node_id: String, pub capacity: RateBps, pub quality: Option<u64>, pub up: bool }`.
- Rules (spec section 4, decisions 4, 5, 9):
  - A subscriber with linked WANs that are up uses one of them; its current linked WAN in `previous` stays if up.
  - A subscriber whose linked WANs are all down, or with no link, uses the WANs that are up; it keeps its `previous` WAN while that WAN is up.
  - New and moved subscribers go to the best WAN by mode, comparing the total load of each WAN (all subscribers): Count = fewest subscribers; Capacity = lowest load ÷ capacity; Quality = lowest load ÷ (capacity × quality %).
  - When no WAN is up, a subscriber keeps its `previous` WAN; without one, it gets none.
  - Process subscribers in ID order, so the result does not depend on the input order.

- [ ] **Step 1: Write the failing tests:** `a_subscriber_keeps_its_wan_while_it_is_up`; `new_subscribers_fill_the_least_loaded_wan_by_count`; `capacity_mode_follows_the_wan_capacity_ratio` (WAN1 300 Mbps, WAN2 100 Mbps, 8 new equal subscribers: 6 and 2); `quality_mode_uses_capacity_times_quality`; `linked_subscribers_use_only_their_linked_wans`; `a_linked_subscriber_moves_to_an_up_wan_when_its_wans_are_down`; `nobody_moves_when_every_wan_is_down`; `a_removed_wan_moves_its_subscribers`; `a_missing_state_file_assigns_once_in_id_order`; `the_state_survives_a_save_and_a_load`.
- [ ] **Step 2: Run them and make sure that they fail.** Run: `cargo test --locked -p lqosync3 --lib routing_balance`. Expected: compile errors.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run the tests and the full checks.** Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(topology): add the routing balance`.

### Task 2: Lists from the balance, and Rebalance routing

**Files:**
- Modify: `src/topology_manager/wan_lists.rs` (`wanted`), `src/runner.rs` (`wanted_lists` near line 2543, the runner state, a new API), `src/api/dispatch.rs`, `crates/web-contract/src/topology.rs`
- Test: `src/topology_manager/wan_lists` tests, `src/runner/tests/canvas_shadow.rs`

**Interfaces:**
- Consumes: Task 1.
- Produces:
  - `pub fn demands(pair: &PreparedPair, catalog: &TopologyCatalog) -> Vec<Demand>` in `routing_balance.rs` (linked = the WAN node IDs above each instance target of the subscriber's source; flat = empty).
  - `wan_lists::wanted(pair, catalog, routing: &RoutingState) -> anyhow::Result<BTreeMap<Ipv4Addr, ListEntry>>`: one entry for each IPv4 address of each subscriber in the pair, in the list of `routing.assigned[subscriber]`; a subscriber without a routing WAN gets no entry.
  - `Runner::routing: Option<RoutingState>` (lazy load); `wanted_lists` runs `balance` with the WAN monitor snapshot, saves the state, then calls `wanted`.
  - API `topology.routing.rebalance` with `{ "apply": bool }`: without `apply`, returns `{ "moves": N }` (the count of subscribers that a balance from an empty state would move); with `apply: true`, replaces the state with that balance. Web contract: `WebCommand::RoutingRebalance { apply: bool }` (operator role).
- [ ] **Step 1: Write the failing tests:** `every_shaped_address_is_in_one_list`; `an_unlinked_card_gets_lists_from_the_balance`; `one_address_gets_one_entry` (two rows with one IPv4 in the pair fixture: one entry); `rebalance_routing_reports_then_applies`.
- [ ] **Step 2: Run them and make sure that they fail.**
- [ ] **Step 3: Implement.** Keep the writer call sites as they are; only the input of `wanted` changes.
- [ ] **Step 4: Run the tests and the full checks.** Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(topology): write the routing lists from the balance`.

### Task 3: Card IDs, flows, and leaf labels in the web view

**Files:**
- Modify: `src/topology_manager/web_view.rs` (`project`, `Input`), `crates/web-contract/src/topology.rs` (`Node`, `TopologyView`), `src/runner.rs` (`publish_web_view`)
- Test: `src/topology_manager/web_view_tests.rs`

**Interfaces:**
- Consumes: Task 1 (`balance`, `demands`), the cached preview.
- Produces (web contract, all `#[serde(default)]`):
  - `Node.card_id: String` — group with a source: `"source:<source_id>"`; router copy: `"router:<router_id>"`; else the node ID.
  - `TopologyView.flows: Vec<Flow>` with `pub struct Flow { pub from: String, pub to: String, pub subscribers: u64 }` (card IDs; one entry for each distinct parent card → child card; the subscribers of the child instances below that parent).
  - `TopologyView.balance: Vec<CardBalance>` with `pub struct CardBalance { pub card_id: String, pub by_wan: Vec<(String, u64)>, pub leaves: Vec<Leaf>, pub more: u64 }` and `pub struct Leaf { pub name: String, pub wan: String }` (WAN node ID; at most 50 leaves; `more` counts the rest).
  - The view computes the balance from the preview and the routing state without writing the state file.
- [ ] **Step 1: Write the failing tests:** `instances_of_one_source_share_one_card_id`; `router_copies_share_one_card_id`; `flows_merge_duplicate_lines_and_count_subscribers`; `a_card_keeps_one_card_id_after_its_wan_goes`; `leaf_labels_are_bounded` (60 subscribers: 50 leaves, `more` 10); `an_unlinked_card_gets_counts_for_each_wan`.
- [ ] **Step 2: Run them and make sure that they fail.**
- [ ] **Step 3: Implement.** Update the `Node` literals in the web-ui and web-gateway tests.
- [ ] **Step 4: Run the tests and the full checks.** Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(topology): give the canvas card IDs, flows, and balance counts`.

### Task 4: Card edits: remove a link, and a capacity for the whole card

**Files:**
- Modify: `src/topology_manager/edits.rs` (`node_remove` near line 760, `node_instance_add` near line 708, `split_minimum` near line 492, `Edit`), `crates/web-contract/src/topology.rs` (`BuilderEdit`)
- Test: `src/topology_manager/edits_tests.rs`

**Interfaces:**
- Produces:
  - `BuilderEdit::LinkRemove { parent: String, child: String }` (`"link.remove"`, node IDs): the only parent of `child` → `node.move` to the top level (children stay); one of several WAN parents of a router copy → remove that copy and its group instances (their subscribers go to the other instances by mode); one of several parents of a group → remove that group instance. Refused: a group and its own router card (`the router branch keeps its groups`), the last instance of a source.
  - `BuilderEdit::CardCapacity { card_id: String, capacity: Capacity }` (`"card.capacity"`): sets the capacity of every instance of the card; a fixed value is the card total, split over the instances in the ratio of their WAN capacities (decision 10).
  - Name and location of a merged card: the existing node edits, sent once for each instance (Task 6).
- [ ] **Step 1: Write the failing tests:** `removing_the_only_link_moves_the_card_to_the_top_with_its_children`; `removing_one_wan_of_a_router_card_removes_that_copy_and_its_groups`; `removing_one_wan_of_a_group_removes_that_instance`; `a_group_link_to_its_router_card_is_refused`; `a_fixed_card_capacity_is_split_by_wan_capacity` (100 Mbps over WAN 300/100: 75 and 25).
- [ ] **Step 2: Run them and make sure that they fail.**
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run the tests and the full checks.** Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(topology): remove links and set card capacity`.

### Task 5: Layout directions

**Files:**
- Modify: `crates/web-ui/src/canvas_layout.rs` (`layout`), `crates/web-ui/src/topology_canvas_model.rs` (`positions`, `wire`), `src/topology_manager/canvas.rs` (`CanvasLayout`, `save`, `load`), the layout API in `src/runner.rs` (`api_layout_save`) and `crates/web-contract/src/topology.rs` (`LayoutRequest`)
- Test: unit tests in `canvas_layout.rs`, `topology_canvas_model.rs`, `canvas.rs`

**Interfaces:**
- Produces:
  - `pub enum Direction { TopDown, BottomUp, LeftRight, RightLeft }` (serde `top_down`, `bottom_up`, `left_right`, `right_left`; default `TopDown`).
  - `layout(cards: &[CardBox], manual: &BTreeMap<String, [f64; 2]>, direction: Direction) -> BTreeMap<String, [f64; 2]>`.
  - `wire(parent, child, direction) -> String`: vertical directions use the bottom and top ports; horizontal use the right and left ports (reversed for `BottomUp` and `RightLeft`).
  - `CanvasLayout` schema 2: `direction: Direction`, `positions: BTreeMap<Direction, BTreeMap<String, [f64; 2]>>`; schema 1 loads as `TopDown` positions. `LayoutRequest` gains `direction`.
- [ ] **Step 1: Write the failing tests:** `each_direction_puts_wans_on_its_start_side`; `no_overlap_in_any_direction_for_1_to_200_cards`; `each_direction_keeps_its_own_manual_positions`; `a_schema_1_layout_loads_as_top_down`; `wires_use_the_ports_of_the_direction`.
- [ ] **Step 2: Run them and make sure that they fail.**
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run the tests and the full checks.** Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(web): add layout directions`.

### Task 6: One card for each node on the canvas

**Files:**
- Modify: `crates/web-ui/src/topology_canvas.rs`, `crates/web-ui/src/topology_canvas_model.rs`, `crates/web-ui/style.css`
- Modify: `tools/frontend/topology-canvas-check.cjs`, `tools/frontend/topology-initialization-check.cjs` (the "Add another parent" steps become link drags)
- Test: model unit tests and the browser checks

**Interfaces:**
- Consumes: Tasks 3, 4, 5.
- Produces (canvas behavior):
  - `pub(crate) fn cards(nodes: &[Node]) -> Vec<Card>` with `pub(crate) struct Card { pub id: String, pub nodes: Vec<String>, pub kind: &'static str, pub name: String, pub members: Option<u64> }`: one card for each `card_id`; layout, ports, selection, drag, and saved positions use the card ID.
  - Lines from `TopologyView.flows`, width by Global Constraints; a click selects a line; "Remove link" in the toolbar and the Delete key send `link.remove` for one parent node and child node of that flow (Task 4 decides the effect). No line between a group and its own router card offers removal.
  - A connector drop from a WAN card on a card that already has a WAN parent sends the existing instance edit; the canvas still shows one card.
  - Leaf line below each group card: "WAN1 15, WAN2 8" with a WAN colour mark; "Show subscribers" expands up to 50 labels and "+N more" links to `/topology?tab=assignments&group=<source_id>`; the layout reserves the space.
  - Card settings for a merged card: one dialog; capacity sends `card.capacity`; name and location send the node edit to each instance; the "Add another parent" and "Add another WAN parent" fields are removed.
  - Toolbar: a direction selector (Top-down, Bottom-up, Left to right, Right to left) that saves the direction.
- [ ] **Step 1: Write the failing tests:** model `one_card_for_each_card_id`; `flow_width_scales_between_1_5_and_10`; `the_router_branch_line_offers_no_removal`; browser checks: a source with two WAN parents shows one card and two lines; remove a link by mouse and by keyboard; expand leaf labels; switch the four directions; light and dark; a 390 px screen.
- [ ] **Step 2: Run them and make sure that they fail.**
- [ ] **Step 3: Implement.** Apply the Vercel frontend-design and Google Modern Web guidance (owner rule).
- [ ] **Step 4: Run the tests, the Wasm Clippy, `tools/frontend/check-all.sh`, and the full checks.** Expected: PASS and clean.
- [ ] **Step 5: Commit** `feat(web): one card for each node with flow lines`.

### Task 7: Documents

**Files:**
- Modify: `docs/operations/WAN_ROUTING_LISTS.md` (the balance for cards without a WAN link, Rebalance routing), `docs/frontend/TOPOLOGY_CANVAS_DESIGN.md` (one card for each node, flows, directions), `brain/30-knowledge/topology-manager-status.md`

- [ ] **Step 1: Update the documents.** Run the STE checker on each changed file and fix the real findings.
- [ ] **Step 2: Run the full checks.** Expected: clean.
- [ ] **Step 3: Commit** `docs(topology): describe flows and the routing balance`.
