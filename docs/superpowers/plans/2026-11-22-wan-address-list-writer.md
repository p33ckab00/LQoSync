# WAN address-list writer: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** LQoSync pushes one `via-<WAN>` address-list for each WAN to the core router, so the WAN that placement chooses for each client is also the WAN that routes its traffic.

**Architecture:** After placement, a pure function turns the placements into the wanted entries (client address → `via-<WAN>` list). A writer task, with its own router user, reads the owned entries on the core router, writes the difference (add before remove), and reports the result. The cycle publishes the shaping pair only after the writer confirms the lists, so shaping and routing stay together. A one-time adoption edit copies the WAN of each client from the old range lists.

**Tech Stack:** Rust 2024, `serde`, `tokio`, `mikrotik-rs` (existing), Leptos 0.8.23, Playwright.

**Spec:** `docs/superpowers/specs/2026-11-22-topology-auto-placement-design.md`, section "WAN address-lists (round 3, 2026-11-22)".

## Global Constraints

- Approved write scope: `/ip/firewall/address-list/add`, `/remove`, `/set` (`address` only), on lists that start with `via-`, on entries whose comment starts with `lqosync:`. No other router write. A test proves the command set.
- Writer credentials are separate: `writer_username`, `writer_password_file` on the core `[[routers]]` entry. The reader user stays read only.
- `[wan_writer] enabled = false` by default. With it off, nothing is written and publication does not wait.
- List name of a WAN node: `via-<WAN node name>`, or the node value (must start with `via-`). Registry schema 8.
- Entry: one IPv4 address for each subscriber endpoint (`/32`), comment `lqosync: <circuit name>`. A flat placement has no entry.
- A failover placement uses its current WAN, not its home, so routing follows the failover.
- Safety: when one write would remove more than `max_removed` (default 50) entries, or more than `max_removed_percent` (default 20) of the owned entries, the writer holds and reports. The `[safety]` style applies.
- When the write fails or holds, the cycle keeps the previous shaping pair and retries next cycle.
- Do not deploy. Docs for operators in ASD-STE100. Stage explicit paths only.

## Review Focus

1. An entry that a person added by hand to `via-WAN1` without the `lqosync:` comment. The writer never changes or removes it. Test: `foreign_entries_are_never_touched` (Task 4).
2. A client moves from WAN1 to WAN2. The new entry exists before the old entry goes: the client never has no list. Test: `a_move_adds_before_it_removes` (Task 4).
3. The core router is not reachable for 10 minutes. The shaping pair stays at the last confirmed version, the status says why, and the next cycle writes the full difference. Test: `a_failed_write_holds_publication_and_retries` (Task 5).
4. A collection outage makes most subscribers disappear for one cycle. The safety cap holds the mass removal. Test: `a_mass_removal_holds` (Task 4).
5. A PPPoE client reconnects with a new address. The writer changes the address of its entry (`set address`), not remove plus add. Test: `a_new_address_changes_the_entry` (Task 4).

---

### Task 1: Config, list names, and writer credentials

**Files:** `src/config.rs`, `config/defaults.toml`, `src/cli.rs` (check), `src/topology_manager/{catalog,model,store,edits}.rs`, `crates/web-contract/src/topology.rs`. Tests in the same modules.

**Interfaces:**
- Produces: `RouterCfg.writer_username: String` and `RouterCfg.writer_password_file: String` (both empty by default; both set or both empty).
- Produces: `Config.wan_writer: WanWriter { enabled: bool = false, max_removed: usize = 50, max_removed_percent: f64 = 20.0 }`.
- Produces: `CatalogNode.address_list: Option<String>` (WAN nodes only, must start with `via-`, 5–63 characters, no whitespace); `TopologyCatalog::wan_list(&self, node: usize) -> Option<String>` (node value, else `via-<name>` with spaces as `-`).
- Produces: edit `node.address_list { node_id, list: Option<String> }`.

