# WAN address-list read: implementation plan

> **Superseded (2026-11-22).** The owner removes the manual lists from the routing decision. See `2026-11-22-wan-address-list-writer.md`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Read the manual WAN address-lists on the core router (for example `WAN1-CUSTOMERS`) and keep each listed client in the branch of its WAN node, so shaping follows the routing that the operator set.

**Architecture:** The WAN monitor, which already holds a session to each WAN router, reads the mangle rules, the routes, and the address-lists once a minute. It finds the WAN of each list (list → routing mark → table → distance-1 route → interface → WAN node) and keeps a map from client address to WAN node. An optional list name on a WAN node (schema 8) overrides the discovery. The runner passes the map to placement next to the WAN snapshot. Placement treats a listed client as pinned to the WAN branch.

**Tech Stack:** Rust 2024, `serde`, `tokio`, `mikrotik-rs` (existing), Leptos 0.8.23, Playwright.

**Spec:** `docs/superpowers/specs/2026-11-22-topology-auto-placement-design.md`, section "Manual address-lists (round 3, 2026-11-22)".

## Global Constraints

- Read only: `/ip/firewall/address-list/print` (`list,address,comment,disabled,dynamic`), `/ip/firewall/mangle/print` (`chain,action,new-routing-mark,src-address-list,disabled`), `/ip/route/print` (`dst-address,routing-table,gateway,distance,disabled`). No MikroTik write.
- Lists that start with `via-` belong to the later system writer. This plan ignores them.
- An operator list name on a WAN node overrides the discovered mapping.
- A listed client follows a failover of its WAN only for a hard failure (interface down, or the gateway does not answer), as `check-gateway=ping` does on the router. A quality failure does not move it. (Owner confirmation pending.)
- A conflict in the lists is a warning, never a placement conflict: it must not block publication.
- Registry schema becomes `8`. A schema-7 file migrates with no change.
- Do not deploy. Write docs, comments, and commit bodies in ASD-STE100. Stage explicit paths only.

## Review Focus

1. A client address in `WAN1-CUSTOMERS` and `WAN2-CUSTOMERS`. No constraint for that client, one warning. Test: `an_address_in_two_lists_is_a_warning_and_no_constraint` (Task 2).
2. A listed client whose source is placed under a node that is not below its WAN. No eligible node remains: keep the unconstrained set and warn, do not conflict. Test: `a_route_outside_the_eligible_set_is_a_warning` (Task 3).
3. The address-list read fails (permission or timeout). Keep the last good map for 10 minutes, then drop it; WAN health is not affected. Test: `a_failed_list_read_keeps_the_last_map_for_ten_minutes` (Task 2).
4. Entries for a subnet (`10.0.0.0/24`) or with `disabled=true`. A subnet matches each client address inside it; a disabled entry is ignored. Test: `subnets_match_and_disabled_entries_are_ignored` (Task 2).
5. A PPPoE client reconnects with a new address that is not in the list. It has no constraint now; its saved assignment stays (stability rule). Test: `a_new_address_without_a_list_entry_keeps_the_saved_node` (Task 3).

---

### Task 1: List name on the WAN node

**Files:** `src/topology_manager/catalog.rs`, `model.rs`, `store.rs`, `edits.rs`, `crates/web-contract/src/topology.rs`. Tests: `catalog_tests.rs`, `store_recovery_tests.rs`, `edits_tests.rs`.

**Interfaces:**
- Produces: `CatalogNode.address_list: Option<String>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`), valid only on a WAN node. It overrides the discovery.
- Produces: `TopologyCatalog::wan_lists(&self) -> HashMap<String, String>` (operator list name → WAN node ID; nodes without a value are absent).
- Produces: `Edit::NodeAddressList { node_id: String, list: Option<String> }` / `BuilderEdit::NodeAddressList` (`node.address_list`).

- [ ] **Step 1: Write the tests that fail**

