# Topology automatic placement: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Place each subscriber in the tree of the operator automatically. Keep the router family rule. Give a flat output for an empty tree.

**Architecture:** A node can name an access router (`router_id`, schema 6). The resolver calculates one eligible set for each router for each index build. It uses that set where the fallback path applies today. A placement without a parent gives an empty `Parent Node`.

**Tech Stack:** Rust 2024 workspace, `serde`, `anyhow`, Leptos 0.8.23 (CSR, Wasm through Trunk), Playwright browser checks.

**Spec:** `docs/superpowers/specs/2026-11-22-topology-auto-placement-design.md`. Read it before Task 1.

## Global Constraints

- Do not deploy. Do not change the live registry, routers, services, or writer ownership.
- Do not change the legacy producer (`src/topology.rs`, legacy dispatch in `src/runner.rs`).
- Do not add a dependency. Keep `leptos = "=0.8.23"`.
- Keep `Tracker::write(outputs, meta, now, hub)` as the only publication boundary.
- Registry schema becomes `6`. A schema-5 file loads and migrates with no data change.
- The resolver calculates each eligible set one time for each router for each index build. Do not walk the tree for each subscriber.
- Write all docs, comments, and commit bodies in ASD-STE100. Commit summaries are imperative and 50 characters or less.
- The tree has uncommitted work from canvas Task 6 (`src/runner/tests/canvas_shadow.rs`) and from the traffic work. Do not overwrite or commit files that this plan does not name.
- Use `TRUNK=~/.cache/lqosync-web-tools/bin/trunk` and `PLAYWRIGHT_MODULE=~/.cache/lqosync-web-tools/node_modules/playwright`.

## Review Focus

1. A nested router node (RouterB inside the branch of RouterA). RouterA subscribers must not go into RouterB. Test: `nested_router_node_is_not_eligible_for_the_parent_router` (Task 2).
2. A subscriber with a saved assignment on a WAN node. The operator then adds its router node. The subscriber moves one time and stays. Test: `adding_a_router_node_moves_its_subscribers_once` (Task 2).
3. A subscriber with PPP and DHCP accounts on two routers. Test: `accounts_on_two_routers_use_the_intersection` (Task 2).
4. An empty tree with no router plan. The resolver records `MissingPlan`. It does not publish an empty pair. Test: `empty_tree_without_a_plan_is_a_conflict` (Task 3).
5. The operator removes the router of a router node from discovery. The registry must still load. Test: `router_node_of_an_absent_router_is_refused_on_edit_but_loads` (Task 1).

## File map

| File | Change |
|------|--------|
| `src/topology_manager/catalog.rs` | `CatalogNode.router_id`, validation, relaxed scope and leaf checks for automatic nodes. |
| `src/topology_manager/model.rs` | `SCHEMA = 6`. |
| `src/topology_manager/store.rs` | Migration arm `5 =>`. |
| `src/topology_manager/edits.rs` | `NodeRouter` edit, family check on `SourceAdd`, `InstanceAdd`, and `NodeMove`. |
| `src/topology_manager/resolver.rs` | Eligible set for each router, flat placement. |
| `src/topology_manager/layout.rs` | Allow automatic subscribers on `Intermediate` and `Wan` nodes. |
| `src/topology_manager/publication.rs`, `compiler.rs`, `aggregate.rs` | `Option` target. |
| `src/topology_manager/web_view.rs` | `router_id`, `auto_members`. |
| `crates/web-contract/src/topology.rs` | `Node` fields, `BuilderEdit::NodeRouter`. |
| `crates/web-ui/src/topology_canvas.rs`, `topology_model.rs`, `style.css` | Router select, chips, Inventory status. |
| `tools/frontend/topology-canvas-check.cjs` | Browser cases. |
| `docs/frontend/TOPOLOGY_CANVAS_DESIGN.md`, `brain/HANDOFF.md` | State and handoff. |

Dependency sequence: Task 1, then Task 2, then Task 3. Task 4 needs Task 1. Task 5 needs Task 4. Task 6 needs all.

---

### Task 1: Router node in the model

**Files:**
- Change: `src/topology_manager/catalog.rs` (`CatalogNode`, `index()`), `model.rs` (`SCHEMA`), `store.rs` (migration), `edits.rs`, `crates/web-contract/src/topology.rs` (`BuilderEdit`)
- Test: `src/topology_manager/catalog_tests.rs`, `store_recovery_tests.rs`, `edits_tests.rs`

