# WAN discovery implementation plan (Phase 2b-3a)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Read the interfaces of each configured router on demand and show the likely WANs in the Builder Inventory, without a registry change, and make the legacy cycle skip routers that have no subscriber protocol.

**Architecture:** A new `topology_manager::wan` module holds the pure candidate and answer rules and one bounded parallel `/interface` read. The runner adds `api_wan_discover`, which keeps the last answer in memory, and the dispatcher adds `wan.discover`. The legacy cycle filters its router loops with `RouterCfg::reads_subscribers`. The TUI adds a WAN section to the Builder Inventory.

**Tech Stack:** Rust 2024, Tokio, ratatui, serde_json. No new dependencies. MSRV 1.89.

**Spec:** `docs/superpowers/specs/2026-11-19-topology-wan-discovery-design.md`

## Global Constraints

- Do not stop, restart, deploy, or reconfigure `lqosync3.service`. Do not touch `/etc/lqosync3`, `/var/lib/lqosync3`, `/opt/libreqos/src`, or routers.
- No new dependencies. Keep `cargo +1.89 build --locked` working. No registry schema change. No new configuration key.
- Gate for every task: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`. Run cargo with `nice -n 19`, `CARGO_BUILD_JOBS=2`.
- Separate worktree, branch `topology-wan-discovery` from `lqosync3`. Do not edit `docs/OPERATIONS.md`.
- Persisted text follows ASD-STE100.
- `wan.discover` changes no registry and no file. It needs no enrollment.
- Router reads stay read-only: one `/interface` print with the `Table::Interfaces` proplist `.id,name,type,disabled,running,comment`. No double-read stability check.
- One time limit of 30 s for the whole discovery.
- `suggested` is `type == "pppoe-out"`. Interfaces whose `type` ends with `-in` are left out. Order: suggested first, then by name.
- Catalog routers match configured routers by name.

## Review Focus

1. A PPP router whose `/interface` has thousands of dynamic `pppoe-in` rows: discovery gives `ok` and no `-in` candidate. Test in Task 2.
2. A core router that stalls at connect: discovery ends at the 30 s limit, that router is `failed`, and the other routers are `ok`. Test in Task 3.
3. A confirmed WAN interface that the operator renames on the core: the old name shows in `missing`, the new name is a candidate without `source_id`. Test in Task 2.
4. A discovery while a preview refresh runs: both requests go out and both answers land. Test in Task 4.
5. A catalog router ID that does not follow `router_id(name)` (a hand-made catalog through `policy.commit`): `source_id`, `used_by`, and `missing` still match by name. Test in Task 2.

---

### Task 1: Legacy cycle skips routers without subscriber protocols

**Files:**
- Modify: `src/config.rs` (`impl RouterCfg`), `src/runner.rs` (router loops near lines 1286 and 1316)
- Test: `src/runner.rs` `mod tests`, `src/topology_manager/coordinator_tests.rs`

**Interfaces:**
- Produces: `impl RouterCfg { pub fn reads_subscribers(&self) -> bool }`, true when `pppoe.enabled || dhcp.enabled || hotspot.enabled`.

- [ ] **Step 1: Write the failing tests:**
  - `a_router_without_protocols_is_not_read_and_does_not_hold_removals` (runner tests): config `R1` (PPP) plus `Core` with no protocol table; `Core` is in `fake.failing`. Cycle 1 with two R1 users, cycle 2 with one. Assert: no print for `Core` (count prints by router in the fake), the status and collection report have no `Core` entry, and the user missing in cycle 2 is removed from the output as it is with R1 alone. Keep `network.mode = "root_promotion"`.
  - `reads_subscribers_is_false_only_with_all_protocols_disabled` (config tests).
  - `a_router_without_roles_is_not_read_for_a_proposal_or_preparation` (coordinator tests): one more router without protocols gets no connect from `read_for_proposal` or `prepare`.
- [ ] **Step 2: Run** `cargo test --locked a_router_without reads_subscribers` — Expected: the runner test FAILS (Core is read). The coordinator test can pass already; it pins current behavior.
- [ ] **Step 3: Implement.** Filter both runner loops with `reads_subscribers()`, so `Core` has no task, no outcome, and no status entry.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `fix(cycle): skip routers without subscriber protocols`.

### Task 2: WAN candidate and answer rules

**Files:**
- Create: `src/topology_manager/wan.rs` (register in `src/topology_manager/mod.rs`)
- Modify: `src/topology_manager/policy_draft.rs` (`router_id` becomes `pub(crate)`)
- Test: same file

**Interfaces:**
- Produces:
  - `pub struct RouterInterfaces { pub router_id: String, pub name: String, pub read: Result<Vec<RosRow>, String> }`
  - `pub fn answer(reads: &[RouterInterfaces], catalog: Option<&TopologyCatalog>, collected_at: u64) -> serde_json::Value`
- Answer shape (spec): `{collected_at, routers: [{router_id, name, state, error, candidates, missing}], unconfigured}`. Candidate: `{name, type, running, disabled, comment, suggested, source_id, used_by}`. `running` and `disabled` are booleans from RouterOS `"true"`/`"false"`. `comment` is `null` when absent. `missing`: `[{source_id, endpoint_ref}]`, empty for a `failed` router. `unconfigured`: `[{source_id, router_id, endpoint_ref}]`. `used_by` is a sorted list of node IDs.
- Matching: a WAN source belongs to a configured router when its catalog router has the same `name`. `unconfigured` lists WAN sources whose catalog router name has no read entry.

- [ ] **Step 1: Write the failing tests:**
  - `pppoe_out_comes_first_and_in_types_are_left_out`: rows `ether1`, `pppoe-out2`, `pppoe-out1`, `<pppoe-a>` (`pppoe-in`), `l2tp-x` (`l2tp-in`) give `pppoe-out1`, `pppoe-out2`, `ether1`; the first two have `suggested: true`.
  - `ten_thousand_dynamic_sessions_give_ok_and_no_in_candidates` (Review Focus 1).
  - `a_failed_router_gives_its_error_and_no_missing_list`.
  - `a_catalog_wan_source_gives_source_id_and_sorted_used_by`: one source with two nodes.
  - `a_renamed_interface_shows_missing_and_a_new_candidate` (Review Focus 3).
  - `catalog_routers_match_by_name` (Review Focus 5): catalog router ID `custom-core`, name `Core`.
  - `a_wan_source_on_an_unconfigured_router_is_listed`.
  - `without_a_catalog_lists_are_empty`.
- [ ] **Step 2: Run** `cargo test --locked topology_manager::wan` — Expected: FAIL (module absent).
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): add WAN candidate rules`.

