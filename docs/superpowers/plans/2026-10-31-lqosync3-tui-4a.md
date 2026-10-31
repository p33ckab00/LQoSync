# lqosync3 TUI Implementation Plan (Plan 4a: service changes and read-only screens)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `lqosync3 tui` with five tabs (ShapedDevices, Network, Settings placeholder, Journal, Dashboard), the actions "sync now" and "reload", and the service data that the TUI and Plan 4b need.

**Architecture:** The service writes more fields to `status.json` and a new `snapshot.json`. The TUI is a synchronous loop in the main thread. Two `std::thread` workers (a file and process loader, a journal reader) send updates over `std::sync::mpsc`. The draw code reads only memory. All parsing, IO, and process calls are in the workers.

**Tech Stack:** Rust 1.89, `ratatui` 0.30.2 (MSRV 1.88). Use the re-export `ratatui::crossterm` (crossterm 0.29 through `ratatui-crossterm` 0.1.2), not a direct `crossterm` dependency. `libc` 0.2 (already in `Cargo.lock` as a transitive dependency; add it as a direct dependency).

**Spec:** `docs/superpowers/specs/2026-10-31-lqosync3-tui-design.md`

## Global Constraints

- All defaults in `config/defaults.toml`. New `[tui]` keys in this plan: `refresh_ms = 1000`, `journal_lines = 2000`, `max_file_mb = 50`, `osc52 = true`, `service_unit = "lqosync3.service"`, `lqosd_units = ["lqosd.service", "libreqos-lqosd.service"]`. The keys `apply_verify_timeout` and `preview_max_age` come with Plan 4b, because nothing in 4a uses them.
- The production service operates in `apply` mode. The service changes must not change `ShapedDevices.csv`, `network.json`, or `libreqos_network.json`. Guard: `tests/scenarios.rs` passes unchanged (do not set `LQOSYNC3_BLESS`), and all runner tests pass.
- The service must operate the same with no TUI. A write error of `snapshot.json` or of the extra status write is only a log line. It never stops a cycle.
- No test connects to a router or to `/run/lqos/bus`, sends a signal to a real process, or writes outside a temporary folder.
- Docs and comments in ASD-STE100.

**Performance (owner request: "optimized for highest performance"):**
- `[profile.release]`: `lto = "fat"`, `codegen-units = 1`. Keep `panic = "unwind"`: the runner catches a panic in a router task (`runner.rs`, "A task that panics fails only its own router").
- The draw code does no IO, no parsing, and no sort. It reads data that the workers prepared.
- Draw only when something changed (key, mouse, resize, new data), and one time each second for the clocks. Wait with `event::poll(timeout)`, never a busy loop.
- A table draws only the visible rows: build `Row` values for `offset..offset + height`, not for all rows.
- Search and sort work on an index vector (`Vec<u32>`) into shared data (`Arc<...>`). Keep a lowercase search text for each row, made one time when the data loads. A numeric sort computes the `f64` keys one time for each sort, not in each comparison.
- The loader parses a file only when its modification time or size changed.
- All buffers have a bound: `journal_lines`, `max_file_mb`, 120 samples for each graph.
- Measure: an `#[ignore]` release test times one key press (rebuild and a `TestBackend` draw) on 20 000 rows. Limit 16 ms.

## Review Focus

1. A process name with spaces or `)` in `/proc/<pid>/stat` (the `comm` field): parse after the last `)`.
2. The MainPID changes (the service restarted): the TUI sends a signal only to the current MainPID from systemd, after a `comm` check, and never to its own PID.
3. 20 000 rows in `ShapedDevices.csv`: the key-press-to-draw time stays under 16 ms in a release build.
4. `journalctl` cannot be read (no `adm` group, or no `journalctl`): the Journal tab shows the error text, and the TUI does not stop. No stderr text draws over the screen.
5. No `status.json` (the service never ran) or a `status.json` with the old version 2 fields: each value shows `unknown`.

---

### Task 1: Service data for the TUI

**Files:** `src/status.rs`, `src/runner.rs`, `src/main.rs`, `src/cli.rs`, `src/files.rs`, `src/model.rs`, `src/notify.rs` (the test helper `st()`).

