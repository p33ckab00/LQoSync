# lqosync3 Runtime Implementation Plan (Plan 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `lqosync3` run: read MikroTik routers, keep router states, run the core cycle, write the files safely in `shadow` or `apply` mode, reload LibreQoS 2.x, write `status.json`, and give the CLI `check`, `once`, `run`, `diff`, `defaults`.

**Architecture:** The pure core (`cycle::assemble`) stays unchanged. New modules wrap it: `routeros` (a `RouterSession` trait, snapshot reads, retries), `mikrotik` (the real session on `mikrotik-rs` 0.9.0), `routerstate` (pure classification), `secrets`, `files` (lock, live inputs, seed, atomic writes, backups), `status`, `diff`, `runner` (one full cycle), and `cli` with `src/main.rs`. Tests use a fake session for logic and a mock RouterOS TCP server for the real session.

**Tech Stack:** Rust (`rust-version = "1.89"`), `tokio`, `mikrotik-rs =0.9.0` (`tokio-tls`), `rustls 0.23` (`ring`), `rustls-pemfile 2`, `webpki-roots 1`, plus the Plan 1 crates.

**Spec:** `docs/superpowers/specs/2026-10-31-lqosync3-runtime-design.md` (primary), `docs/superpowers/specs/2026-10-31-lqosync3-core-design.md`, `docs/CONFIGURATION.md`, `config/defaults.toml`, ADR-0003, ADR-0008, ADR-0009.

## Global Constraints

- Repository root `/home/pi/LQoSync3.0`, branch `lqosync3`. `export PATH=$HOME/.cargo/bin:$PATH`. Commit locally after each task. Push only after the final review passes.
- Every default value goes in `config/defaults.toml`. No policy literal in Rust (ADR-0007 follow-up).
- The tool only sends `<path>/print` commands to a router. No command that changes a router.
- No secret in a log line, `status.json`, `state.json`, or an error message.
- `src/cycle.rs`, `src/topology.rs`, `src/sources.rs` do not change, except for a bug that a new test proves.
- Every task ends with `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` green. Use `rtk proxy cargo test` to see the real test counts.
- Code comments follow ASD-STE100.

## Review Focus

1. A router that answers the TCP connect but never answers a command must hit `api_timeout`, be `Failed`, and must not stop the other routers. Test in Task 5.
2. Two `lqosync3` processes on one `state_dir` must not both write. Test in Task 6.
3. A crash between the `state.json` write and the LibreQoS file write must give the same files on the next cycle. Test in Task 8.
4. A `network.json` that is not valid JSON in `preserve` mode must stop the cycle, not wipe the operator tree. Test in Task 6.
5. `SIGHUP` with a bad config must keep the old config. Test in Task 9.

## File Structure

| File | Responsibility |
|------|----------------|
| `config/defaults.toml` | New keys: `router_defaults.tls_verify`, `router_defaults.tls_ca_file` |
| `src/config.rs` | `RouterCfg.tls_verify`, `RouterCfg.tls_ca_file`; `Target::V3*` check moves to `check` |
| `src/secrets.rs` | Read a secret file with mode and owner checks |
| `src/routeros.rs` | `ReadError`, `RouterSession`, `Connector`, `TABLES`, `read_snapshot`, `read_router` |
| `src/routerstate.rs` | `classify` outcome to `RouterResult` + `RouterMemo` |
| `src/mikrotik.rs` | `MikrotikConnector`, `MikrotikSession`, TLS config |
| `src/files.rs` | Lock, live inputs, state load or seed, atomic write, backups, reload command |
| `src/status.rs` | `Status` type and builder |
| `src/diff.rs` | Semantic diff of two pairs |
| `src/runner.rs` | `Runner::cycle` |
| `src/cli.rs`, `src/main.rs` | Argument parse, commands, run loop, signals |
| `systemd/lqosync3.service` | The unit |
| `tests/mock_router.rs` | Mock RouterOS server tests |

---

### Task 1: Dependencies, TLS keys, binary skeleton, `defaults`

**Files:** Modify `Cargo.toml`, `config/defaults.toml`, `src/config.rs`, `src/lib.rs`. Create `src/cli.rs`, `src/main.rs`.