### Task 3: Bounded read, runner operation, and dispatch

**Files:**
- Modify: `src/topology_manager/wan.rs`, `src/runner.rs`, `src/api/dispatch.rs`
- Test: `src/runner/tests/api.rs` (`mod operations`), `src/runner.rs` test fake

**Interfaces:**
- Consumes: Task 2 `RouterInterfaces`, `answer`; `policy_draft::router_id`.
- Produces:
  - `pub async fn read_all<C: Connector + 'static>(connector: Arc<C>, cfg: &Config, limit: Duration) -> Vec<RouterInterfaces>` in `wan.rs`. One `JoinSet`, one fresh connection for each configured router, one print of `Table::Interfaces.command()`, one deadline for all. A router not done at the deadline gets `Err("read expired")`. Other errors use the `ReadError` text. Output in config order.
  - `Runner::api_wan_discover(&mut self) -> anyhow::Result<Value>`: takes the lock, gets the catalog from `api_document()` when one exists, runs `read_all` with 30 s, calls `answer`, stores the value in a new field `wan_discovery: Option<Value>`, and returns it.
  - Dispatch: `"wan.discover"` with no arguments, like `status`.

- [ ] **Step 1: Write the failing tests:**
  - `wan_discover_lists_candidates_and_marks_failed_routers`: R1 has `pppoe-out1` and `ether1`; `Core` is failing. Answer: R1 `ok` with `pppoe-out1` first and `suggested`, `Core` `failed` with an error text.
  - `wan_discover_changes_no_registry`: before enrollment, the answer is ok and `state/topology-manager` does not exist.
  - `a_stalled_router_fails_alone_within_the_limit` (Review Focus 2): add a `stalling` router set to the runner test fake; with `tokio::time::pause`, the stalled router is `failed` with "read expired" and R1 is `ok`.
  - `wan_discover_is_dispatched`: `call(&mut r, "wan.discover", json!({}))` gives `ok: true` and a `routers` list.
- [ ] **Step 2: Run** `cargo test --locked runner::tests::api` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(api): add wan.discover`.

### Task 4: WAN candidates in the Builder Inventory

**Files:**
- Modify: `src/tui/topology/mod.rs`, `src/tui/topology/builder.rs`, `src/tui/app.rs` (Topology hint)
- Test: `src/tui/topology/builder.rs`, `src/tui/topology/mod.rs`

**Interfaces:**
- Consumes: the Task 3 answer shape.
- State: `wan: Option<Value>`, `discovering: bool` in `TopologyView`.
- Key `w` in the Builder sends `wan.discover` when `discovering` is false. The answer clears `discovering`, stores `wan`, and an error goes to `error`.
- The Inventory panel shows, below the subscriber sources, a "WAN candidates" section: before the first answer "[w] discover WANs"; while it runs "reading routers"; then each router with its state or error, each candidate with name, type, up/down, disabled, a mark for `suggested`, and its `used_by` count, each missing entry as "missing", and the `unconfigured` entries.
- The Topology hint adds `[w] WANs`.

- [ ] **Step 1: Write the failing tests:**
  - `w_in_the_builder_sends_wan_discover_once_while_it_runs`.
  - `a_discovery_during_a_preview_refresh_sends_both` (Review Focus 4): `r` then `w` gives both ops; both answers land in their own state.
  - `before_discovery_the_section_shows_the_key`.
  - `the_section_lists_candidates_with_marks_errors_and_missing_entries`.
  - `the_wan_section_fits_80_columns`.
- [ ] **Step 2: Run** `cargo test --locked tui::topology` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): show WAN candidates in the Builder`.

### Task 5: Records and final verification

**Files:**
- Modify: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`, `brain/30-knowledge/topology-manager-status.md`
- Create: `brain/10-sessions/2026-11-19-topology-wan-discovery.md`

- [ ] **Step 1: Run** `cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked && git diff --check` — Expected: all pass.
- [ ] **Step 2: Update** the records: the WAN discovery section, the legacy skip with the owner's later review, the Phase 4 runbook item (add the core `[[routers]]` entry together with the deployment of the tested build), and the 2b-3b next step.
- [ ] **Step 3: Commit** `docs(topology): record WAN discovery`.
