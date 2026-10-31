# lqosync3 Core Implementation Plan (Plan 1 of 4)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the pure core library of `lqosync3`: config, speed rules, state, cycle assembly, topology that follows the LibreQoS v3 node rules, and the pair render, with scenario tests. `updatecsv.py` is a guide only (ADR-0007).

**Architecture:** One pure function `assemble()` turns config, state, static rows, and per-router RouterOS snapshots into CSV rows, nodes, a new state, and a report. It does no I/O, and it does not read the clock or a random source. Renderers turn rows and nodes into `ShapedDevices.csv` and `network.json`. Scenario tests pin the output, and an optional dev check runs the v3 compiler on it.

**Tech Stack:** Rust 1.99 toolchain (crate `rust-version = "1.88"`, edition 2024), `serde`, `serde_json`, `toml`, `csv`, `regex`, `anyhow`. No Python in the crate or its tests.

**Spec:** `docs/superpowers/specs/2026-10-31-lqosync3-core-design.md` (primary), `docs/CONFIGURATION.md`, `docs/ARCHITECTURE.md`, `brain/30-knowledge/updatecsv-py-dissection.md` (domain guide), `brain/30-knowledge/v3-node-handling.md`, ADR-0005, ADR-0006, ADR-0007.

## Global Constraints

- All paths are relative to the repository root `/home/pi/LQoSync3.0/`. The crate root is the repository root. The crate name is `lqosync3`.
- Git: branch `lqosync3`, repository root `/home/pi/LQoSync3.0`. Commit locally. Do not push.
- `export PATH=$HOME/.cargo/bin:$PATH` before each `cargo` command.
- Runtime dependencies: only `serde` (feature `derive`), `serde_json`, `toml`, `csv`, `regex`, `anyhow`. No `serde_json` feature `preserve_order`. No other crate in Plan 1.
- The core (`src/`) does no file, network, clock, or random access. Tests may read fixture files.
- Every task ends with `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` green.
- Fixtures are synthetic. No live subscriber names, MAC addresses, or IP addresses in the repository.
- Code comments follow ASD-STE100: short sentences, active voice.
- Rates: `f64` Mbps. Times: `u64` Unix seconds.
- LibreQoS rules (2.2 and v3): circuit max >= 0.2, min >= 0.1, node names unique at all levels, node `id` charset `[A-Za-z0-9._-]`, length <= 64.

## Review Focus

1. A RouterOS row without a field (for example, a lease with no `host-name` key) must act as an empty string, not a panic. Test in Task 4.
2. Non-ASCII names (`Niño`) must pass through raw UTF-8 in CSV and JSON and sort by bytes. Test in Task 7.
3. A `ShapedDevices.csv` saved with a UTF-8 BOM or CRLF line ends (edited in a spreadsheet) must seed correctly. Test in Task 3.
4. A router that is `Online` with all tables empty must remove its rows, and still make the `ppp` node at `0.128`. Test in Task 5 (`ppp_node_exists_with_zero_users`) and Task 6 (`empty_online_router_removes_its_rows`).
5. A config with zero `[[routers]]`, or two routers with the same name, must fail at load with the problem named. Test in Task 1.

## File Structure

| File | Responsibility |
|------|----------------|
| `Cargo.toml` | Crate metadata and the six dependencies |
| `src/lib.rs` | Module declarations only |
| `src/config.rs` | TOML types, defaults, validation, resolved per-router values |
| `src/speed.rs` | `Rate`, `SpeedParser`, `round3`, `fmt_rate`, clamp, per-circuit rates |
| `src/model.rs` | `CsvRow`, `Source`, `RouterSnapshot`, state types |
| `src/state.rs` | CSV read, static split, seed, state JSON |
| `src/sources.rs` | `collect_router()`: RouterOS rows to `Collected` circuits |
| `src/topology.rs` | Parent names, group nodes, root nodes, nested JSON |
| `src/cycle.rs` | `assemble()`, removal rules, duplicate IP, `Report` |
| `src/validate.rs` | Stop-the-cycle checks |
| `src/writer/mod.rs`, `src/writer/pair.rs` | CSV and `network.json` text |
| `tests/config_doc.rs` | The example in `docs/CONFIGURATION.md` parses |
| `tests/scenarios.rs` | Runs each scenario and compares the output |
| `tests/scenarios/<case>/` | Synthetic scenarios with expected output |
| `tools/v3sim/check.sh` | Dev only: v3 compiler check of a pair |

Unit tests live in `#[cfg(test)] mod tests` inside each `src/` file.

---

### Task 1: Crate scaffold and config

**Files:**
- Create: `Cargo.toml`, `src/lib.rs`, `src/config.rs`, `tests/config_doc.rs`
- Modify: `.gitignore` (already has `/target/`)

**Interfaces:**
- Produces (all `pub`, `#[derive(Debug, Clone, Deserialize)]`, `#[serde(deny_unknown_fields, default)]` on each section struct):
  - `Config { runtime: Runtime, router_access: RouterAccess, safety: Safety, libreqos: Libreqos, network: Network, nodes: Nodes, circuits: CircuitNames, speed: Speed, bandwidth_defaults: BandwidthDefaults, backstop_caps: BackstopCaps, telegram: Telegram, routers: Vec<RouterCfg> }`
  - `impl Config { pub fn from_toml_str(s: &str) -> anyhow::Result<Config> }`: parse, then `validate()`.
  - Field names and defaults: exactly the keys and values in the `docs/CONFIGURATION.md` example (including `[safety]`), except the defaults below. `RouterCfg` has no default for `name`, `address`, `username`, `password_file`.
    - `RouterCfg.port = 8729`, `tls = true`. `pppoe.enabled = false`, `pppoe.per_plan_node = false`. `hotspot.enabled = false`, `include_mac = true`. `dhcp.enabled = false`, `require_bound = false`, `servers = []`.
    - Hotspot rates and factors: `Option<f64>`. DHCP server `default_plan_down_mbps`, `default_plan_up_mbps`, `download_factor`, `upload_factor`: `Option<f64>`. DHCP server `speed_sources: Option<Vec<DhcpSource>>`.
  - Enums, `#[serde(rename_all = "snake_case")]`: `RuntimeMode { Shadow, Apply }`, `Target { Auto, V2Pair, V3Pair, V3Network }`, `NetworkMode { GroupedRouterRoot, FlatRouterRoot, FlatGlobalRoot, NoParent, RootPromotion, FlatPromotion }`, `OfflineAction { Remove, Keep }`, `RateLimitOrder { DownloadUpload, RouterosRxTx }`, `UnitlessUnit { Mbps, Bps }`, `DhcpMode { PerSite, PerPlan }`, `PppSource { SecretComment, ActiveComment, ProfileComment, ProfileRateLimit, ProfileName, Default }`, `DhcpSource { ServerComment, LeaseComment, Settings }`. `LogLevel { Error, Warn, Info, Debug }`.
  - `NodeTemplate { name: String, r#type: Option<String> }` with `#[serde(rename = "type")]`. `Nodes.client` is `ClientTemplate { r#type: String }` (default `"client"`).
  - `impl NodeTemplate { pub fn type_for(&self, v2_default: &str, v3: bool) -> String }`: the explicit `type`, else `"site"` if `v3`, else `v2_default` (ADR-0006).
  - `impl Config { pub fn targets_v3(&self) -> Option<bool> }`: `Some(true)` for `V3Pair` and `V3Network`, `Some(false)` for `V2Pair`, `None` for `Auto`.
  - `BackstopCap { enabled: bool, download_mbps: f64, upload_mbps: f64, multiplier: f64 }`, `impl BackstopCap { pub fn limit(&self) -> Option<crate::speed::Rate> }`: `None` if not enabled, else `download_mbps × multiplier`, `upload_mbps × multiplier`.
  - `impl HotspotCfg { pub fn per_client(&self, bd: &BandwidthDefaults) -> Rate; pub fn factors(&self, bd: &BandwidthDefaults) -> (f64, f64) }`. The fallbacks are `hotspot_default_per_client_mbps`, `hotspot_download_factor`, `hotspot_upload_factor`.
  - `impl DhcpServer { pub fn sources(&self) -> Vec<DhcpSource>; pub fn settings_rate(&self, bd: &BandwidthDefaults) -> Rate; pub fn factors(&self, bd: &BandwidthDefaults) -> (f64, f64) }`. The `sources()` default is `[ServerComment, Settings]` for `PerSite` and `[LeaseComment]` for `PerPlan`. The fallbacks are `dhcp_default_per_client_mbps`, `dhcp_download_factor`, `dhcp_upload_factor`.
  - `Safety { enabled: bool, max_removed_percent: f64, max_removed_count: usize }`. Validation: `0 < max_removed_percent <= 100`.
  - `Rate` comes from Task 2. Put `pub struct Rate { pub down: f64, pub up: f64 }` (`Debug, Clone, Copy, PartialEq`) in `src/speed.rs` now, with no functions.