**Interfaces:**
- `RouterCfg { .., tls_verify: bool, tls_ca_file: String }`. Defaults in `[router_defaults]`: `tls_verify = true`, `tls_ca_file = ""`.
- `cli::Command { Check, Once { force: bool }, Run, Diff, Defaults }`, `cli::Args { command: Command, config: PathBuf }`.
- `cli::parse(args: &[String]) -> anyhow::Result<Args>`: `lqosync3 <check|once [--force]|run|diff|defaults> [--config <path>]`. Default config path `/etc/lqosync3/lqosync3.toml`. An unknown word gives an error with the usage text.
- `config::DEFAULTS_TOML` becomes `pub`.
- `main`: `defaults` prints `DEFAULTS_TOML`. Other commands print "not ready" until Task 9.

- [ ] **Step 1: Failing tests**

```rust
// src/cli.rs
#[test] fn parse_commands_and_config_path() {
    let a = parse(&s(&["lqosync3", "once", "--force", "--config", "/tmp/x.toml"])).unwrap();
    assert_eq!((a.command, a.config), (Command::Once { force: true }, PathBuf::from("/tmp/x.toml")));
    assert_eq!(parse(&s(&["lqosync3", "run"])).unwrap().config, PathBuf::from("/etc/lqosync3/lqosync3.toml"));
    assert!(format!("{:#}", parse(&s(&["lqosync3", "wipe"])).unwrap_err()).contains("usage"));
    assert!(parse(&s(&["lqosync3"])).is_err());
}
// src/config.rs
#[test] fn tls_keys_have_defaults_and_can_change() {
    let c = Config::from_toml_str(&format!("{MIN}tls_verify=false\ntls_ca_file=\"/etc/ca.pem\"\n")).unwrap();
    assert_eq!((c.routers[0].tls_verify, c.routers[0].tls_ca_file.as_str()), (false, "/etc/ca.pem"));
    assert!(Config::from_toml_str(MIN).unwrap().routers[0].tls_verify);
}
```

- [ ] **Step 2: Run, see them fail. Step 3: implement. Step 4: run checks.**

Run: `cargo run -q -- defaults | head -3` — Expected: the first three lines of `config/defaults.toml`.

- [ ] **Step 5: Commit** — `feat(cli): add the binary, argument parse, and defaults`

---

### Task 2: Secrets

**Files:** Create `src/secrets.rs`.

**Interfaces:**
- `secrets::read_secret(path: &Path, owner_uid: u32) -> anyhow::Result<String>`: the file must exist, `mode & 0o077 == 0`, `uid == owner_uid`. Remove one final `\n` (and `\r\n`). The error names the path and the problem, never the content.
- Production passes `owner_uid = 0`. Tests pass the current uid (from the metadata of a temp file).

- [ ] **Step 1: Failing tests** — `reads_and_trims_a_good_secret`, `rejects_group_or_other_bits` (mode `0640`), `rejects_wrong_owner` (pass `owner_uid + 1`), `missing_file_names_the_path`, `error_never_contains_the_secret` (content `"s3cret"`, mode `0644`: the error text has no `s3cret`).
- [ ] **Steps 2–4**, then **Commit** — `feat(secrets): read secret files with mode and owner checks`

---

### Task 3: Router state classification

**Files:** Create `src/routerstate.rs`. Create the `ReadError` type in `src/routeros.rs` (only the type in this task).

**Interfaces:**
- `routeros::ReadError { Unreachable(String), Login(String), Tls(String), Timeout(String), Trap(String), Protocol(String) }`, `Display`, `Clone`.
- `routerstate::classify(outcome: Result<RouterSnapshot, ReadError>, memo: &RouterMemo, now: u64, access: &RouterAccess) -> (RouterResult, RouterMemo, Option<String>)`: spec section 4. The `String` is the error text for the status.

- [ ] **Step 1: Failing tests** — `ok_is_online_and_clears_memo`, `login_tls_trap_timeout_are_failed_and_keep_memo`, `first_unreachable_sets_since`, `unreachable_before_offline_after`, `offline_at_offline_after` (since = now − 300, `offline_after = 300`), `offline_after_zero_is_offline_at_once`.
- [ ] **Steps 2–4**, **Commit** — `feat(routerstate): classify router read outcomes`

---

### Task 4: Snapshot reads and retries

