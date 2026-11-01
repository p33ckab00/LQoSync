# lqosync3 DHCP Client Overrides Implementation Plan (Plan 4c)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A speed override for one DHCP client by MAC address (`[[routers.dhcp.clients]]`), the `shape_unassigned` key, and a DHCP view in the Settings tab that lists the servers and leases from `snapshot.json`.

**Architecture:** `sources.rs` gets one shared lease list (`dhcp_leases`) that gives each lease its result (shaped with a rate and its source, or skipped with a reason). `collect_dhcp` builds the circuits from it, and the TUI shows it. The `per_site` node rate uses a new `unit` field (the server rate), not the rate of the first circuit.

**Tech Stack:** Rust 1.89, the existing crates.

**Spec:** `docs/superpowers/specs/2026-11-01-lqosync3-dhcp-clients-design.md`. Settings editor: `docs/superpowers/plans/2026-11-01-lqosync3-tui-4b.md`.

## Global Constraints

- All defaults in `config/defaults.toml`: `[router_defaults.dhcp]` gets `shape_unassigned = false` and `clients = []`.
- The production shaper operates LibreQoS 2.2 in `apply` mode with no override and no lease without a server. Its output must not change: `tests/scenarios.rs` passes without `LQOSYNC3_BLESS`, and all existing cycle, sources, and topology tests pass unchanged. A test whose expected value changes is a finding, not an update.
- The override replaces only the rate. Each skip rule stays (spec section 5).
- No rate or group code in the TUI: it calls `sources::dhcp_leases`.
- No test connects to a router or writes outside a temporary folder.
- Docs and comments in ASD-STE100.

## Review Focus

1. A MAC in another form (`aa-bb-cc-dd-ee-ff`, lower case) in the lease or in the TOML file: the override still applies to that client, and only to it.
2. The override client has the alphabetically first circuit name of a `per_site` server: the server node rate does not change.
3. An override on a skipped lease (excluded IP, not bound, disabled or excluded server, no server): no circuit.
4. `snapshot.json` cannot be read (not root, no file): the DHCP view shows only the TOML servers and says that the leases are not known.
5. A router that is not online in the snapshot: its leases are not listed, and its overrides show "no active lease" only if the snapshot has the router online.

---

### Task 1: Config

**Files:** `src/config.rs`, `config/defaults.toml`, `docs/CONFIGURATION.md`, `src/tui/settings/meta.rs`.

**Produces:**
- `pub struct DhcpClient { pub mac: String, pub download_mbps: f64, pub upload_mbps: f64, #[serde(default)] pub name: Option<String> }` (`deny_unknown_fields`).
- `DhcpCfg` gets `pub shape_unassigned: bool` and `pub clients: Vec<DhcpClient>`.
- `pub fn normalize_mac(s: &str) -> Option<String>`: 6 groups of 2 hex digits with `:` or `-`, any case, gives `AA:BB:CC:DD:EE:FF`; else `None`.
- Validation for each router (house style `config: router "<name>": dhcp.clients: ...`): each `mac` gives `Some` from `normalize_mac`; no two normalized MACs are the same; both rates finite and `>= bandwidth_defaults.max_floor_mbps`. `router_defaults.dhcp.clients` is not allowed (it would apply to each router): add it to the blocked keys of the `router_defaults` merge (config.rs, beside `name`, `address`).
- Ruling on the MAC form (spec section 4): the loader validates the MAC and keeps the text as written; each compare uses `normalize_mac`; the TUI writes the normalized form.
- Meta: `routers.*.dhcp.clients` and `router_defaults.dhcp.clients` are not keys (as `servers`): extend the skip in the walk (meta.rs, `router_defaults.dhcp.servers`). Extra keys `routers.*.dhcp.clients.*.mac` (`Text`), `.download_mbps`, `.upload_mbps` (`Float`, in `FLOATS`), `.name` (`Text`, optional).
- `docs/CONFIGURATION.md`: the example gets `shape_unassigned` only if the example has `[router_defaults.dhcp]` (`tests/config_doc.rs` checks the example values against the defaults).

- [ ] Tests (`config.rs`): `dhcp_client_mac_forms_load`, `a_bad_or_duplicate_mac_is_an_error` (also `aa:bb` and the same MAC in two cases), `a_client_rate_below_the_floor_is_an_error`, `shape_unassigned_defaults_to_false`, `router_defaults_clients_is_an_error`. `meta.rs`: extend the filter of `each_default_key_has_meta` (`router_defaults.dhcp.clients`) and add a `routers.*.dhcp.clients.*` arm to the `file()` helper (a client entry with a MAC and both rates), so `number_kinds_match_the_config_types` tests the new float keys.
- [ ] Commit: `feat(config): add DHCP client overrides and shape_unassigned`.

### Task 2: The shared lease list and the cycle

**Files:** `src/sources.rs`, `src/cycle.rs`, `src/topology.rs`.