- [ ] **Step 1: Tests that fail:** `writer_credentials_are_both_or_neither`, `wan_writer_defaults_are_off_and_safe`, `wan_list_uses_the_node_value_or_via_name` (`"WAN 1"` → `via-WAN-1`), `a_list_name_without_via_is_refused`, `schema_7_registry_migrates_to_8_without_change`, `check_reports_a_missing_writer_password_file`.
- [ ] **Step 2: Run** `cargo test --locked -p lqosync3 --lib -- writer_credentials wan_writer wan_list list_name schema_7 check_reports` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test --locked -p lqosync3 --lib && cargo test --locked -p web-contract` — PASS.
- [ ] **Step 5: Commit** `feat(wan): add writer config and list names`.

### Task 2: Wanted entries from placement

**Files:** new `src/topology_manager/wan_lists.rs`. Tests in that file.

**Interfaces:**
- Produces: `struct ListEntry { list: String, address: Ipv4Addr, comment: String }` and `fn wanted(pair: &PreparedPair, catalog: &TopologyCatalog) -> anyhow::Result<BTreeMap<Ipv4Addr, ListEntry>>`.
- WAN of a placement: the WAN root above `target_id`. Comment: `lqosync: <circuit name>`, at most 64 characters.

- [ ] **Step 1: Tests that fail:** `each_client_address_gets_the_list_of_its_wan`, `a_flat_placement_has_no_entry`, `a_failover_uses_the_current_wan`, `ipv6_addresses_have_no_entry`, `two_wans_for_one_address_is_an_error`.
- [ ] **Step 2: Run** `cargo test --locked -p lqosync3 --lib wan_lists` — FAIL.
- [ ] **Step 3: Implement.** Expose `target_id` per row through `PreparedPair` (the CSV row already has `parent_node_id`; map it to its WAN root with the catalog).
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(wan): derive the wanted address-lists`.

### Task 3: The write session

**Files:** `src/routeros.rs`, `src/mikrotik.rs`, `tests/mock_router.rs`.

**Interfaces:**
- Produces: `trait ListWriter: Send { fn add(&mut self, e: &ListEntry) -> Future<Result<String, ReadError>>; fn remove(&mut self, id: &str) -> Future<Result<(), ReadError>>; fn set_address(&mut self, id: &str, address: Ipv4Addr) -> Future<Result<(), ReadError>>; }` (the `add` result is the new `.id`).
- Produces: `MikrotikConnector::connect_writer(&self, router: &RouterCfg, access: &RouterAccess) -> Future<Result<MikrotikSession, ReadError>>` with the writer credentials.

- [ ] **Step 1: Tests that fail (mock router):** `the_writer_sends_only_address_list_commands` (the log holds only `/login`, `/ip/firewall/address-list/print|add|remove|set`), `add_returns_the_new_id`, `the_writer_logs_in_with_the_writer_user`.
- [ ] **Step 2: Run** `cargo test --locked -p lqosync3 --test mock_router writer` — FAIL.
- [ ] **Step 3: Implement.** `set` sends `=.id=` and `=address=` only.
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(wan): add the address-list write session`.

### Task 4: Diff and apply

**Files:** new `src/wan_writer.rs`. Tests in that file (fake session).

**Interfaces:**
- Produces: `struct Owned { id: String, list: String, address: Ipv4Addr, comment: String }`; `fn read_owned(rows: &[RosRow]) -> Vec<Owned>` (list starts with `via-`, comment starts with `lqosync:`, not dynamic).
- Produces: `struct Plan { add: Vec<ListEntry>, set: Vec<(String, Ipv4Addr)>, remove: Vec<String> }`; `fn plan(owned: &[Owned], wanted: &BTreeMap<Ipv4Addr, ListEntry>) -> Plan`. Same client comment in the same list with a new address → `set`; other changes → add then remove.
- Produces: `async fn apply<W: ListWriter + RouterSession>(w: &mut W, wanted, cfg: &WanWriter) -> Result<WriteReport, WriteError>`; `WriteError::{Held(String), Router(String)}`; `WriteReport { added, changed, removed, owned_after }`.

- [ ] **Step 1: Tests that fail:** `foreign_entries_are_never_touched`, `a_move_adds_before_it_removes`, `a_new_address_changes_the_entry`, `an_unchanged_world_writes_nothing`, `a_mass_removal_holds`, `duplicate_owned_entries_are_reduced_to_one`.
- [ ] **Step 2: Run** `cargo test --locked -p lqosync3 --lib wan_writer` — FAIL.
- [ ] **Step 3: Implement.** Re-read the owned entries after the write and compare with `wanted`; a difference is `Router("lists differ after the write")`.
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(wan): write the difference of the lists`.

### Task 5: Cycle integration

**Files:** `src/runner.rs`, `src/runner/topology_cycle.rs`, `src/main.rs`, `src/system_health.rs` (domain component `wan_lists`). Tests: runner tests with a fake writer.

**Interfaces:**
- Produces: `Runner.wan_writer: Option<WanWriterHandle>`; before `publish_topology`, the cycle computes `wanted`, calls `apply`, and publishes only on success. With `enabled = false`, the step is skipped.
- Status: `status.json` and the topology view get `wan_lists: { state, at, added, changed, removed, owned, error }`.