**Interfaces:**
- Produces: `CatalogNode.router_id: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- Produces: `BuilderEdit::NodeRouter { node_id: String, router_id: Option<String> }`.
- Produces: `TopologyCatalog::router_node(&self, router_id: &str) -> Option<usize>` (node index).
- Produces: `TopologyCatalog::family_of(&self, node: usize) -> Option<&str>` (nearest router node ID at or above `node`).

- [ ] **Step 1: Write the tests that fail**

```rust
// catalog_tests.rs
#[test] fn router_node_round_trips_and_old_nodes_omit_it()      // serde: None is not written; Some("r1") survives.
#[test] fn router_node_must_name_a_catalog_router()             // validate() error contains "router node"
#[test] fn one_router_has_at_most_one_router_node()              // second node with router_id "r1" -> error
#[test] fn wan_node_cannot_be_a_router_node()                   // kind Wan + router_id -> error
#[test] fn family_of_returns_the_nearest_router_node()          // RouterB inside RouterA: family_of(child of B) == Some("rB")
// store_recovery_tests.rs
#[test] fn schema_5_registry_migrates_to_6_without_change()     // all fields equal except schema_version == 6; backup exists
// edits_tests.rs
#[test] fn node_router_edit_sets_and_clears_the_router()
#[test] fn source_of_router_a_is_refused_in_the_branch_of_router_b() // error contains "another router"
#[test] fn router_node_of_an_absent_router_is_refused_on_edit_but_loads()
```

- [ ] **Step 2: Run the tests and see them fail**

Run: `cargo test --locked -p lqosync3 --lib -- router_node family_of schema_5 node_router source_of_router_a`
Expected: FAIL, compile errors for `router_id` and `NodeRouter`.

- [ ] **Step 3: Implement the model, migration, and edit**

- Validation: `router_id` must be in `catalog.routers` on edit. On load, an absent router gives an issue, not an error (Review Focus 5).
- Migration arm `5 =>` decodes, sets `schema_version = SCHEMA`, and keeps all data. Change the error text of the current arm to "schema-6".
- The family check runs in the current edit validation path. It compares the router of the source (`SourceScope.router_id`) with `family_of(target)`.

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test --locked -p lqosync3 --lib topology_manager:: && cargo test --locked -p web-contract`
Expected: PASS, 0 failed.

- [ ] **Step 5: Commit**

```bash
git add src/topology_manager/{catalog,model,store,edits}.rs src/topology_manager/{catalog_tests,store_recovery_tests,edits_tests}.rs crates/web-contract/src/topology.rs
git commit -m "feat(topology): add router nodes and schema 6"
```

### Task 2: Eligible sets for each router

**Files:**
- Change: `src/topology_manager/resolver.rs` (`Index::new`, `Index::prepare`), `catalog.rs` (`checked_scopes`), `layout.rs` (`validate_layout`)
- Test: `src/topology_manager/resolver_tests.rs`

**Interfaces:**
- Consumes: `router_node`, `family_of` from Task 1.
- Produces: `Index.auto_targets: HashMap<&'a str, BTreeSet<usize>>` (router ID to target indexes). Absent key means flat.
- Produces: `Index::auto_for(&self, router_id: &str) -> Option<&BTreeSet<usize>>`.

- [ ] **Step 1: Write the tests that fail**

```rust
#[test] fn unplaced_source_with_router_node_goes_to_its_children()      // R3: target in {Tower1, Tower2}
#[test] fn router_node_without_children_takes_its_subscribers()          // R3: target == RouterA
#[test] fn router_without_node_goes_to_its_own_wan()                     // R4: target == WAN2 (own WAN source of rC)
#[test] fn router_without_node_or_wan_goes_to_the_roots()                // R5: target in roots
#[test] fn roots_that_are_other_router_nodes_give_no_parent()            // R6: target_id == None
#[test] fn nested_router_node_is_not_eligible_for_the_parent_router()    // RouterA subscribers never in RouterB subtree
#[test] fn adding_a_router_node_moves_its_subscribers_once()             // cycle 1: WAN1; edit; cycle 2: RouterA child; cycle 3: same
#[test] fn saved_assignment_stays_when_a_new_child_appears()            // stability rule
#[test] fn accounts_on_two_routers_use_the_intersection()               // empty intersection -> NoEligibleTarget
#[test] fn explicit_fallback_keeps_priority_and_obeys_the_family_rule()
#[test] fn placed_source_ignores_automatic_sets()                       // R2 unchanged
```