- [ ] **Step 1: Write `Cargo.toml` and `src/lib.rs`**

`[package] name = "lqosync3"`, `version = "0.1.0"`, `edition = "2024"`, `rust-version = "1.88"`. Dependencies: `anyhow = "1"`, `csv = "1"`, `regex = "1"`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `toml = "0.9"`. `src/lib.rs` declares `pub mod config; pub mod speed;` (later tasks add their modules).

- [ ] **Step 2: Write the failing tests**

In `src/config.rs` tests, with this helper: `const MIN: &str = "[[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n";`

```rust
#[test] fn minimal_config_gets_documented_defaults() {
    let c = Config::from_toml_str(MIN).unwrap();
    assert_eq!(c.runtime.scan_interval_active, 25);
    assert_eq!(c.runtime.scan_interval_idle, 120);
    assert_eq!(c.router_access.offline_after, 300);
    assert_eq!(c.router_access.offline_action, OfflineAction::Remove);
    assert_eq!(c.runtime.static_comment, "static");
    assert_eq!(c.network.root_name, "LibreQoS-Network");
    assert_eq!(c.nodes.dhcp_site.name, "DHCP-{server}-{router}");
    assert_eq!(c.speed.pppoe_sources.len(), 6);
    assert_eq!(c.routers[0].port, 8729);
    assert_eq!(c.bandwidth_defaults.max_floor_mbps, 0.2);
    assert_eq!((c.safety.enabled, c.safety.max_removed_percent, c.safety.max_removed_count), (true, 20.0, 50));
    assert_eq!(c.speed.rate_limit_order, RateLimitOrder::RouterosRxTx);
}
#[test] fn unknown_key_is_rejected_with_its_name() {
    let e = Config::from_toml_str(&format!("[runtime]\nscan_intervall=5\n{MIN}")).unwrap_err();
    assert!(format!("{e:#}").contains("scan_intervall"));
}
#[test] fn out_of_range_values_are_rejected() {
    for bad in ["[bandwidth_defaults]\nmin_rate_percentage=1.5\n",
                "[bandwidth_defaults]\nsafe_min_mbps=0.05\n",
                "[bandwidth_defaults]\nmax_floor_mbps=0.1\n",
                "[router_access]\napi_timeout=0\n",
                "[speed]\nprofile_name_regex='('\n",
                "[speed]\npppoe_sources=[]\n",
                "[nodes]\nppp={name=\"PPP\",type=\"plan\"}\n",
                "[circuits]\nhotspot=\"HS\"\n"] {
        assert!(Config::from_toml_str(&format!("{bad}{MIN}")).is_err(), "{bad}");
    }
}
#[test] fn router_list_problems_are_rejected() {
    assert!(Config::from_toml_str("").is_err());
    assert!(format!("{:#}", Config::from_toml_str(&format!("{MIN}{MIN}")).unwrap_err()).contains("R1"));
}
#[test] fn v3_target_rejects_no_parent() {
    let e = Config::from_toml_str(&format!("[libreqos]\ntarget=\"v3_pair\"\n[network]\nmode=\"no_parent\"\n{MIN}")).unwrap_err();
    assert!(format!("{e:#}").contains("root_promotion"));
    assert!(Config::from_toml_str(&format!("[network]\nmode=\"no_parent\"\n{MIN}")).is_ok());   // auto: Plan 3 checks
}
#[test] fn node_type_defaults_follow_target() {
    let c = Config::from_toml_str(MIN).unwrap();
    assert_eq!(c.nodes.ppp_plan.type_for("plan", false), "plan");
    assert_eq!(c.nodes.ppp_plan.type_for("plan", true), "site");
    let c = Config::from_toml_str(&format!("[nodes]\nppp_plan={{name=\"{{profile}}-{{router}}\",type=\"ap\"}}\n{MIN}")).unwrap();
    assert_eq!(c.nodes.ppp_plan.type_for("plan", true), "ap");
}
#[test] fn per_site_rejects_lease_comment_and_defaults_follow_mode() {
    let s = |mode: &str, src: &str| format!("{MIN}[routers.dhcp]\nenabled=true\n[[routers.dhcp.servers]]\nname=\"LAN\"\nmode=\"{mode}\"\n{src}");
    assert!(Config::from_toml_str(&s("per_site", "speed_sources=[\"lease_comment\"]\n")).is_err());
    let c = Config::from_toml_str(&s("per_plan", "")).unwrap();
    assert_eq!(c.routers[0].dhcp.servers[0].sources(), vec![DhcpSource::LeaseComment]);
    let c = Config::from_toml_str(&s("per_site", "")).unwrap();
    assert_eq!(c.routers[0].dhcp.servers[0].sources(), vec![DhcpSource::ServerComment, DhcpSource::Settings]);
}
```

In `tests/config_doc.rs`: `example_in_configuration_md_parses`. Read `concat!(env!("CARGO_MANIFEST_DIR"), "/docs/CONFIGURATION.md")`, take the text between the first "```toml\n" and the next "```", and `Config::from_toml_str(..).unwrap()`.