- [ ] **Step 1: Tests that fail:** `a_failed_write_holds_publication_and_retries`, `a_disabled_writer_publishes_without_waiting`, `a_successful_write_then_publishes_the_pair`, `a_held_write_reports_the_reason`.
- [ ] **Step 2: Run** `cargo test --locked -p lqosync3 --lib -- failed_write disabled_writer successful_write held_write` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** workspace tests — PASS.
- [ ] **Step 5: Commit** `feat(wan): write the lists before each publication`.

### Task 6: One-time adoption of the old range lists

**Files:** `src/wan_monitor.rs` (read mangle, routes, lists), `src/topology_manager/edits.rs`, `src/runner.rs`. Tests: `wan_monitor.rs`, runner tests.

**Interfaces:**
- Produces: `fn discover_lists(mangle: &[RosRow], routes: &[RosRow], wans: &[WanProbe]) -> HashMap<String, String>` (list → WAN node; enabled `prerouting` `mark-routing` rule → table → enabled default route with the lowest distance → interface → WAN node). Lists that start with `via-` are skipped.
- Produces: edit `wan.adopt` (no fields). For each active subscriber without a pin whose address is in an old list range, set its assignment to the eligible node below that WAN with the lowest load; answer `{adopted, skipped}`. It is a reviewed edit (proposal, then confirm), not automatic.

- [ ] **Step 1: Tests that fail:** `the_core_router_lists_map_to_their_wans` (the FSE-CORE rows of 2026-11-22), `ranges_and_subnets_match_client_addresses` (`10.0.100.211-10.0.100.221`, `10.15.0.0/24`), `adoption_keeps_the_current_wan_of_each_client`, `adoption_skips_pins_and_unlisted_clients`.
- [ ] **Step 2–4:** run, implement, run.
- [ ] **Step 5: Commit** `feat(wan): adopt the WAN of each client once`.

### Task 7: View and canvas

**Files:** `web_view.rs`, `crates/web-contract/src/topology.rs`, `crates/web-ui/src/topology_canvas.rs`, `style.css`, `browser_fixture.rs`, `topology-canvas-check.cjs`.

- [ ] **Step 1: Browser cases that fail:** WAN node modal **Address list** field (`via-WAN-North`, change → one `node.address_list` edit); WAN card chip `24 routed`; Settings **Routing lists** block (state, last write, counts, error, held reason); **Adopt current routing** button opens a review with the counts and saves one `wan.adopt` edit only after **Confirm**; 375 px and 768 px, light and dark, focus return.
- [ ] **Step 2–4:** run `STAGES=browser CHECKS=topology-canvas-check tools/frontend/check-all.sh`, implement (Vercel and Google guidance, house components, text not color, no layout shift), run `tools/frontend/check-all.sh`.
- [ ] **Step 5: Commit** `feat(canvas): show the WAN routing lists`.

### Task 8: Operator documentation and acceptance

**Files:** new `docs/operations/WAN_ROUTING_LISTS.md`, `docs/frontend/DEPLOYMENT.md`, `docs/frontend/TOPOLOGY_CANVAS_DESIGN.md`, `brain/HANDOFF.md`, `src/runner/tests/canvas_shadow.rs`.

- [ ] **Step 1: Shadow test** `routing_lists_follow_placement_and_failover` (two WANs, fake writer; a failover moves the entry; no foreign entry changes).
- [ ] **Step 2: Write `WAN_ROUTING_LISTS.md`** in ASD-STE100, with exact RouterOS commands and a check after each step:
  1. Router users: give `lqosync` the `test` policy (`/user group set lqosync-read policy=read,api,test`); make group `lqosync-write` (`read,write,api`) and user `lqosync-writer` with an address limit to the LQoSync host.
  2. LQoSync config: `writer_username`, `writer_password_file`, `[wan_writer] enabled = true`.
  3. WAN names and list names in the canvas.
  4. Adoption: **Adopt current routing**, review, confirm.
  5. Mangle: add the `via-WAN1` and `via-WAN2` rules above rules 0 and 1 (`/ip firewall mangle add ... place-before=0`), then check the packet counters.
  6. Fallback: keep `WAN1-CUSTOMERS` and `WAN2-CUSTOMERS` below as a safety net; remove them only when the counters show no traffic.
  7. Rollback: disable the `via-*` rules; the old rules route as before. Set `enabled = false`.
- [ ] **Step 3:** `DEPLOYMENT.md` (writer section, schema-7 backup), design doc state, handoff. STE check.
- [ ] **Step 4:** fmt, clippy (native and Wasm), workspace tests, `tools/frontend/check-all.sh` — all pass.
- [ ] **Step 5: Commit** `docs(wan): add the routing list guide`.

## Not in this plan

- IPv6 routing lists.
- Any change of mangle rules or routes by the system.