**Produces:**
- `pub enum RateSource { Override, ServerComment, LeaseComment, Settings }`.
- `pub enum LeaseResult { Shaped { rate: Rate, source: RateSource, group: GroupKey, unit: Rate }, Skipped(String) }`.
- `pub struct LeaseView { pub server: String, pub mac: String, pub ip: String, pub host: String, pub comment: String, pub has_override: bool, pub result: LeaseResult }`.
- `fn dhcp_leases(cfg: &Config, router: &RouterCfg, snap: &RouterSnapshot, speed: &SpeedParser, ex: &Excluder, report: &mut Vec<String>) -> Vec<LeaseView>` (private: `Excluder` is private, and a pub fn that takes it fails `clippy -D warnings`). One entry for each lease of `snap.dhcp_lease` that has a MAC and an IP. Order: the leases of each server in the server order of today (`names`, the same per-server loop), then the other leases in reply order. Without an override, the report lines are the same, in the same order, as today.
  - Skip reasons (text), checked in this order: `no server (dhcp.shape_unassigned = false)`, `disabled server`, `excluded server`, `unknown server` (with `discover = true`) or `server not listed (dhcp.discover = false)`, `excluded IP`, `not bound`, `no rate` (only without an override). A skipped lease never calls `comment_rate`.
  - A lease has no server only if its `server` field is `""` or `all` and that name is not in `names` (a TOML server named `all` keeps its leases, as today). With `shape_unassigned = true`: group `GroupKey::DhcpSite("unassigned")`, rate: the lease comment (through `comment_rate`) else `bd.dhcp_default_per_client_mbps` for both directions.
  - Override: the router `dhcp.clients` entry with `normalize_mac(lease mac) == normalize_mac(client.mac)`. It replaces the rate only, after the skip checks, and it saves a lease that has no rate. An override lease does not call `comment_rate` (its comment lines go away).
  - `LeaseView.mac` is the raw lease MAC (`Collected.mac` stays raw).
  - `unit`: for `DhcpSite`, the server-level rate: the first server-level source of the server that gives a rate, through `speed.comment` (the existing `server_rate` closure), not `comment_rate`; if there is none, `server.settings_rate(bd)`; for `unassigned`, `bd.dhcp_default_per_client_mbps`. For `DhcpPlan`, the rate of the lease. So all members of a `DhcpSite` group have the same unit.
- A pub wrapper for the TUI: `pub fn router_leases(cfg: &Config, router: &RouterCfg, snap: &RouterSnapshot) -> anyhow::Result<Vec<LeaseView>>` (makes the `SpeedParser` and the `Excluder`; drops the report).
- `Collected` gets `pub unit: Rate` (PPP and Hotspot: `unit = base`). `collect_dhcp` builds a `Collected` for each `Shaped` lease, with the same name, device name, MAC, IPv4, and comment rules as today.
- One warning line for each router with skipped no-server leases and `shape_unassigned = false`: `router <name>: <n> DHCP leases have no server and are not shaped (routers.dhcp.shape_unassigned = false)`.
- `cycle.rs`: `members` holds the unit too (it is filled at one place, where `Collected` is in scope), and `GroupTotal.unit` is the unit of the first member. `topology.rs`: only the doc comment of `GroupTotal.unit` changes ("the group rate of one circuit: the server rate for DHCP sites"); `count_rate` stays.

- [ ] Tests (`sources.rs` and `cycle.rs`, with the fixtures of the existing tests):
  - `an_override_changes_only_that_client` (per_site: one override client, two server-comment clients; MAC in `aa-bb-...` form in the TOML and upper case in the lease).
  - `an_override_in_per_plan_moves_to_its_plan_node`.
  - `the_per_site_node_rate_ignores_the_override` (the override client has the first name; the node rate is the count × the server rate × the factors).
  - `an_override_for_a_missing_mac_has_no_effect`.
  - `an_override_on_a_skipped_lease_gives_no_circuit` (excluded IP; not bound with `require_bound`).
  - `no_server_leases_are_skipped_with_one_warning` and `shape_unassigned_puts_them_in_the_unassigned_node`.
  - `dhcp_leases_gives_each_skip_reason` (disabled, excluded, not listed, excluded IP, not bound, no server, no rate).
  - `an_override_gives_the_same_rate_in_v2_pair_v3_pair_and_v3_network` (cycle level: `assemble` with `v3` false and true, then `render_csv(.., Columns::V2 / V3)` and `render_libreqos_network`). Node-rate tests use `Config::from_toml_str` as `same_name_twice_in_one_router_keeps_first_and_warns` does; the sources tests use `cfg_text` and `DISCOVER`.
  - `zero_speed_comments_give_a_report_line` keeps its exact count of 3.
  - All existing tests and `tests/scenarios.rs` pass unchanged.
- [ ] Commit: `feat(dhcp): add client overrides and the shared lease list`.

### Task 3: The DHCP view of the Settings tab

**Files:** `src/tui/data.rs`, `src/tui/app.rs`, `src/tui/settings/draft.rs`, `src/tui/settings/view.rs`.