**Produces:**
- `pub enum Trigger { Start, Timer, SyncNow, Reload, Once }` in `status.rs`, `Serialize`/`Deserialize` as snake_case, `Default` = `Start`. Ruling: `Once` is added for `lqosync3 once`; the spec lists four values.
- `Status` (and `RouterStatus`) derive `Default`. `Status` gets `config_path: String`, `config_digest: String`, `cycle_seq: u64`, `pid: u32`, `trigger: Trigger`, `program_version: String` (`env!("CARGO_PKG_VERSION")`), `telegram: String`. `version` becomes `3`. The struct literals in `status.rs` (test) and `notify.rs` (`st()`) use `..Default::default()`.
- `pub fn config_digest(bytes: &[u8]) -> String` in `cli.rs`: `format!("{:016x}", topology::fnv1a64(bytes))`.
- `pub fn read_config(path: &Path) -> anyhow::Result<(Config, String)>` in `cli.rs` (the config and the digest of the same bytes). `load_config` calls it. `reload(cfg, path)` becomes `reload(cfg: &mut Config, digest: &mut String, path: &Path)`. Update the test `reload_keeps_old_config_on_error` (`cli.rs:327`, `cli.rs:330`): the digest also stays on an error.
- `Runner` gets `pub trigger: Trigger` (main sets it before each cycle), and the private fields `config_path`, `config_digest`, `cycle_seq`, `telegram`, `last: Option<Status>`. Ruling: `Runner::cycle(now, force)` keeps its signature, so the 21 test calls and `main.rs` do not change (review finding 3).
- `Runner::set_source(&mut self, path: &Path, digest: String)`: call at start and after a good reload.
- `Runner::set_telegram(&mut self, result: String)`: store it; if it is different from `last.telegram`, write the status again. Values: `"disabled"` (Telegram off), `"none"` (no send in this process; the initial value), `"ok"`, or the error text. Ruling: the extra write removes the one-cycle lag of review finding 9.
- `Runner::write_reload_status(&mut self, now: u64)`: write `last` again with `trigger = reload`, `updated = now`, the current digest, and `config_error`. `cycle_seq` stays. If no cycle ran yet, write a status with only these fields, `pid`, `program_version`, and `config_path`.
- `files::write_snapshot(state_dir: &Path, routers: &[(String, RouterResult)], read_ms: &[u64], now: u64) -> anyhow::Result<()>`: `<state_dir>/snapshot.json`, JSON `{"version":1,"taken":now,"routers":[{"name","state","read_at","read_ms","snapshot"}]}` (`snapshot` is `null` if the router is not `online`). Open the temporary file `snapshot.json.tmp` with `OpenOptionsExt::mode(0o600)`, write, `set_permissions(0o600)` (an old temporary file keeps its mode), then rename. No fsync and no compare-read: `taken` changes each cycle, and the file is a cache. Ruling: `write_if_changed` is not used, because its temporary file is world-readable before the mode copy (review finding 2). `RouterSnapshot` gets `Serialize`.
- The cycle calls `write_snapshot` after the router loop and before `assemble` (`routers` moves into `CycleInput`). An error gives `eprintln!("warning: cannot write snapshot.json: {e:#}")`.
- `check(cfg, owner_uid, digest: &str)`: a `Warn` line "the config file on disk is not the loaded config (status.json has <x>)" if `<state_dir>/status.json` has a `config_digest` that is not empty and is different from `digest`.
- `main.rs`: one `read_config` before the `match` gives `(cfg, digest)` for `check`, `once`, and `run`. `run` sets `trigger = Start` for the first cycle, then `Timer`, `SyncNow` after `SIGUSR1`, `Reload` after `SIGHUP`. After `SIGHUP` (good or bad), call `write_reload_status(now())` before the next cycle. `once` sets `Once`. `notify()` returns the `telegram` value, and main calls `set_telegram` with it.