- [ ] **Step 2: Run the tests and see them fail**

Run: `cargo test --locked -p lqosync3 --lib topology_manager::resolver`
Expected: FAIL.

- [ ] **Step 3: Implement the eligible sets**

- In `Index::new`, after the queue pass, build `auto_targets` one time for each router in `catalog.routers`. Use `children`, the router node, the WAN nodes (`SourceOrigin::Wan { router_id, .. }` on `node.source_id`), and the roots. Remove each node where `family_of(node)` is a different router.
- Make a `Target` for each node in an automatic set that is not a leaf. Use the same `independent_path` walk. Give it an empty `source_path`.
- In `prepare`, where `!any_matched`: if `catalog.fallback` is `Some`, keep the current path and filter it with the family rule. Else use `auto_for(router)` and intersect over all accounts.
- Relax `checked_scopes`: a scope with no catalog source is valid if its router is in `catalog.routers`. An `assignment` may name a node in an automatic set.
- Relax `validate_layout`: `Intermediate` and `Wan` nodes can have automatic subscribers. Keep the rule that an `Intermediate` node cannot use `Automatic` capacity.

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test --locked -p lqosync3 --lib topology_manager::`
Expected: PASS, 0 failed.

- [ ] **Step 5: Commit**

```bash
git add src/topology_manager/{resolver,catalog,layout}.rs src/topology_manager/resolver_tests.rs
git commit -m "feat(topology): place unplaced sources by router"
```

### Task 3: Flat output

**Files:**
- Change: `src/topology_manager/resolver.rs` (`Placement`), `publication.rs` (CSV rows), `compiler.rs`, `aggregate.rs`, the topology cycle where `catalog` is `None`
- Test: `src/topology_manager/resolver_tests.rs`, `compiler_tests.rs`

**Interfaces:**
- Produces: `Placement.target_id: Option<String>`. `None` writes `parent_node = ""` and `parent_node_id = ""`.
- Consumes: `Index.auto_targets` from Task 2.

- [ ] **Step 1: Write the tests that fail**

```rust
#[test] fn empty_tree_places_every_subscriber_without_a_parent()   // all target_id == None
#[test] fn empty_tree_without_a_plan_is_a_conflict()               // MissingPlan; candidate == None
#[test] fn flat_pair_has_empty_parent_and_an_empty_network()       // network.json == {}; Parent Node == ""
#[test] fn missing_catalog_uses_the_default_catalog()              // registry.catalog None -> flat, not an error
```

- [ ] **Step 2: Run the tests and see them fail**

Run: `cargo test --locked -p lqosync3 --lib -- empty_tree flat_pair missing_catalog`
Expected: FAIL.

- [ ] **Step 3: Implement the flat path**

- If the catalog has no nodes, the resolver places each active subscriber with `target_id: None`. The rates come from the router plan. No plan gives `MissingPlan`.
- `aggregate.rs` skips a `None` target. `publication.rs` writes empty parent fields. Do not use a fake root node.

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test --locked -p lqosync3 --lib -- topology_manager:: writer:: && cargo test --locked -p lqosync3 --test write_receipt`
Expected: PASS, 0 failed.

- [ ] **Step 5: Commit**

```bash
git add src/topology_manager/{resolver,publication,compiler,aggregate}.rs src/topology_manager/{resolver_tests,compiler_tests}.rs
git commit -m "feat(topology): publish a flat pair for an empty tree"
```

### Task 4: Web view and contract

**Files:**
- Change: `crates/web-contract/src/topology.rs` (`Node`), `src/topology_manager/web_view.rs`, `crates/web-gateway/examples/browser_fixture.rs`
- Test: `src/topology_manager/web_view_tests.rs`, `crates/web-contract` tests

**Interfaces:**
- Consumes: `CatalogNode.router_id`, `Placement.target_id`.
- Produces: `Node.router_id: Option<String>` and `Node.auto_members: Option<u64>`, both `#[serde(default)]`.
- Produces: `Source.auto_parent: Option<String>` (node ID, or `None` for flat), `#[serde(default)]`.