```rust
#[test] fn wan_lists_hold_only_operator_values()              // wan1 None -> absent; wan2 Some("isp-b") -> "isp-b"
#[test] fn a_via_list_name_is_refused()                         // "via-WAN1" belongs to the writer -> error contains "via-"
#[test] fn an_address_list_on_a_non_wan_node_is_refused()     // validate error contains "address list"
#[test] fn two_wan_nodes_cannot_share_one_list()              // same list name -> error
#[test] fn node_address_list_edit_sets_and_clears_the_name()  // {"kind":"node.address_list","node_id":"wan1","list":"via-pldt"}
#[test] fn schema_7_registry_migrates_to_8_without_change()
```

- [ ] **Step 2: Run** `cargo test --locked -p lqosync3 --lib -- wan_lists address_list schema_7` — Expected: compile errors.
- [ ] **Step 3: Implement.** List text: 1–63 characters, no whitespace. Migration arm `7 =>` like the arm `6 =>`.
- [ ] **Step 4: Run** `cargo test --locked -p lqosync3 --lib topology_manager:: && cargo test --locked -p web-contract` — Expected: 0 failed.
- [ ] **Step 5: Commit** `feat(topology): add the WAN address-list name`.

### Task 2: Read the lists in the WAN monitor

**Files:** `src/wan_monitor.rs`, `src/topology_manager/wan_health.rs`. Tests in `src/wan_monitor.rs`.

**Interfaces:**
- Consumes: `wan_lists` (Task 1).
- Produces in `wan_health.rs`: `struct WanRoutes { pub by_address: Vec<(IpAddr, u8, String)>, pub warnings: Vec<String> }` (address, prefix length, WAN node ID) with `WanRoutes::wan_of(&self, ip: IpAddr) -> Option<&str>` (longest prefix wins) and `Default`.
- Produces: `Handle::set_wans(wans, auto_failback, lists: HashMap<String, String>)`; `Handle::routes(&self, now: u64) -> WanRoutes`.
- The list read runs once every 60 s on each router that carries a WAN node, after the probes of that tick.

- Produces: `fn discover_lists(mangle: &[RosRow], routes: &[RosRow], wans: &[WanProbe]) -> HashMap<String, String>` (list → WAN node ID): an enabled `prerouting` `mark-routing` rule gives list → mark; the enabled `0.0.0.0/0` route of that table with the lowest distance gives the interface; the WAN probe with that interface gives the node.

- [ ] **Step 1: Write the tests that fail**

```rust
#[test] fn the_core_router_lists_map_to_their_wans()               // FSE-CORE rows: WAN1-CUSTOMERS -> node of pppoe-out1, WAN2-CUSTOMERS -> pppoe-out2
#[test] fn disabled_rules_and_fallback_routes_do_not_map()          // distance 2 route and disabled mangle are ignored
#[tokio::test] async fn via_lists_are_ignored()                     // via-WAN1 entries give no route
#[tokio::test] async fn listed_addresses_map_to_their_wan_node()     // via-wan1 10.100.0.234 -> wan_of(10.100.0.234) == Some("wan1")
#[tokio::test] async fn subnets_match_and_disabled_entries_are_ignored()  // 10.1.0.0/24 matches 10.1.0.9; disabled=true ignored
#[tokio::test] async fn an_address_in_two_lists_is_a_warning_and_no_constraint()
#[tokio::test] async fn a_list_that_no_wan_names_is_ignored()        // "via-old" -> no route, no warning
#[tokio::test] async fn a_failed_list_read_keeps_the_last_map_for_ten_minutes()
```

- [ ] **Step 2: Run** `cargo test --locked -p lqosync3 --lib wan_monitor` — Expected: FAIL.
- [ ] **Step 3: Implement** with the standard library only (`IpAddr`, prefix compare by bits). Keep the read in `Prober`; store the map and its read time in `Shared`.
- [ ] **Step 4: Run** the same command — Expected: PASS.
- [ ] **Step 5: Commit** `feat(wan): read the WAN address-lists`.

### Task 3: Placement follows the lists

**Files:** `src/topology_manager/observation.rs`, `resolver.rs`, `coordinator.rs`, `src/runner.rs`. Tests: `resolver_tests.rs`.