**Files:** Modify `src/routeros.rs`.

**Interfaces:**
- `pub trait RouterSession: Send { fn print(&mut self, path: &str, proplist: &str) -> impl Future<Output = Result<Vec<RosRow>, ReadError>> + Send; }`
- `pub trait Connector: Send + Sync { type Session: RouterSession; fn connect(&self, router: &RouterCfg, access: &RouterAccess) -> impl Future<Output = Result<Self::Session, ReadError>> + Send; }`
- `pub const TABLES: [(&str, &str); 6]`: the paths and `.proplist` values of the spec section 3 table, in that order.
- `pub async fn read_snapshot<S: RouterSession>(s: &mut S, router: &RouterCfg, api_timeout: Duration) -> Result<RouterSnapshot, ReadError>`: read only the tables of enabled sources (PPPoE: secret, active, profile. Hotspot: active. DHCP: server, lease). Each call in `tokio::time::timeout`. A timeout is `ReadError::Timeout`.
- `pub async fn read_router<C: Connector>(c: &C, session: &mut Option<C::Session>, router: &RouterCfg, access: &RouterAccess) -> Result<RouterSnapshot, ReadError>`: connect if `None` (in `connect_timeout`; a timeout here is `Unreachable`). On an error: set `*session = None`, sleep `retry_delay`, try again, up to `read_retries` more times. Keep the session after a success.

- [ ] **Step 1: Failing tests** with a `FakeConnector` (scripted results per call):
  - `reads_only_enabled_tables` (PPPoE only: 3 paths asked, in `TABLES` order, each with its `.proplist`).
  - `retry_reconnects_and_succeeds` (first `print` fails, then ok: 2 connects, result ok).
  - `gives_last_error_after_all_retries` (`read_retries = 2`: 3 attempts).
  - `stalled_call_times_out` (a fake `print` that sleeps longer than `api_timeout`; use `tokio::time::pause`).
  - `session_is_kept_between_reads`.
- [ ] **Steps 2–4**, **Commit** — `feat(routeros): read router tables with timeouts and retries`

---

### Task 5: The `mikrotik-rs` session and the mock router

**Files:** Create `src/mikrotik.rs`, `tests/mock_router.rs`.

**Interfaces:**
- `mikrotik::MikrotikConnector { owner_uid: u32 }` implements `Connector`. It reads the password with `secrets::read_secret(password_file, owner_uid)`. `tls = false`: `MikrotikDevice::connect`. `tls = true`: `builder(..).credentials(..)`, then `tls_insecure()` if `tls_verify = false` (log a warning), else `tls_config(tls_client_config(router)?, server_name)`.
- `mikrotik::tls_client_config(router: &RouterCfg) -> Result<Arc<rustls::ClientConfig>, ReadError>`: roots from `tls_ca_file` (PEM, `rustls-pemfile`) or `webpki-roots`.
- Error map: `DeviceError::Connection` (io) gives `Unreachable`, `Authentication` and `Login` give `Login`, `Tls` gives `Tls`, others give `Protocol`. `Event::Trap` gives `Trap(message)`, `Event::Fatal` gives `Protocol`.
- `MikrotikSession::print`: `CommandBuilder::new().command("<path>/print").attribute(".proplist", Some(proplist))`, collect `Event::Reply` attributes (a `None` value becomes `""`) until `Done` or `Empty`.

- [ ] **Step 1: Write the mock router** in `tests/mock_router.rs`: a `TcpListener` on `127.0.0.1:0` that reads API sentences (RouterOS length-prefixed words), answers `/login` with `!done`, and answers `<path>/print` from a script. Each reply echoes the `.tag` word of the command. Then failing tests:
  - `reads_300_rows_sent_in_one_write` (Review Focus: the 0.7.0 bug).
  - `proplist_is_sent` (the mock records the words).
  - `empty_table_is_ok` (`!empty`).
  - `trap_is_a_trap_error`.
  - `wrong_password_is_login_error` (`!trap` on `/login`).
  - `closed_port_is_unreachable` (connect to a port with no listener).
  - `silent_router_times_out_and_does_not_block_another` (Review Focus 1: two routers, one never answers. With `read_router`, the good one is ok in less than `api_timeout × 2`).