- [ ] **Step 1: Write the tests that fail**

```rust
#[test] fn view_shows_router_nodes_and_automatic_counts()  // router_id == Some("rA"); auto_members == Some(2)
#[test] fn view_shows_the_automatic_parent_of_a_source()    // auto_parent == Some("wan1")
#[test] fn old_view_json_without_new_fields_decodes()       // web-contract
```

- [ ] **Step 2: Run the tests and see them fail**

Run: `cargo test --locked -p lqosync3 --lib topology_manager::web_view && cargo test --locked -p web-contract`
Expected: FAIL.

- [ ] **Step 3: Implement the fields from the last resolution. Calculate them in one pass over the placements.**

- [ ] **Step 4: Run the tests and see them pass.** Use the commands of Step 2. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/web-contract/src/topology.rs src/topology_manager/{web_view,web_view_tests}.rs crates/web-gateway/examples/browser_fixture.rs
git commit -m "feat(topology): show router nodes and auto counts"
```

### Task 5: Canvas controls

**Files:**
- Change: `crates/web-ui/src/topology_canvas.rs` (`NodeForm`, `form_edits`, `NodeEditor`), `topology_model.rs`, `style.css` (scoped to the canvas)
- Test: `tools/frontend/topology-canvas-check.cjs`

**Interfaces:**
- Consumes: `Node.router_id`, `Node.auto_members`, `Source.auto_parent`, `BuilderEdit::NodeRouter`.

- [ ] **Step 1: Add the browser cases that fail**

- `router select lists only routers without a node` (expect the option of the current node and `None`).
- `saving a router sends one NodeRouter edit` (expect one request body with `node_router`).
- `router node shows a badge and auto chip` (expect text `+2 auto`).
- `inventory shows the automatic parent` (expect text `Auto: WAN1`, and `Auto: no parent` for flat).
- `refused family edit keeps the draft` (expect the service sentence in the modal, focus on the select).
- `Escape closes the modal and returns focus` and `no overflow at 375 px and 768 px` in light and dark themes.

- [ ] **Step 2: Run the check and see it fail**

Run: `tools/frontend/check-all.sh`
Expected: `topology-canvas-check` FAIL.

- [ ] **Step 3: Implement the controls**

- Add `router: String` to `NodeForm`. `form_edits` adds `NodeRouter` only when the value changes.
- Derive the select options and the chips with `Memo` from the current view. Do not add a fetch.
- Each control has a visible `<label>`. Each chip has text, not only color.
- Reserve the space of the chip, so a count change gives no layout shift.

- [ ] **Step 4: Run the checks and see them pass**

Run: `tools/frontend/check-all.sh`
Expected: all PASS (`pulse-check` can SKIP).

- [ ] **Step 5: Commit**

```bash
git add crates/web-ui/src/{topology_canvas,topology_model}.rs crates/web-ui/style.css tools/frontend/topology-canvas-check.cjs
git commit -m "feat(canvas): add router select and auto status"
```

### Task 6: Acceptance and docs

**Files:**
- Change: `docs/frontend/TOPOLOGY_CANVAS_DESIGN.md` ("Subscriber allocation"), `brain/HANDOFF.md`
- Test: if canvas Task 6 has a commit, add cases to its shadow test. Else, record the gap.

- [ ] **Step 1: Run the workspace checks**

Run: `cargo fmt --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace`
Expected: 0 failed, no warnings.

- [ ] **Step 2: Run `tools/frontend/check-all.sh`.** Expected: all PASS.

- [ ] **Step 3: Update the design doc state.** Write R1–R6 and the open owner question about stability.

- [ ] **Step 4: Run the STE check**

Run: `python3 ~/.claude/skills/asd-ste100/scripts/ste_check.py docs/frontend/TOPOLOGY_CANVAS_DESIGN.md`
Expected: no new findings in the changed lines.

- [ ] **Step 5: Add a `brain/HANDOFF.md` entry** with commits, checks, open decisions, and "not deployed".

- [ ] **Step 6: Commit**

```bash
git add docs/frontend/TOPOLOGY_CANVAS_DESIGN.md brain/HANDOFF.md
git commit -m "docs(topology): record automatic placement state"
```