**Interfaces:**
- Consumes: `WanRoutes` (Task 2).
- Produces: `Reconciliation.routes: WanRoutes`; `Coordinator::set_routes(&mut self, routes: WanRoutes)`.
- Produces: `Placement.route: Option<String>` (WAN node ID from a list) and `PlacementResolution.warnings: Vec<String>`.

- [ ] **Step 1: Write the tests that fail**

```rust
#[test] fn a_listed_client_stays_in_its_wan_branch()                   // auto set {wan1, wan2}; route wan2 -> target wan2 for all listed
#[test] fn a_listed_client_follows_a_hard_failover_only()            // wan2 Unavailable by interface -> moves (home wan2); by quality -> stays
#[test] fn a_route_outside_the_eligible_set_is_a_warning()             // placed group1 under wan1, route wan2 -> group1, warning, no conflict
#[test] fn a_new_address_without_a_list_entry_keeps_the_saved_node()
#[test] fn a_listed_client_moves_once_to_its_wan_and_stays()           // saved wan1, route wan2 -> wan2; next run same
```

- [ ] **Step 2: Run** `cargo test --locked -p lqosync3 --lib topology_manager::resolver` — Expected: FAIL.
- [ ] **Step 3: Implement.** In `prepare`, after the failover filter: the route of the subscriber is the WAN of the first endpoint address that has one. Restrict `full` and `eligible` to targets whose `wan_roots` entry is that WAN node; failover exclusion does not apply to it. A listed client leaves its WAN only when that WAN is unavailable for a hard reason; `WanStatus` gets `hard: bool` (interface down or no reply from every target). If the restriction empties the set, keep the set and add a warning `"<subscriber>: listed on <WAN> but placed outside it"`. The runner sets the routes with the WAN snapshot before each preparation.
- [ ] **Step 4: Run** `cargo test --locked -p lqosync3 --lib topology_manager::` — Expected: 0 failed.
- [ ] **Step 5: Commit** `feat(topology): keep listed clients on their WAN`.

### Task 4: View and canvas

**Files:** `web_view.rs`, `crates/web-contract/src/topology.rs`, `crates/web-ui/src/topology_canvas.rs`, `style.css`, `browser_fixture.rs`, `topology-canvas-check.cjs`.

**Interfaces:**
- Produces: `Node.address_list: Option<String>` (effective name, WAN nodes only) and `Node.listed: Option<u64>` (listed clients); `TopologySummary.route_warnings: Vec<String>` (first 20).
- Consumes: `node.address_list` edit (Task 1).

- [ ] **Step 1: Add the browser cases that fail**
  - WAN node modal: an **Address list** field shows the discovered list (`WAN1-CUSTOMERS`, help text "Found on the router") or the operator value; a change saves one `node.address_list` edit; an empty field sends `list: null`.
  - WAN card shows `12 listed` as text.
  - Preview modal lists the route warnings.
- [ ] **Step 2: Run** `STAGES=browser CHECKS=topology-canvas-check tools/frontend/check-all.sh` — Expected: FAIL.
- [ ] **Step 3: Implement** with the existing `.check`/field patterns, compact, text not color, reserved chip space.
- [ ] **Step 4: Run** `tools/frontend/check-all.sh` — Expected: all PASS (`pulse-check` can SKIP).
- [ ] **Step 5: Commit** `feat(canvas): show WAN address-lists`.

### Task 5: Acceptance and docs

- [ ] **Step 1:** Shadow test `listed_clients_follow_their_wan_and_others_balance` in `src/runner/tests/canvas_shadow.rs` (two WANs, one client listed on `wanB`, the others in count mode; no added or missing row).
- [ ] **Step 2:** Run `cargo fmt --all -- --check`, clippy (native and Wasm), workspace tests, `tools/frontend/check-all.sh`. Expected: all pass.
- [ ] **Step 3:** Update `TOPOLOGY_CANVAS_DESIGN.md` (WAN section), `DEPLOYMENT.md` (the `lqosync` user on FSE-CORE needs a group with `read,api,test`, for example `ti-probe`; the schema-7 backup for rollback), `brain/HANDOFF.md`. STE check.
- [ ] **Step 4: Commit** `docs(wan): record the address-list read`.

## Not in this plan

- Writing the lists (the address-list writer). It needs owner scope approval first.