- [ ] Tests (`runner.rs`):
  - `status_has_digest_seq_pid_trigger`: two cycles give `cycle_seq` 1 and 2, `trigger` as set, `pid == std::process::id()`, `config_digest` as set, `version == 3`.
  - `reload_status_keeps_the_cycle_seq`: after a cycle, `config_failed("x")` then `write_reload_status(2000)` gives a `status.json` with `trigger = "reload"`, `config_error = "x"`, `updated = 2000`, and `cycle_seq = 1`.
  - `reload_status_without_a_cycle`: `write_reload_status` on a new runner writes `trigger = "reload"` and `cycle_seq = 0`.
  - `telegram_result_is_written_at_once`: after a cycle, `set_telegram("ok")` changes `status.json` to `telegram = "ok"`.
  - `snapshot_json_has_each_router_and_mode_0600`: `online` with its tables, `failed` with `null`; mode `0o600`, also when an old `snapshot.json.tmp` with mode `0o644` exists.
  - `snapshot_write_error_does_not_stop_the_cycle`: a folder named `snapshot.json.tmp` in `state_dir`; the cycle still writes the candidate files.
- [ ] Tests (`cli.rs`): `config_digest_is_stable_and_16_hex`; `check_warns_when_the_file_is_not_loaded`; `check_has_no_warning_without_status_json`.
- [ ] Run the whole suite: `cargo test`. `tests/scenarios.rs` passes without a change to its expected files.
- [ ] Commit: `feat(status): add config digest, cycle data, and snapshot.json`.

### Task 2: TUI frame

**Files:** create `src/tui/mod.rs`, `src/tui/app.rs`; modify `Cargo.toml`, `config/defaults.toml`, `docs/CONFIGURATION.md` (`tests/config_doc.rs` checks that each example key has the default value), `src/config.rs` (`pub struct Tui`, `deny_unknown_fields`), `src/cli.rs` (`Command::Tui`, `"tui"` in `parse` and in `USAGE`), `src/main.rs`, `src/lib.rs`.

**Produces:**
- `pub enum Tab { Devices, Network, Settings, Journal, Dashboard }` with `Tab::ALL` in tab order and `title()`.
- `pub enum Action { None, Quit, SyncNow, Reload, Copy(String), SelectMode(bool) }`.
- `pub struct App` with `tab`, `read_only: bool`, `message: Option<String>`, `modal: Option<Modal>`, and one view state for each tab. `App::on_key(&mut self, KeyEvent) -> Action`, `App::on_mouse(&mut self, MouseEvent) -> Action`, `App::draw(&mut self, &mut Frame)`. `draw` stores the tab title rectangles for the mouse hit test.
- Layout: header (service state, mode, target, last cycle age, next scan, `READ-ONLY (not root)` if not root), tabs, main area, footer (the keys of the current tab). `fn split_main(area: Rect) -> (Rect, Rect)`: the side panel is at the right if `width >= 100`, else under the main panel. Each state color comes with its word (`online`, `failed`).
- `pub fn run(config: &Path) -> anyhow::Result<()>` in `tui/mod.rs`:
  - `ratatui::try_init()` (`init` panics when there is no TTY), then `EnableMouseCapture`.
  - A `TerminalGuard` whose `Drop` sends `DisableMouseCapture` and calls `ratatui::restore()`. So an `Err` from `?` also restores the terminal.
  - `fn install_panic_hook(restore: fn())`: the hook restores the terminal only on the thread that installed it, then calls the old hook. A panic in a worker does not touch the terminal. The main loop sees the closed channel, joins the worker, and shows its panic text as a message.
- `main.rs`: dispatch `Command::Tui` before the `read_config` line, as `Defaults` is. Call `tui::run` directly (no `block_in_place`: no other tokio task runs). The TUI reads the config with `read_config`. If the read fails, the header shows the error, and the TUI uses `config::defaults()`.
- Read-only mode: `unsafe { libc::geteuid() } != 0`. The actions and Plan 4b edits are blocked with a message.
- The Settings tab in 4a shows the config file text, read-only, with the note "editor: Plan 4b". Ruling: the tab exists so the tab order of the spec stays.

- [ ] Tests (`app.rs`, with `ratatui::backend::TestBackend`):
  - `keys_1_to_5_select_the_tabs_and_q_quits`.
  - `a_click_on_a_tab_title_selects_it` (draw one time, then a `MouseEventKind::Down(MouseButton::Left)` inside the stored rectangle).
  - `read_only_blocks_sync_now`.
  - `draws_on_a_40x10_terminal_without_a_panic`.
  - `narrow_terminal_puts_the_side_panel_under_the_main_panel` (`split_main` at width 80 and 120).
  - `panic_hook_restores_only_on_its_own_thread` (a counter `restore`; a panic in `catch_unwind` on the test thread gives 1; a panic in a spawned thread keeps 1).
  - `tui_command_parses` (`cli.rs`).