**Produces:**
- `Data.snapshot: Option<Arc<String>>`: the loader reads `<state_dir>/snapshot.json` when its time or size changes. A read error (not root) gives `None` and no error line (the file is `0600`). The view parses it with `preview::snapshot_routers`.
- `App::set_data` calls `settings.set_snapshot(data.snapshot.clone())` when the `Arc` changes.
- `Draft::set_client(&mut self, router: usize, mac: &str, down: f64, up: f64, name: Option<&str>) -> anyhow::Result<()>` (update the entry with the same normalized MAC, else add one; the MAC is written normalized) and `Draft::remove_client(&mut self, router: usize, mac: &str) -> anyhow::Result<bool>`.
- `SettingsView`:
  - Keeps the lease lists: after each change and each new snapshot, for each router of the draft config (`validate::parse`; on an error keep the last lists) that is `online` in the snapshot, `sources::router_leases`.
  - `Group` gets `server_name: Option<String>` (the server of a server group, with or without a TOML entry) and its rows become `Vec<Row>` with `enum Row { Key(KeyPath, &'static KeyMeta), Lease(LeaseView), Client(usize) }`. `self.key`, `move_sel`, `current()`, `select()`, and the keys panel use the rows. `r`, `Enter`, `+`, `-` branch on the row and on `server_name`: `-` on a server group removes the server entry (if any), never the router; `+` on a server or dhcp group adds a server.
  - DHCP server groups of a router: the TOML servers and each server name of its leases, in this order, with the title `router <r> dhcp server <name> (<n> leases)`; a group `router <r> dhcp no server (<n> leases)` if there are such leases. A router with `dhcp.enabled = false` gets no lease rows and the line `DHCP is off for this router`.
  - A server with no TOML entry shows the server keys with the values of `DhcpCfg::server(name)` of the parsed draft. The edit calls `add_server` and sets the key at the commit of the field (one undo step; `Esc` makes nothing). With `discover = false`, the help says that a new entry adds the server to the shaped servers.
  - Below the keys of a server group: one line for each lease: `host (or MAC)  IP  MAC  <down>/<up> Mbps  <source>` and `override`; a skipped lease shows `not shaped: <reason>` in grey.
  - `Enter` on a shaped lease: a form (as the `Input::AddRouter` step form) with download, upload (prefilled with the override or the current rate), and name; `Enter` on the last field calls `set_client`. `Enter` on a skipped lease: message `not shaped: <reason>; an override has no effect`. `r` on a lease with an override: `remove_client`.
  - A group `router <r> dhcp clients`: each override entry with its lease state (`active lease`, or `no active lease` if the router is online in the snapshot and has no lease with that MAC). `r` removes the entry.
  - With no snapshot: the server groups come from the TOML file only, and the keys panel says `leases not known: snapshot.json cannot be read (start the TUI with sudo)`.

- [ ] Tests (`view.rs`, `draft.rs`, `data.rs`):
  - `set_client_adds_and_updates_by_mac`, `remove_client_by_mac` (draft).
  - `discovered_servers_and_leases_are_listed` (a snapshot text with two servers, one without a TOML entry).
  - `enter_on_a_lease_makes_an_override` (the draft text has the normalized MAC and the rates; `Config::from_toml_str` loads it).
  - `a_skipped_lease_cannot_get_an_override`.
  - `an_edit_of_a_discovered_server_makes_its_entry`.
  - `an_override_without_a_lease_shows_no_active_lease`.
  - `no_snapshot_shows_only_the_toml_servers`.
  - `minus_on_a_server_group_never_removes_the_router`.
  - `r_on_a_lease_removes_its_override`.
  - `run_preview_shows_one_circuit_line_for_an_override` (temp files and a snapshot text, as the preview tests).
  - The view tests use their own config text with `[routers.dhcp] enabled = true` (the `FILE` fixture has DHCP off).
  - `loader_reads_the_snapshot_text` (data).
- [ ] Commit: `feat(tui): add the DHCP servers and leases to the settings`.

### Task 4: Docs, review, graph

- [ ] `docs/CONFIGURATION.md`: "DHCP servers" gets the client override and `shape_unassigned`. `docs/OPERATIONS.md`: "Change the configuration in the TUI" gets the lease steps. `brain/HANDOFF.md`.
- [ ] Whole-branch review by a fresh reviewer, then graphify update, then commit and push.

## Review rulings (fact-check of 2026-11-01)

- Findings 1 to 10, 12, 13, 14, 15, 18, 19, 22: fixed in the tasks above.
- Finding 11: the unit falls back to the settings rate of the server, so all members of a site group have one unit.
- Finding 16: the no-server warning changes the status.json warning count and the Telegram warning list; no output file changes. Accepted.
- Finding 17: a bound lease of a `server = all` static lease has its real server only in `active-server`, which this plan does not read. Such leases count as "no server". Accepted (the live router has none).
- Finding 20: two leases with the same host give one circuit name; the cycle keeps the first. An override on the second MAC then has no effect. Accepted as a known limit (the cycle already warns about the duplicate name).
- Finding 21: duplicate server names do not occur in RouterOS. Accepted.