- [ ] **Step 3: Run the tests to make sure they fail**

Run: `cargo test` — Expected: compile errors (types not defined).

- [ ] **Step 4: Implement `src/config.rs`**

`validate(&self) -> anyhow::Result<()>` checks, each error names the key:
- `routers` not empty. Router names not empty and unique. DHCP server names unique in a router.
- `0 < min_rate_percentage <= 1`. `safe_min_mbps >= 0.1`. `max_floor_mbps >= 0.2` (default `0.2`, in `[bandwidth_defaults]`). All factors and multipliers `>= 0`. All rates in `[network]` and caps `> 0`.
- Each interval, timeout, and `offline_after` that the doc lists in seconds: `scan_interval_*`, `error_retry_interval`, `connect_timeout`, `api_timeout`, `v2_timeout`, `v3_status_timeout`, `telegram.timeout` `> 0`. `retry_delay >= 0`. `offline_after >= 0`.
- `4 <= id_length <= 32`.
- `speed.pppoe_sources` not empty. `profile_name_regex` compiles and has 1 or 2 capture groups.
- `pppoe_default_rate` parses with `crate::speed::parse_default_rate` (Task 2). Until Task 2 exists, check that it contains `/`.
- Templates: `nodes.ppp.name` and `nodes.hotspot.name` contain `{router}`. `ppp_plan` contains `{profile}` and `{router}`. `dhcp_site` contains `{server}` and `{router}`. `dhcp_plan` contains `{plan}` and `{router}`. `circuits.pppoe` contains `{user}`. `circuits.hotspot` and `circuits.dhcp` contain `{key}`.
- `per_site` servers: `sources()` does not contain `LeaseComment`. No list is empty.
- `targets_v3() == Some(true)` and `network.mode == NoParent`: error that names `root_promotion` (ADR-0006).

Use `anyhow::Context` to wrap the `toml` error, so that the unknown key name stays in the message.

- [ ] **Step 5: Run the tests to make sure they pass**

Run: `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` — Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src tests .gitignore
git commit -m "feat(config): add TOML config with defaults and checks"
```

---

### Task 2: Rate format and speed rules

**Files:**
- Modify: `src/speed.rs`, `src/config.rs` (use `parse_default_rate` in `validate`)

**Interfaces:**
- Consumes: `config::{Speed, RateLimitOrder, UnitlessUnit}`.
- Produces:
  - `speed::round3(x: f64) -> f64`: `format!("{:.3}", x).parse().unwrap()`.
  - `speed::fmt_rate(x: f64) -> String`: `format!("{:.3}", x)`, then trim trailing `0` and a trailing `.`.
  - `speed::SpeedParser::new(cfg: &Speed) -> anyhow::Result<SpeedParser>`
  - `SpeedParser::comment(&self, s: &str) -> Option<Rate>`: the three forms of `docs/CONFIGURATION.md` "Comment format", case-insensitive, always download/upload. Forms 1 and 2 accept `k` ×0.001, `m` ×1, `g` ×1000. Form 3 accepts only `m`. Each unit must end the word: `\b` after the unit.
  - `SpeedParser::rate_limit(&self, s: &str) -> Option<Rate>`: first whitespace token, split on `/`. Unit `k`, `M`, `G` (any case), or no unit = `unitless_unit` (`Bps` ×1e-6, `Mbps` ×1). `RouterosRxTx` (default): download = second value, upload = first value. `None` unless both values `> 0`.
  - `SpeedParser::profile_name(&self, s: &str) -> Option<Rate>`: last match of `profile_name_regex`. Group 1 is the number. Group 2 (if present) is the unit, an empty unit = `unitless_unit`. `None` if the value is `<= 0`.
  - `speed::parse_default_rate(s: &str) -> Option<Rate>`: `A/B`, download/upload order, a number with no unit is Mbps.
  - `speed::clamp(x: f64, safe_min: f64) -> f64`: `round3(x).max(safe_min)`.
  - `speed::CircuitRates { down_min, up_min, down_max, up_max: f64 }` and `speed::circuit_rates(base: Rate, min_pct: f64, safe_min: f64, max_floor: f64) -> CircuitRates`: max = `clamp(base).max(max_floor)`, min = `clamp(base × min_pct).min(max)`.

- [ ] **Step 1: Write the failing tests**

```rust
// p() = SpeedParser::new(&Speed::default()).unwrap(); r(d,u) = Some(Rate{down:d,up:u})
// legacy() = SpeedParser::new(&Speed { rate_limit_order: RateLimitOrder::DownloadUpload, unitless_unit: UnitlessUnit::Mbps, ..Speed::default() }).unwrap()
#[test] fn round_and_format() {
    assert_eq!(round3(0.0625), 0.062);  assert_eq!(round3(2.0005), 2.001);  assert_eq!(round3(1.2345), 1.234);
    assert_eq!(fmt_rate(50.0), "50");   assert_eq!(fmt_rate(157.5), "157.5");
    assert_eq!(fmt_rate(0.128), "0.128"); assert_eq!(fmt_rate(0.1 + 0.2), "0.3");
}
#[test] fn comment_forms() {
    assert_eq!(p().comment("Alice|25M/5M"), r(25.0, 5.0));
    assert_eq!(p().comment("x 10M/2M |25M/5M"), r(25.0, 5.0));
    assert_eq!(p().comment("Plan 25m"), r(25.0, 25.0));
    assert_eq!(p().comment("1G/500M"), r(1000.0, 500.0));
    assert_eq!(p().comment("|100k/50k"), r(0.1, 0.05));
    assert_eq!(p().comment("512k"), None);  assert_eq!(p().comment("Wifi 5G"), None);
    assert_eq!(p().comment("Room 5Main"), None);
    assert_eq!(p().comment(""), None);  assert_eq!(p().comment("no speed"), None);
}
#[test] fn rate_limit_routeros_default() {
    assert_eq!(p().rate_limit("2M/10M 4M/20M 1M/8M 10/10"), r(10.0, 2.0));  // rx/tx: upload/download
    assert_eq!(p().rate_limit("2m/10m"), r(10.0, 2.0));
    assert_eq!(p().rate_limit("5000000/10000000"), r(10.0, 5.0));          // no unit = bit/s
    assert_eq!(p().rate_limit("10M"), None);  assert_eq!(p().rate_limit("0/0"), None);
}
#[test] fn rate_limit_legacy_option() {
    assert_eq!(legacy().rate_limit("10M/2M"), r(10.0, 2.0));
    assert_eq!(legacy().rate_limit("10/5"), r(10.0, 5.0));
}
#[test] fn profile_name_rule() {
    assert_eq!(p().profile_name("Gold-25M"), r(25.0, 25.0));
    assert_eq!(p().profile_name("Tier-15M-WAN1"), r(15.0, 15.0));
    assert_eq!(p().profile_name("1G-Max"), r(1000.0, 1000.0));
    assert_eq!(p().profile_name("Gold-25"), None);
}
#[test] fn default_rate_and_circuit_rates() {
    assert_eq!(parse_default_rate("10M/10M"), r(10.0, 10.0));
    let c = circuit_rates(Rate { down: 50.0, up: 50.0 }, 0.1, 0.128, 0.2);
    assert_eq!((c.down_min, c.down_max), (5.0, 50.0));
    let c = circuit_rates(Rate { down: 0.5, up: 0.5 }, 0.1, 0.128, 0.2);
    assert_eq!((c.down_min, c.down_max), (0.128, 0.5));
    let c = circuit_rates(Rate { down: 0.1, up: 0.1 }, 0.1, 0.128, 0.2);   // LibreQoS rejects max < 0.2
    assert_eq!((c.down_min, c.down_max), (0.128, 0.2));
}
```

- [ ] **Step 2: Run the tests to make sure they fail**

Run: `cargo test speed` — Expected: compile errors.

- [ ] **Step 3: Implement `src/speed.rs`**

Compile each regex one time in `SpeedParser::new`.

- [ ] **Step 4: Run the checks**

Run: `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` — Expected: all pass.

- [ ] **Step 5: Commit** — `git commit -m "feat(speed): add rate parsing, rounding, and rate text"`

---

### Task 3: Model, state file, and seed

**Files:**
- Create: `src/model.rs`, `src/state.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces:
  - `model::CsvRow`: struct with 15 `String` fields, `#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]`, each field `#[serde(rename = "<column>", default)]`. Fields: `circuit_id, circuit_name, device_id, device_name, parent_node, parent_node_id, anchor_node_id, mac, ipv4, ipv6, download_min, upload_min, download_max, upload_max, comment`. Columns: `Circuit ID, Circuit Name, Device ID, Device Name, Parent Node, Parent Node ID, Anchor Node ID, MAC, IPv4, IPv6, Download Min Mbps, Upload Min Mbps, Download Max Mbps, Upload Max Mbps, Comment`.
  - `model::Columns { V2, V3 }` with `pub fn names(self) -> &'static [&'static str]`: 14 columns (no `Anchor Node ID`) and 15 columns, each in the order above.
  - `model::Source { Ppp, Hotspot, Dhcp, Unknown }`, `#[serde(rename_all = "lowercase")]`.
  - `model::RosRow = BTreeMap<String, String>`. `model::RouterSnapshot { ppp_secret, ppp_active, ppp_profile, hotspot_active, dhcp_server, dhcp_lease: Vec<RosRow> }`, `Deserialize`, each `#[serde(default)]`.
  - `model::StoredCircuit { router: Option<String>, source: Source, last_seen: u64, row: CsvRow }`
  - `model::StoredNode { id: String, router: Option<String>, parent: Option<String>, r#type: String, down: f64, up: f64 }` (`type` in JSON).
  - `model::RouterMemo { unreachable_since: Option<u64> }`
  - `model::State { version: u32, circuits: BTreeMap<String, StoredCircuit>, nodes: BTreeMap<String, StoredNode>, routers: BTreeMap<String, RouterMemo> }`, `Default` with `version = 1`.
  - `state::read_csv(text: &str) -> anyhow::Result<Vec<CsvRow>>`: skip rows with an empty Circuit Name. A later row with the same name replaces the earlier row. Keep the first position of each name.
  - `state::split_static(rows: Vec<CsvRow>, static_comment: &str) -> (Vec<CsvRow>, Vec<CsvRow>)`: `(static, other)`. Compare `comment.trim()` with `static_comment`, case is ignored.
  - `state::seed_state(rows: &[CsvRow], now: u64) -> State`: the spec section 3 seed rules. The caller passes only the rows that are not static.
  - `impl State { pub fn from_json(s: &str) -> anyhow::Result<State>; pub fn to_json(&self) -> String }`. Reject `version != 1`.