- [ ] **Steps 2–4**, **Commit** — `feat(mikrotik): read routers through mikrotik-rs 0.9 with TLS`

---

### Task 6: Files

**Files:** Create `src/files.rs`. Modify `config/defaults.toml` and `src/config.rs` (`runtime.keep_backups`).

**Interfaces:**
- `files::lock(state_dir: &Path) -> anyhow::Result<std::fs::File>`: create the folder if needed, open `lqosync3.lock`, `try_lock`. On `WouldBlock`: "another lqosync3 is running on <state_dir>".
- `files::write_if_changed(path: &Path, bytes: &[u8]) -> anyhow::Result<bool>`: compare with the current bytes. If different: temp file in the same folder, `sync_all`, rename, `sync_all` on the folder. Return `true` if written.
- `files::Live { static_rows: Vec<CsvRow>, other_rows: Vec<CsvRow>, network: Option<serde_json::Value> }` and `files::read_live(lqos_dir: &Path, static_comment: &str, need_network: bool) -> anyhow::Result<Live>`: a missing file is empty. A `network.json` that is not valid JSON is an error when `need_network` (preserve mode or first run).
- `files::load_state(state_dir: &Path, live: &Live, names: &CircuitNames, now: u64) -> anyhow::Result<State>`: read `state.json`, or seed with `seed_state(other_rows)` and `seed_nodes(network)`.
- `files::backup(applied_dir: &Path, sources: &[&Path], stamp: &str, keep: usize) -> anyhow::Result<()>`: copy the existing files into `<applied_dir>/<stamp>/`, then remove the oldest folders above `keep`. `keep` comes from a new key `runtime.keep_backups = 10` in `config/defaults.toml`.
- `files::run_reload(cmd: &str, args: &[String], timeout: Duration) -> Result<(), String>`: `tokio::process::Command`, kill on timeout, the error text holds the exit code and the last 20 lines of stderr.

- [ ] **Step 1: Failing tests** — `second_lock_fails` (Review Focus 2), `write_if_changed_skips_same_bytes`, `write_if_changed_leaves_no_temp_file`, `read_live_splits_static_rows`, `bad_network_json_stops_in_preserve_mode` (Review Focus 4), `load_state_seeds_on_first_run_and_reads_after`, `backup_keeps_the_last_n`, `reload_ok_and_failure_and_timeout` (`/bin/true`, `/bin/false`, `/bin/sleep 5` with 1 s).
- [ ] **Steps 2–4**, **Commit** — `feat(files): add lock, live inputs, atomic writes, backups, reload`

---

### Task 7: Status and diff

**Files:** Create `src/status.rs`, `src/diff.rs`.

**Interfaces:**
- `status::RouterStatus { name, state: &'static str /* online|failed|unreachable|offline */, error: Option<String>, rows: BTreeMap<String, usize>, read_ms: u64 }`.
- `status::Status { version: u32, updated: u64, mode, target, cycle_ms, next_scan_seconds, routers: Vec<RouterStatus>, report: ReportSummary, files_changed: bool, reload: String, error: Option<String> }`, `Serialize`. `ReportSummary` holds the counts and the lists of `cycle::Report` (spec section 7).
- `diff::diff_pair(old_csv: &str, new_csv: &str, old_net: &serde_json::Value, new_net: &serde_json::Value) -> anyhow::Result<String>`: by Circuit Name: `+ name`, `- name`, `~ name: Column old -> new`. Nodes by name at all levels: `+ node`, `- node`, `~ node: field old -> new`. A last line with the counts. Rows with the same name and other device rows compare as a set.

- [ ] **Step 1: Failing tests** — `status_json_has_the_spec_fields` (serialize, check the keys), `diff_reports_added_removed_changed`, `diff_ignores_column_order_and_row_order` (13-column old file against a 14-column new file: only real changes).
- [ ] **Steps 2–4**, **Commit** — `feat(status): add status.json and the pair diff`

---

### Task 8: The runner

**Files:** Create `src/runner.rs`.