- [ ] Commit: `feat(tui): add the TUI frame and tabs`.

### Task 3: Loader worker

**Files:** create `src/tui/data.rs`, `src/tui/proc.rs`.

**Produces:**
- `pub struct Data` (all `Arc` for large parts): `status: Option<serde_json::Value>`, `service: Option<Unit { load: String, active: String, main_pid: u32 }>`, `rows: Arc<Vec<CsvRow>>`, `row_search: Arc<Vec<String>>`, `flags: Arc<Vec<u8>>`, `sources: Arc<HashMap<String, (String, String)>>` (circuit name to source and router, from `state.json`), `tree: Arc<serde_json::Value>`, `node_circuits: Arc<HashMap<String, Vec<u32>>>` (parent name to row index), `config_text: Arc<String>`, `versions: Versions`, `proc: Option<ProcInfo>`, `errors: Vec<String>`.
- `pub fn spawn_loader(cfg: &Config, config_path: PathBuf, tx: mpsc::Sender<Data>) -> std::thread::JoinHandle<()>` (thread name `tui-loader`). Each `refresh_ms`: read `status.json`; run `systemctl show -p LoadState -p ActiveState -p MainPID <service_unit>`; read the files whose modification time or size changed; sample the process; each 60 s read the versions. Send a new `Data` only if something changed. Command arguments as a list.
- Target: the `target` of `status.json`, else `detect::resolve(&cfg.libreqos)`. Files: the pair from `libreqos.directory` (`apply`) or `<state_dir>/candidate` (`shadow`); read the CSV with `state::read_csv`. For `v3_network`, read `libreqos_network.json` and convert it with `fn rows_from_libreqos_network(&Value) -> anyhow::Result<(Vec<CsvRow>, Value)>`: the parent node name comes from the node `id`; the tree is rebuilt from `parent`. A file larger than `max_file_mb` gives an error in `errors` and keeps the last good data.
- `pub struct Versions { libreqos: Option<String>, lqosd: Option<(String, String)>, }`: read the text of `libreqos.dpkg_status`, then `detect::package_version(&text, "libreqos")`. `lqosd`: the first unit of `lqosd_units` with `LoadState=loaded`, and its `ActiveState`.
- `proc.rs`: `pub fn parse_stat(text: &str) -> Option<Stat { comm: String, utime: u64, stime: u64, starttime: u64 }>` (split after the last `)`; after it, token 0 is the state, `utime` is index 11, `stime` 12, `starttime` 19); `pub fn parse_rss_kb(status: &str) -> Option<u64>` (`VmRSS:`); `pub fn boot_time(proc_stat: &str) -> Option<u64>` (`btime`); `pub fn cpu_percent(prev_ticks: u64, ticks: u64, secs: f64, clk_tck: u64) -> f64`; `pub fn clk_tck() -> u64` (`libc::sysconf(libc::_SC_CLK_TCK)`); `pub fn parse_unit(show: &str) -> Unit`.
- `pub struct ProcInfo { pid: u32, start_unix: u64, elapsed_s: u64, rss_kb: u64, cpu_pct: Option<f64> }` for the MainPID. MainPID 0, or `/proc/<pid>/comm` not `lqosync3`: `proc` is `None`. `cpu_pct` is `None` for the first sample.
- Flags: `validate::rate_problem(row: &CsvRow) -> Option<String>` (moved from `validate::check`, which calls it) sets the rate bit. The loader also sets the duplicate IPv4 bit and the "parent not in the tree" bit.