- [ ] **Step 1: Write the failing tests**

```rust
// src/state.rs
const HDR: &str = "Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,MAC,IPv4,IPv6,Download Min Mbps,Upload Min Mbps,Download Max Mbps,Upload Max Mbps,Comment\n";
#[test] fn read_csv_accepts_bom_crlf_and_missing_columns() {
    let t = format!("\u{feff}{}", HDR.replace('\n', "\r\n")) + "A1,alice,D1,alice,P,,10.0.0.2,,5.0,5.0,50.0,50.0,PPP\r\n";
    let rows = read_csv(&t).unwrap();
    assert_eq!(rows[0].circuit_id, "A1"); assert_eq!(rows[0].comment, "PPP");
    let rows = read_csv("Circuit Name,IPv4\nbob,10.0.0.3\n").unwrap();
    assert_eq!((rows[0].circuit_name.as_str(), rows[0].comment.as_str()), ("bob", ""));
    let v3 = "Circuit ID,Circuit Name,Parent Node,Parent Node ID,Anchor Node ID\nC1,carl,N,lq3-ab,\n";
    assert_eq!(read_csv(v3).unwrap()[0].parent_node_id, "lq3-ab");
}
#[test] fn read_csv_skips_blank_names_and_later_duplicate_wins() {
    let rows = read_csv(&format!("{HDR}A,x,,,,,,,,,,,1\nB,,,,,,,,,,,,2\nC,x,,,,,,,,,,,3\n")).unwrap();
    assert_eq!(rows.len(), 1); assert_eq!(rows[0].circuit_id, "C");
}
#[test] fn split_static_ignores_case() {
    let mk = |n: &str, c: &str| CsvRow { circuit_name: n.into(), comment: c.into(), ..Default::default() };
    let (s, o) = split_static(vec![mk("a", "Static"), mk("b", "PPP")], "static");
    assert_eq!((s.len(), o.len()), (1, 1));
}
#[test] fn seed_infers_source_and_keeps_ids() {
    let mk = |n: &str, c: &str| CsvRow { circuit_id: format!("ID-{n}"), circuit_name: n.into(), comment: c.into(), ..Default::default() };
    let st = seed_state(&[mk("alice", "PPP"), mk("HS-AABB", "HS"), mk("DHCP-pc", "pc"), mk("odd", "x")], 7);
    assert_eq!(st.circuits["alice"].source, Source::Ppp);
    assert_eq!(st.circuits["HS-AABB"].source, Source::Hotspot);
    assert_eq!(st.circuits["DHCP-pc"].source, Source::Dhcp);
    assert_eq!(st.circuits["odd"].source, Source::Unknown);
    assert_eq!(st.circuits["alice"].row.circuit_id, "ID-alice");
    assert_eq!((st.circuits["alice"].router.clone(), st.circuits["alice"].last_seen), (None, 7));
}
#[test] fn state_json_round_trip_and_version_check() {
    let st = seed_state(&[], 1);
    assert_eq!(State::from_json(&st.to_json()).unwrap().version, 1);
    assert!(State::from_json(r#"{"version":2,"circuits":{},"nodes":{},"routers":{}}"#).is_err());
}
```