**Interfaces:**
- `runner::Runner<C: Connector + 'static> { pub cfg: Config, connector: Arc<C>, sessions: HashMap<String, Option<C::Session>> }`, `Runner::new(cfg, connector: Arc<C>)`. Each router task gets a clone of the `Arc`.
- `Runner::cycle(&mut self, now: u64, force: bool) -> CycleSummary`, `CycleSummary { changed: bool, error: Option<String>, status: Status }`:
  1. `files::lock` (hold for the cycle). 2. `read_live`. 3. `load_state`. 4. Read all routers at the same time (`tokio::task::JoinSet`, one task each; move each session into its task and back). 5. `classify` each. 6. `assemble` with `v3 = false` (`auto` and `v2_pair`). 7. On an error: no writes, status with the error. 8. Write `state.json` (with the new router memos), then the pair (`Columns::V2`) to `candidate/` (`shadow`) or to `<lqos>` after `backup` (`apply`). 9. `apply` and a file changed: `run_reload`. 10. Write `status.json`.
- `runner::CycleSummary::next_sleep(&self, rt: &Runtime) -> u64`: error: `error_retry_interval`. Changed: `scan_interval_active`. Else `scan_interval_idle`.

- [ ] **Step 1: Failing tests** with `FakeConnector` and temp folders (`state_dir` and `lqos_dir` in a tempdir, `v2_command = "/bin/true"`):
  - `shadow_writes_candidates_only` (live files unchanged).
  - `apply_backs_up_writes_and_reloads`.
  - `second_cycle_without_change_writes_nothing` (`changed == false`, file times unchanged).
  - `failed_router_keeps_rows_and_status_says_failed`.
  - `assemble_error_writes_nothing` (two node rules with one name).
  - `restart_after_state_write_gives_same_files` (Review Focus 3: write `state.json` only, then run a new `Runner`: the pair equals the pair of a normal run).
- Add `tempfile = "3"` as a dev-dependency.
- [ ] **Steps 2–4**, **Commit** — `feat(runner): run one full cycle with safe writes`

---

### Task 9: Commands, run loop, signals, systemd

**Files:** Modify `src/cli.rs`, `src/main.rs`, `docs/INSTALLATION.md`, `docs/OPERATIONS.md`. Create `systemd/lqosync3.service`.

**Interfaces:**
- `cli::check(cfg: &Config, owner_uid: u32) -> Vec<CheckLine>`, `CheckLine { level: Level /* Ok|Warn|Error */, text: String }`: spec section 8 `check` list, plus: `target` is `v3_pair` or `v3_network` gives `Error "LibreQoS v3 targets come in Plan 3"`.
- `cli::load_config(path: &Path) -> anyhow::Result<Config>`.
- `main`: `check` prints the lines and exits 1 on any `Error`. `once` runs one cycle and prints the status summary. `diff` prints `diff_pair` for candidate against live. `run` loops: `tokio::select!` on the sleep, `SIGUSR1` (cycle with `force = true`), `SIGHUP` (reload config: on error keep the old one and log it), `SIGTERM`/`SIGINT` (stop after the cycle).
- `systemd/lqosync3.service`: `Type=simple`, `ExecStart=/usr/local/bin/lqosync3 run --config /etc/lqosync3/lqosync3.toml`, `ExecReload=/bin/kill -HUP $MAINPID`, `Restart=on-failure`, `RestartSec=10`, `User=root`, `NoNewPrivileges=yes`, `ProtectSystem=full`, `ReadWritePaths=/var/lib/lqosync3 /opt/libreqos/src`.

- [ ] **Step 1: Failing tests** — `check_finds_plain_api_and_no_source_router`, `check_rejects_v3_target_for_now`, `reload_keeps_old_config_on_error` (Review Focus 5: a `reload(&mut cfg, path)` helper that returns the error and keeps `cfg`).
- [ ] **Step 2–4**, then **Step 5 (manual, operator approval needed):** on the shaper, create a read-only RouterOS user, write `/etc/lqosync3/lqosync3.toml` with `mode = "shadow"`, run `lqosync3 check`, `lqosync3 once`, `lqosync3 diff`. Expected: `check` has no `Error`, `once` gives status `online` for the router, `diff` shows only the known ADR-0007 differences.
- [ ] **Step 6: Commit** — `feat(cli): add check, once, run, diff, signals, and the systemd unit`

---

## After Plan 2

- Update `brain/HANDOFF.md`, add a session note, refresh the graph.
- Plan 2b: Telegram. Plan 3: LibreQoS v3. Plan 4: TUI. Plan 5: installer.