- [ ] Tests: `parse_stat_handles_spaces_and_parens_in_comm` (fixture `"123 (a) b) S 1 ... "`), `rss_and_btime_parse`, `cpu_percent_math` (100 ticks in 1 s at 100 Hz = 100.0), `parse_unit_reads_main_pid`, `rows_from_libreqos_network_round_trip` (input: `render_libreqos_network` output; compare Circuit ID, Circuit Name, Device ID, Device Name, MAC, IPv4, IPv6, Parent Node, Comment as text, and the four rates as numbers), `loader_skips_unchanged_files` (the same `Arc` pointer after a second load), `too_large_file_keeps_last_data`, `flags_mark_duplicate_ip_missing_parent_and_low_rate`. `validate.rs` tests pass unchanged.
- [ ] Commit: `feat(tui): add the file and process loader`.

### Task 4: ShapedDevices tab

**Files:** create `src/tui/devices.rs`.

**Produces:** `pub struct DevicesView { query: String, sort: (Column, bool), index: Vec<u32>, selected: Option<(String, String)>, offset: usize }` (`selected` is Circuit ID and Device ID, so the selection stays after a refresh). `fn rebuild(&mut self, data: &Data)` (filter and sort; called only when the data, the query, or the sort changes). The table columns of spec 6.1, plus source and router from `sources`. The detail panel: all fields of the row, the source, the router, and the flag words.

- [ ] Tests:
  - `search_matches_name_ip_and_mac_case_insensitive`.
  - `sort_by_download_max_is_numeric` (`"9"` before `"10"`).
  - `selection_stays_on_the_same_circuit_after_a_refresh_with_new_rows`.
  - `draw_builds_rows_only_for_the_visible_window` (a counter in the row builder: 20 000 rows, height 30, at most 30 rows built).
  - `enter_opens_the_parent_node_in_the_network_tab` (returns the node name; the App selects the Network tab and calls `reveal`).
  - `devices_tab_renders_flags_as_words`.
  - `perf_20k_rows_key_press_under_16ms` (`#[ignore]`; run with `cargo test --release -- --ignored perf`).
- [ ] Commit: `feat(tui): add the ShapedDevices tab`.

### Task 5: Network tab

**Files:** create `src/tui/network.rs`.

**Produces:** `pub struct NetworkView { expanded: HashSet<String>, visible: Vec<(u16, String)>, selected: usize, query: String }`. `fn rebuild(&mut self, tree: &Value)` (flatten only the expanded nodes; called on a toggle or new data). `fn reveal(&mut self, tree: &Value, name: &str)` (expand the path to `name` and select it). The detail panel: type, `id`, rates, number of child nodes, and the circuits under the node (from `node_circuits`).

- [ ] Tests: `collapsed_tree_shows_only_roots`, `space_expands_and_collapses`, `expand_all_and_collapse_all`, `reveal_expands_the_path`, `node_details_count_circuits`, `search_selects_the_first_match`, `network_tab_renders`.
- [ ] Commit: `feat(tui): add the Network tab`.

### Task 6: Journal tab

**Files:** create `src/tui/journal.rs`, `src/tui/osc52.rs`.

**Produces:**
- `pub fn spawn_journal(program: &str, unit: &str, lines: usize, tx: mpsc::Sender<JournalMsg>) -> JournalHandle`: `<program> -u <unit> -n <lines> -o short-iso --no-pager -f` (main passes `"journalctl"`). Stdout is read with `BufReader::lines` in thread `tui-journal`. Stderr is piped and read in a second thread; each stderr line is sent as `JournalMsg::Error(text)` (journalctl prints the `adm` group hint on stderr and keeps running). A start error or an end of the child also sends `JournalMsg::Error`. `JournalHandle` kills and waits for the child in `Drop`. Ruling: the `program` parameter replaces a `PATH` change in the test (review finding 12).
- `pub struct JournalView { lines: VecDeque<String> (max journal_lines), level: Level, query: String, follow: bool, sel: Option<(usize, usize)>, offset: usize, error: Option<String> }`. Level from the text: `error:` or `ERROR` = error; `warning:` or `ALERT` = warning; else info.
- Selection: a mouse press and drag, or `Shift` with `Up`/`Down`, sets `sel`. `y` copies the selected lines (or the line under the cursor).
- `pub fn osc52(text: &str) -> String` (`ESC ] 52 ; c ; <base64> BEL`, own base64 encoder). Copy at most 64 KiB; above that, show a message. `y` sends it when `tui.osc52 = true`, then shows "sent to the terminal clipboard (not confirmed); if it did not arrive, press s and select with the terminal". `s` toggles select mode (`DisableMouseCapture` / `EnableMouseCapture`).