- [ ] **Step 2: Run the tests to make sure they fail** — `cargo test state` — compile errors.
- [ ] **Step 3: Implement `src/model.rs` and `src/state.rs`** — `csv::ReaderBuilder::new().flexible(true)` and `deserialize::<CsvRow>()`.
- [ ] **Step 4: Run the checks** — all pass.
- [ ] **Step 5: Commit** — `git commit -m "feat(state): add model types, CSV read, and seed"`

---

### Task 4: Circuit collection for one router

**Files:**
- Create: `src/sources.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `Config`, `RouterCfg`, `RouterSnapshot`, `SpeedParser`, `Rate`, `Source`.
- Produces:
  - `sources::GroupKey { Ppp, PppPlan(String /*profile*/), Hotspot, DhcpSite(String /*server*/), DhcpPlan(String /*label*/) }`, `Debug, Clone, PartialEq, Eq, PartialOrd, Ord`.
  - `sources::Collected { name, device_name, mac, ipv4, comment: String, source: Source, base: Rate, group: GroupKey }`
  - `sources::collect_router(cfg: &Config, router: &RouterCfg, snap: &RouterSnapshot, speed: &SpeedParser, report: &mut Vec<String>) -> Vec<Collected>`: the spec section 5 rules. Output order: PPPoE in name order, then Hotspot in reply order, then DHCP servers in config order with leases in reply order. A disabled source gives nothing. `report` gets one line for each skipped lease with no rate, and one line with the count of PPP active sessions with no secret (only if the count is not 0).
  - `sources::plan_label(r: Rate) -> String`: `"<trunc(down)>M"` if `down == up`, else `"<trunc(down)>M-<trunc(up)>M"`.
- Rules that are not in the spec table:
  - A missing field is an empty string. Use one helper `fn f<'a>(row: &'a RosRow, key: &str) -> &'a str` that returns `""` for a missing key and trims the value.
  - PPP secret is out if `disabled` or `inactive` is present and not `"false"` (case is ignored). Later secret or active rows with the same name replace earlier ones. Profile: secret `profile`, else `pppoe_default_profile`.
  - PPP speed: walk `speed.pppoe_sources`. `SecretComment`, `ActiveComment`, `ProfileComment` use `SpeedParser::comment`. `ProfileRateLimit` uses `rate_limit` on the profile row `rate-limit`. `ProfileName` uses `profile_name`. `Default` uses `parse_default_rate(pppoe_default_rate)`. A profile that is not in `ppp_profile` gives `None` for the three profile sources. If nothing matches, use the default rate.
  - Group for PPP: `PppPlan(profile)` if `per_plan_node`, else `Ppp`.
  - DHCP server rate: one time per server, from `sources()` in order. `ServerComment` uses the comment of the first `dhcp_server` row with `name` = server name. `Settings` uses `settings_rate`. `LeaseComment` is per lease (`per_plan` only). If no rate exists for a lease, skip the lease.
  - DHCP `per_plan` group: `DhcpPlan(plan_label(base))`.

- [ ] **Step 1: Write the failing tests**

Build a `Config` from TOML in a helper `cfg(extra: &str)` that adds one router `R1` with all three sources enabled, and a helper `snap(json: &str) -> RouterSnapshot` with `serde_json::from_str`. Tests:

```rust
#[test] fn ppp_needs_active_and_enabled_secret() {
    // secrets: alice ok, bob disabled="true", carol inactive="true"; active: alice, bob, carol, dave (no secret)
    let out = collect_router(&c, &c.routers[0], &s, &sp, &mut rep);
    assert_eq!(out.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["alice"]);
    assert!(rep.iter().any(|l| l.contains('1') && l.contains("secret")));
}
#[test] fn ppp_speed_chain_order() {
    // u1 secret comment "|30M/6M" and profile comment "20M" -> 30/6
    // u2 active comment "25M" -> 25/25
    // u3 profile "P" comment "20M/4M" -> 20/4
    // u4 profile "Q" rate-limit "2M/8M" -> 8/2 (RouterOS rx/tx: download 8, upload 2)
    // u5 profile "Gold-12M" (no comment, no rate-limit) -> 12/12
    // u6 profile "missing" -> default 10/10
}
#[test] fn ppp_mac_falls_back_to_secret_caller_id() { /* active caller-id "" , secret "AA:BB:CC:00:00:01" */ }
#[test] fn hotspot_key_rules() {
    // include_mac=true: mac "aa:bb:cc:dd:ee:ff" -> name "HS-AABBCCDDEEFF", device_name = user
    // no mac, user "guest" -> "HS-guest"; no mac and no user -> skipped
    // base = router download_limit_mbps / upload_limit_mbps
}
#[test] fn dhcp_per_site_filters_and_names() {
    // lease server "LAN" host-name "pc1" -> "DHCP-pc1", comment "pc1"
    // lease with no host-name key -> "DHCP-<MAC no colons upper>", comment "DHCP"
    // lease other server, lease without mac, lease without address -> skipped
    // active-address wins over address
    // require_bound=true: lease status "waiting" skipped
}
#[test] fn dhcp_server_comment_beats_settings() {
    // dhcp_server row {name:"LAN", comment:"Site|80M/40M"} -> base 80/40 for every LAN lease
    // server "IOT" with no comment -> settings default_plan_down_mbps
}
#[test] fn dhcp_per_plan_uses_lease_comment() {
    // comment "Juan|25M/5M" -> name "DHCP-Juan", group DhcpPlan("25M-5M"), comment "Juan"
    // comment "Plan 2.5M" -> group DhcpPlan("2M")
    // no comment speed -> skipped, one report line
}
```

Write each test with full row data and `assert_eq!` on `name`, `device_name`, `mac`, `ipv4`, `comment`, `base`, `group` for the cases named in its comment.

- [ ] **Step 2: Run the tests to make sure they fail** — compile errors.
- [ ] **Step 3: Implement `src/sources.rs`** — format templates with `str::replace` on `{user}` and `{key}`.
- [ ] **Step 4: Run the checks** — all pass.
- [ ] **Step 5: Commit** — `git commit -m "feat(sources): collect circuits from RouterOS rows"`

---

### Task 5: Topology

**Files:**
- Create: `src/topology.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `Config`, `GroupKey`, `Collected`, `StoredNode`, `CsvRow`, `clamp`.
- Produces:
  - `topology::parent_for(cfg: &Config, router: &str, group: &GroupKey) -> (String, String)`: `(Parent Node, Parent Node ID)`. Grouped modes give the group node. `flat_router_root` gives the router root. `flat_global_root` gives the global root. `no_parent` gives `("", "")`. `flat_promotion` gives `("", "")`, and `cycle::assemble` sets the circuit's own client node (spec section 6).
  - `topology::group_node_name(cfg: &Config, router: &str, group: &GroupKey) -> String`: fill the `[nodes]` template.
  - `topology::fnv1a64(bytes: &[u8]) -> u64` (offset `0xcbf29ce484222325`, prime `0x100000001b3`).
  - `topology::node_id(kind: &str, router: &str, key: &str) -> String`: `format!("lq3-{:016x}", fnv1a64(format!("{kind}|{router}|{key}").as_bytes()))`. `topology::client_node_id(circuit_id: &str) -> String`: `format!("lq3-c-{circuit_id}")`.
  - `topology::group_node_id(router: &str, group: &GroupKey) -> String`: kind and key from the group (`Ppp` gives `("ppp", "")`, `PppPlan(p)` gives `("ppp_plan", p)`, `Hotspot` gives `("hotspot", "")`, `DhcpSite(s)` gives `("dhcp_site", s)`, `DhcpPlan(l)` gives `("dhcp_plan", l)`). Router root: `node_id("root", router, "")`. Global root: `node_id("root", "", "")`.
  - `topology::GroupTotal { count: usize, sum: Rate, unit: Rate }`: `unit` is the `base` of the first circuit in the group.
  - `topology::RouterGroups { router: String, ppp_enabled: bool, per_plan_node: bool, totals: BTreeMap<GroupKey, GroupTotal> }`. `cycle::assemble` fills it from the kept circuits (duplicate-IP skips are not counted). Each group counts unique Circuit Names. If two rows give the same name, the last one wins, and the group counts it one time.
  - `topology::build_nodes(cfg: &Config, v3: bool, online: &[RouterGroups], carried: &BTreeMap<String, StoredNode>, rows: &[CsvRow]) -> BTreeMap<String, StoredNode>`: node types from `NodeTemplate::type_for` with v2 defaults `plan` (`ppp`, `ppp_plan`, `dhcp_plan`) and `site` (`hotspot`, `dhcp_site`). spec section 6. `carried` holds the previous nodes of `Failed`, `Unreachable`, and kept `Offline` routers. `rows` feeds `flat_promotion`. The caller passes only rows that are not static. The rates come from the max columns.
  - `topology::to_network_json(nodes: &BTreeMap<String, StoredNode>, preserved: Option<serde_json::Value>, prev_generated: &BTreeMap<String, StoredNode>) -> serde_json::Value`: nest by `parent`. Each node is `{"children":{},"downloadBandwidthMbps":down,"id":id,"type":type,"uploadBandwidthMbps":up}` with JSON floats.
- Group rates: `Hotspot` and `DhcpSite` use `count × unit × factor`, from left to right. `Ppp`, `PppPlan`, and `DhcpPlan` use `sum × factor`. Then `min` with the cap of the source, then `clamp`. The factors come from `HotspotCfg::factors`, `DhcpServer::factors`, and `pppoe_aggregate_factor`.

- [ ] **Step 1: Write the failing tests**

One test for each mode with router `R1`, PPP `per_plan_node = true` (profiles `Gold` 2 users × 50, `Silver` 1 user × 20), and one DHCP `per_site` server `LAN` with 4 leases × 50, `pppoe_aggregate_factor = 0.35`. Assert with `serde_json::json!`. `id(k, key)` is `node_id(k, "R1", key)`:

```rust
#[test] fn fnv_and_id_format() {
    assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);  assert_eq!(fnv1a64(b"a"), 0xaf63dc4c8601ec8c);
    let i = node_id("ppp_plan", "R1", "Gold");
    assert!(i.len() == 20 && i.starts_with("lq3-") && i[4..].chars().all(|c| c.is_ascii_hexdigit()));
}
#[test] fn id_does_not_change_with_the_name_template() {}       // ppp_plan template "{profile}@{router}" -> same id
#[test] fn root_promotion_puts_groups_at_top() {
    assert_eq!(to_network_json(&build_nodes(..), None, &empty), json!({
      "DHCP-LAN-R1": {"children":{},"downloadBandwidthMbps":200.0,"id":id("dhcp_site","LAN"),"type":"site","uploadBandwidthMbps":200.0},
      "Gold-R1":     {"children":{},"downloadBandwidthMbps":35.0,"id":id("ppp_plan","Gold"),"type":"plan","uploadBandwidthMbps":35.0},
      "Silver-R1":   {"children":{},"downloadBandwidthMbps":7.0,"id":id("ppp_plan","Silver"),"type":"plan","uploadBandwidthMbps":7.0}}));
}
#[test] fn grouped_router_root_nests_groups_under_router() {}   // root R1 640/640 site, same three children
#[test] fn flat_router_root_and_flat_global_root_make_roots_only() {}
#[test] fn no_parent_gives_empty_object() {}
#[test] fn flat_promotion_gives_one_client_node_per_row() {}   // id = client_node_id(circuit_id)
#[test] fn ppp_node_exists_with_zero_users() {}               // per_plan_node=false, no users -> "PPP-R1" 0.128
#[test] fn backstop_cap_limits_group() {}
#[test] fn v3_flag_makes_group_nodes_site() {}               // v3=true: Gold-R1 and PPP-R1 type "site"                     // pppoe cap 30 enabled -> Gold-R1 30.0
#[test] fn carried_nodes_of_failed_router_stay() {}
#[test] fn preserve_keeps_manual_nodes_and_drops_old_generated() {}
#[test] fn parent_for_each_mode() {}
```

- [ ] **Step 2: Run the tests to make sure they fail** — compile errors.
- [ ] **Step 3: Implement `src/topology.rs`**
- [ ] **Step 4: Run the checks** — all pass.
- [ ] **Step 5: Commit** — `git commit -m "feat(topology): build nodes for the six network modes"`

---

### Task 6: Cycle assembly and validation

**Files:**
- Create: `src/cycle.rs`, `src/validate.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: Tasks 1 to 5: `topology::parent_for`, `topology::RouterGroups`, `topology::GroupTotal`, `topology::build_nodes`, `topology::to_network_json`.
- Produces:
  - `cycle::RouterResult { Online(RouterSnapshot), Failed, Unreachable, Offline }`
  - `cycle::Report { added: Vec<String>, updated: Vec<(String, Vec<String>)>, removed: Vec<String>, held: Vec<String>, duplicates: Vec<(String /*ip*/, String /*kept*/, String /*dropped*/)>, warnings: Vec<String>, alerts: Vec<String> }`, `Default`. `held` lists the rows that the guard kept.
  - `model::State` gets `guard: Option<GuardHold>` with `#[serde(default)]`, and `model::GuardHold { fingerprint: u64, since: u64, count: usize }`. `updated` lists the changed column names.
  - `cycle::CycleInput<'a> { config: &'a Config, state: &'a State, static_rows: &'a [CsvRow], routers: Vec<(String, RouterResult)>, current_network: Option<serde_json::Value>, v3: bool, force: bool, now: u64 }`. If `v3` and `network.mode == NoParent`, return an error that names `root_promotion`.
  - `cycle::CycleOutput { rows: Vec<CsvRow>, network: serde_json::Value, state: State, report: Report }`
  - `cycle::assemble(input: CycleInput, new_id: &mut dyn FnMut() -> String) -> anyhow::Result<CycleOutput>`: the spec section 4 rules 1 to 7. `rows` excludes nothing that the CSV must hold, static rows included. `state.circuits` excludes static rows. `state.nodes` holds the generated nodes. `state.routers` is a copy of the input (Plan 2 sets it).
  - `validate::check(rows: &[CsvRow], network: &serde_json::Value, mode: NetworkMode) -> anyhow::Result<()>`: spec section 7 stop errors. The node names are all keys at all levels of `network`, so that preserved manual nodes count. A rate field that does not parse as `f64` is an error. The error names the Circuit Name.