- [ ] Tests: `base64_matches_known_values` (`""`, `"f"` -> `Zg==`, `"fo"` -> `Zm8=`, `"foo"` -> `Zm9v`, `"hi"` -> `aGk=`), `osc52_frame`, `buffer_keeps_the_last_n_lines`, `level_filter`, `selection_text_is_the_selected_lines`, `shift_down_extends_the_selection`, `journal_start_error_is_shown` (program `/nonexistent/journalctl`: the view gets `Error`), `journal_stderr_is_shown` (program: a script in a temporary folder that prints a hint to stderr, then sleeps), `journal_tab_renders`.
- [ ] Commit: `feat(tui): add the Journal tab`.

### Task 7: Dashboard tab and actions

**Files:** create `src/tui/dashboard.rs`; modify `src/tui/app.rs`.

**Produces:**
- Cards from `status.json` and `Data`: service state (systemd `ActiveState`), mode, target, last cycle (time, `cycle_ms`, `trigger`, error), routers (state word, rows, `read_ms`), counts, guard, last Telegram send, reload, `config_digest` check ("loaded" if the digest of `config_text` equals `config_digest`, else "the file on disk is not loaded"), versions (`program_version` of the service and `CARGO_PKG_VERSION` of the TUI; different values give "service needs a restart"; LibreQoS package and `lqosd` state), process (MainPID, start, elapsed, RSS, CPU). A missing value shows `unknown`, never `0`.
- Two `Sparkline` graphs (CPU percent, RSS) from at most 120 samples of this session.
- `fn signal_target(main_pid: u32, own_pid: u32, proc_comm: impl Fn(u32) -> Option<String>) -> Result<u32, String>`: reject `0`, reject `own_pid`, require `comm == "lqosync3"`. The PID comes from `systemctl show -p MainPID` (spec 6.3, 6.5), not from `status.json`.
- `fn wait_result(sent: &Sent, status: &Value) -> Wait` with `Wait { Pending, Done(String), Restarted }`:
  - `S` (after the confirmation modal and `y`): `SIGUSR1` with `libc::kill`. Done when `pid` equals the PID at the send, `cycle_seq` is higher, and `trigger = "sync_now"`. A different `pid`: `Restarted`.
  - `R`: `SIGHUP`. Done when `trigger = "reload"` and `updated >= ` the send time (seconds). Ruling: the time check stops an old reload status from matching at once (review finding 10).
  - After 120 s without `Done`, the message shows "no result from the service".

- [ ] Tests: `unknown_values_never_show_zero` (a status of version 2 without the new fields), `digest_check_shows_not_loaded`, `version_difference_says_restart`, `signal_target_rejects_zero_own_pid_and_other_comm`, `sync_now_waits_for_a_higher_cycle_seq_of_the_same_pid`, `reload_ignores_an_old_reload_status`, `dashboard_tab_renders`.
- [ ] Commit: `feat(tui): add the Dashboard tab and actions`.

### Task 8: Docs, review, graph

- [ ] `docs/OPERATIONS.md`: `lqosync3 tui` in the command table, the keys, read-only mode, and select mode. `docs/CONFIGURATION.md`: the `[tui]` keys. `brain/HANDOFF.md`.
- [ ] Release check: `cargo test --release -- --ignored perf` passes, and `lqosync3 tui --config <copy>` starts on a copy of the production files.
- [ ] Whole-branch review by a fresh reviewer, then graphify update, then commit and push.

## Review rulings (fact-check of 2026-10-31)

- Finding 1: the byte-fixture test is removed. The new IDs are random. `tests/scenarios.rs` is the guard.
- Finding 2: `write_snapshot` makes its own `0600` temporary file.
- Findings 3 to 13, 15, 16, 19 to 21: fixed in the tasks above.
- Finding 14: the rate rules move to `validate::rate_problem`, and both callers use it.
- Finding 17: the header, footer, narrow layout, state words, detail panel, mouse and `Shift` selection, full OSC 52 text, render tests, panic test, and the read time for each router are in the tasks.
- Finding 18: the wording now says that `tests/config_doc.rs` checks the example values. It does not check that each default key is in the example.