- Row fields for a collected circuit: `(parent_node, parent_node_id) = topology::parent_for(cfg, router, &group)`, in `flat_promotion` `(circuit_name, client_node_id(circuit_id))`. Rates from `circuit_rates(.., max_floor_mbps)` as `fmt_rate`. `ipv6 = ""`, `anchor_node_id = ""`. For a new row, `circuit_id` and `device_id` come from `new_id()`. Call it again while the value is in use by a row or by an ID made in this cycle.
- Removal guard: spec section 4 rule 6. Use `topology::fnv1a64` for the fingerprint.
- Duplicate IP (spec section 7): run after removal, on all rows with a non-empty IPv4. The new-name check in rule 4 runs first, during collection.

- [ ] **Step 1: Write the failing tests**

Helper `ids()` returns a closure that yields `"N0001"`, `"N0002"`, ... Helper `run(cfg, state, statics, routers, now)`. Use mode `root_promotion`, one or two routers.

```rust
#[test] fn new_circuit_gets_new_ids_and_existing_keeps_them() {}
#[test] fn id_collision_asks_for_another_id() {}            // state already has circuit_id "N0001"
#[test] fn failed_and_unreachable_router_keep_rows_and_nodes() {}  // R2 Failed: its rows and state.nodes stay byte-equal
#[test] fn online_router_removes_rows_not_seen() {}
#[test] fn offline_remove_ignores_grace_and_keep_keeps() {}
#[test] fn seed_rows_go_only_when_all_routers_online() {}  // router=None row; R2 Failed -> stays; all Online -> removed
#[test] fn rows_of_a_router_not_in_config_are_removed() {}
#[test] fn grace_keeps_a_missing_subscriber() {}           // grace 600: last_seen now-300 stays, now-600 goes
#[test] fn remove_inactive_entries_false_removes_nothing() {}
#[test] fn static_row_passes_through_and_collected_replaces_it() {} // static "10" stays "10"; same name collected -> PPP row
#[test] fn duplicate_ip_new_vs_existing_keeps_existing() {}
#[test] fn duplicate_ip_existing_vs_existing_keeps_collected() {}  // R1 Online gives 10.0.0.9, R2 Failed row also 10.0.0.9
#[test] fn same_name_on_two_routers_later_router_wins() {}
#[test] fn empty_online_router_removes_its_rows() {}       // Review Focus 4
#[test] fn v3_with_no_parent_stops() {}
#[test] fn guard_holds_a_mass_removal_and_keeps_other_changes() {}   // 60 of 100 rows gone: held, 1 new row still added, alert in report
#[test] fn guard_releases_on_same_fingerprint_next_cycle() {}
#[test] fn guard_restarts_on_a_different_fingerprint() {}
#[test] fn guard_releases_on_force() {}
#[test] fn guard_off_removes_at_once() {}
#[test] fn small_removal_is_not_held() {}                          // 3 of 100 rows
#[test] fn flat_promotion_has_no_client_node_for_a_static_row() {}
#[test] fn flat_promotion_parent_is_own_client_node() {}   // Parent Node = Circuit Name, Parent Node ID = lq3-c-<Circuit ID>
#[test] fn group_counts_skip_duplicate_ip_circuits() {}
```

```rust
// src/validate.rs
#[test] fn duplicate_circuit_id_stops() {}
#[test] fn missing_parent_stops_except_in_no_parent() {}
#[test] fn parent_node_id_must_match_the_parent_node() {}
#[test] fn duplicate_node_name_at_any_level_stops() {}
#[test] fn min_above_max_or_bad_number_stops() {}
#[test] fn rates_below_libreqos_floor_stop() {}              // max 0.15 or min 0.05 in a static row
```

Each test asserts the exact `rows` (Circuit Name, IDs, Parent Node, rate strings) and the matching `report` entry.

- [ ] **Step 2: Run the tests to make sure they fail** — compile errors.
- [ ] **Step 3: Implement `src/cycle.rs` and `src/validate.rs`**
- [ ] **Step 4: Run the checks** — all pass.
- [ ] **Step 5: Commit** — `git commit -m "feat(cycle): assemble rows with router state and removal rules"`

---

### Task 7: Pair writer

**Files:**
- Create: `src/writer/mod.rs`, `src/writer/pair.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces:
  - Node rates in `network.json` are `round3` values. The JSON number text is from `serde_json` (`50.0`, `157.5`).
  - `writer::pair::render_csv(rows: &[CsvRow], cols: Columns) -> String`: header of `cols.names()` plus rows sorted by `(parent_node, circuit_name, device_name, ipv4, mac)`. Write records by hand with `csv::Writer::write_record`, one field for each name in `cols`. `csv::WriterBuilder::new().terminator(csv::Terminator::Any(b'\n')).quote_style(csv::QuoteStyle::Necessary)`.
  - `writer::pair::render_network_json(v: &serde_json::Value) -> String`: `PrettyFormatter::with_indent(b"    ")`, then `"\n"`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn csv_header_sort_and_quoting() {
    let out = render_csv(&[row("b", "P2"), row("a,b", "P1"), row("Niño", "P1")], Columns::V2);
    assert!(out.starts_with("Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,Parent Node ID,MAC,IPv4,IPv6,Download Min Mbps,Upload Min Mbps,Download Max Mbps,Upload Max Mbps,Comment\n"));
    let names: Vec<&str> = out.lines().skip(1).map(|l| l.split(',').nth(1).unwrap()).collect();
    assert_eq!(names, ["\"a", "Niño", "b"]);           // P1 before P2, then bytes; "a,b" is quoted
    assert!(out.contains(",\"a,b\","));  assert!(out.ends_with('\n'));
}
#[test] fn csv_quotes_like_python() {
    // Python csv: '"a,b","say ""hi""",Niño,\n'
    let r = CsvRow { circuit_id: "a,b".into(), circuit_name: "say \"hi\"".into(), device_id: "Niño".into(), ..Default::default() };
    assert!(render_csv(&[r], Columns::V2).lines().nth(1).unwrap().starts_with("\"a,b\",\"say \"\"hi\"\"\",Niño,,"));
}
#[test] fn csv_headers_for_v2_and_v3() {
    assert!(render_csv(&[], Columns::V2).starts_with("Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,Parent Node ID,MAC,"));
    assert!(render_csv(&[], Columns::V3).starts_with("Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,Parent Node ID,Anchor Node ID,MAC,"));
}
#[test] fn network_json_matches_python_bytes() {
    let v = serde_json::json!({"Router-1":{"children":{"PPP-Router-1":{"children":{},"downloadBandwidthMbps":0.128,"type":"plan","uploadBandwidthMbps":0.128}},"downloadBandwidthMbps":620.0,"type":"site","uploadBandwidthMbps":620.0},"Niño-HS":{"children":{},"downloadBandwidthMbps":157.5,"type":"client","uploadBandwidthMbps":157.5}});
    // Four-space indent, sorted keys, raw UTF-8, final newline:
    assert_eq!(render_network_json(&v), "{\n    \"Niño-HS\": {\n        \"children\": {},\n        \"downloadBandwidthMbps\": 157.5,\n        \"type\": \"client\",\n        \"uploadBandwidthMbps\": 157.5\n    },\n    \"Router-1\": {\n        \"children\": {\n            \"PPP-Router-1\": {\n                \"children\": {},\n                \"downloadBandwidthMbps\": 0.128,\n                \"type\": \"plan\",\n                \"uploadBandwidthMbps\": 0.128\n            }\n        },\n        \"downloadBandwidthMbps\": 620.0,\n        \"type\": \"site\",\n        \"uploadBandwidthMbps\": 620.0\n    }\n}\n");
    assert_eq!(render_network_json(&serde_json::json!({})), "{}\n");
}
```

- [ ] **Step 2: Run the tests to make sure they fail** — compile errors.
- [ ] **Step 3: Implement `src/writer/pair.rs`**
- [ ] **Step 4: Run the checks** — all pass.
- [ ] **Step 5: Commit** — `git commit -m "feat(writer): render ShapedDevices.csv and network.json"`

---

### Task 8: Scenario tests and v3 check

**Files:**
- Create: `tests/scenarios.rs`, `tests/scenarios/<case>/{config.toml, snapshot.json, previous.csv, expected/ShapedDevices.csv, expected/network.json, expected/report.json}`, `tools/v3sim/check.sh`

**Interfaces:**
- Consumes: `Config::from_toml_str`, `read_csv`, `split_static`, `seed_state`, `assemble`, `render_csv`, `render_network_json`.
- Fixture rules:
  - `snapshot.json`: `{"routers": {"<name>": RouterSnapshot}, "results": {"<name>": "online" | "failed" | "unreachable" | "offline"}}`. A router that is not in `results` is `online`.
  - `expected/report.json`: `{"added": [...], "removed": [...], "held": [...], "alerts": <count>}`. Only these keys.
  - Write each expected file by hand from the spec. Check each number with the formula in a comment line in `config.toml`.
  - Synthetic data only: `100.64.0.0/10` addresses, `02:00:...` MAC addresses, made-up names.
- Cases (one folder each), `v3 = false` unless the name says `v3`:
  1. `root_promotion`: one router `Hub-1`. PPPoE `per_plan_node = true` with 2 profiles (one comment speed, one `rate-limit` `5M/50M`, so the RouterOS order gives 50/5), DHCP `per_site` with 3 servers (one with 0 leases), `pppoe_aggregate_factor = 0.35`, `min_rate_percentage = 0.1`, PPPoE cap 620. `previous.csv` has one row that stays, one row that goes, one static row. The snapshot has one new circuit.
  2. `grouped_router_root`: two routers. Router A: PPPoE `per_plan_node = false`, Hotspot `include_mac = true`. Router B: DHCP `per_plan` with lease comments `Ana|25M/5M`, `Ben|10M`, `Cy|10M`.
  3. `flat_router_root`, 4. `flat_global_root`, 5. `no_parent`, 6. `flat_promotion`: the snapshot and `previous.csv` of case 2, with only the mode changed.
  7. `failed_router`: case 2 with router B `failed`. Router B rows and nodes stay the same as in `previous.csv` and the previous state.
  8. `guard_hold`: `previous.csv` with 80 PPP rows. The snapshot has 20 of them. Expected: 60 rows `held`, 1 alert, files keep the 80 rows.
  9. `v3_root_promotion`: case 1 with `v3 = true`. Group node types are `site`. Columns `V3`.

- [ ] **Step 1: Write `tests/scenarios.rs`**

```rust
#[test] fn scenarios_match_expected_output() {
    // For each case folder, in name order:
    //   cfg = Config::from_toml_str(config.toml); prev = read_csv(previous.csv)
    //   (statics, other) = split_static(prev, &cfg.runtime.static_comment); state = seed_state(&other, 1_000)
    //   routers = cfg.routers in order -> (name, result from snapshot.json)
    //   v3 = case name starts with "v3_"
    //   out = assemble(CycleInput { .., current_network: None, v3, force: false, now: 1_000 }, &mut ids())
    //   cols = if v3 { Columns::V3 } else { Columns::V2 }
    //   assert_eq!(records(&render_csv(&out.rows, cols), &prev_names), records(&expected_csv, &prev_names), "{case}")
    //   assert_eq!(render_network_json(&out.network), expected/network.json, "{case}")
    //   assert report keys of expected/report.json
    //   write both rendered files to target/scenario-out/<case>/
}
```

`ids()` yields `"N0001"`, `"N0002"`, and so on. `records(text, keep)` parses CSV text to `Vec<Vec<String>>` and replaces the Circuit ID, Device ID, and Parent Node ID of `flat_promotion` rows with `"*"` for each Circuit Name not in `keep`.

- [ ] **Step 2: Write the nine scenarios**

- [ ] **Step 3: Run the scenario test**

Run: `cargo test --test scenarios -- --nocapture` — Expected: PASS for all nine cases. If a case fails, find which side is wrong against the spec. Fix the code or the expected file, and say which in the commit message.

- [ ] **Step 4: Write `tools/v3sim/check.sh <pair-dir>` (dev only)**

It follows `brain/30-knowledge/v3-offline-simulation-method.md`: a new `src/` and `state/` folder for each run (no `rm`), `env -i`, `strace -f -e inject=connect:error=EACCES`, and `write_compiled_topology_from_legacy_artifacts("shaped_devices", "full")`. It prints `RESULT: True` or the v3 error. It fails if `strace` shows a write outside the run folder. It needs `LQOSYNC3_V3_BINDING=<dir with liblqos_python.so>`. If the variable is not set, it prints `skip: no v3 binding` and exits 0.

Run: `for d in target/scenario-out/*/; do tools/v3sim/check.sh "$d"; done` — Expected: `RESULT: True` for each case except `no_parent` with v3 (not in the set). Each node keeps its `lq3-` `id` in `state/topology/topology_canonical_state.json`.

- [ ] **Step 5: Run all checks**

Run: `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` — Expected: all pass.

- [ ] **Step 6: Commit** — `git commit -m "test(scenarios): pin output for modes, failure, guard, and v3"`

---

## After Plan 1

- Update `brain/HANDOFF.md` and add a session note.
- Plan 2 (runtime) starts from `assemble()`. It adds the collector, router states, `state.json`, `status.json`, file writes, `shadow` mode, `diff`, Telegram, and the CLI. Plan 3 adds v3. Plan 4 adds the TUI. Plan 5 adds the installer.
