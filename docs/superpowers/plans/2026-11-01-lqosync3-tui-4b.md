# lqosync3 TUI Implementation Plan (Plan 4b: the Settings editor)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the read-only Settings tab with an editor for all config keys and routers: edit, validate, preview, confirm, apply, verify (spec 6.3), with the secrets rules of the spec.

**Architecture:** The editor holds a `toml_edit::DocumentMut` of the user file, so comments and key order stay. Key metadata comes from `config/defaults.toml` (the embedded text) plus a short list of optional keys and a table of the 10 enums. Validate is `Config::from_toml_str` and the `lqosync3 check` rules. Preview runs `cycle::assemble` two times on `snapshot.json` in a worker thread and shows `diff::diff_pair` text. Apply writes the secrets, then the TOML file, then sends `SIGHUP`, and verify waits for the new `config_digest` in `status.json`.

**Tech Stack:** Rust 1.89, `ratatui` 0.30.2 (already in), `toml_edit` 0.25 (new; the 4a spike compiled 0.25.17 on this host), `toml` 0.9 (already in).

**Spec:** `docs/superpowers/specs/2026-10-31-lqosync3-tui-design.md` (sections 2, 3, 6.3, 7, 9, 10, 11). Plan 4a: `docs/superpowers/plans/2026-10-31-lqosync3-tui-4a.md`.

## Global Constraints

- All defaults in `config/defaults.toml`. New key in this plan: `[tui] preview_max_age = 300` (seconds). `apply_verify_timeout = 120` exists (4a ruling).
- The service must operate the same with no TUI. A change to the service code is a refactor only: `tests/scenarios.rs` passes without `LQOSYNC3_BLESS`, and all runner tests pass.
- Anti-goals (spec 2): no apply without a preview and a confirmation; no "success" without the new `config_digest` in `status.json`; no secret on the screen, in the clipboard, in a log, in the undo history text, or in the preview; no TUI calculation that copies cycle code.
- A password goes only into its `*_file` with mode `0600` and owner root, never into the TOML file.
- `runtime.state_dir` is read-only in the editor. The TUI never restarts the service.
- Editing, preview, and apply need root. In read-only mode (4a) the Settings tab shows the values and blocks each edit with a message.
- No test connects to a router or to `/run/lqos/bus`, sends a signal to a real process, or writes outside a temporary folder.
- Docs and comments in ASD-STE100.
- Performance: the draw code does no IO and no parse. `parse` (one `from_toml_str`, about 1 ms, no IO) runs on a commit of a field, not on each key press. `validate` (it has IO) and the preview run in a worker thread.

## Review Focus

1. The config file changes on disk between the start of the edit and the apply (a hand edit, a second TUI): the apply stops and shows the new diff. Nothing is written.
2. The apply stops after a secret write and before the TOML write (disk full, permission): a new secret file is removed, a replaced secret file gets its old bytes back.
3. The service does not load the new file (the service rejects it, or the service is down): the TUI never shows "applied"; it shows `config_error`, or "not verified" with the two digests.
4. A typed secret appears in the screen buffer, the undo history, the TOML text, the preview, a message, or a panic text: never.
5. A minimal user file (only `[[routers]]`, no other section): a set adds the key in the right table, a reset removes the key and an empty table that the editor made, and the other comments and the key order stay.

---

### Task 1: Key metadata

**Files:** create `src/tui/settings/mod.rs`, `src/tui/settings/meta.rs`; modify `src/tui/mod.rs` (`pub mod settings;`), `Cargo.toml` (`toml_edit = "0.25"`), `config/defaults.toml` and `docs/CONFIGURATION.md` (`preview_max_age`), `src/config.rs` (`Tui.preview_max_age: u64`, range 10 to 86400). `runtime.keep_backups` help: "backup folders of the old LibreQoS files, and config backups of the TUI, to keep".

**Produces:**
- `pub enum Kind { Bool, Int, Float, Text, Choice(&'static [&'static str]), List(Box<Kind>) }`.
- `pub struct KeyMeta { pub path: String, pub kind: Kind, pub default: Option<String>, pub help: String, pub optional: bool, pub read_only: bool }`. `path` uses `routers.*` and `routers.*.dhcp.servers.*` for list items.
- `pub fn all() -> &'static [KeyMeta]` (`OnceLock`): walk `DEFAULTS_TOML` with `toml_edit::DocumentMut`.
  - Kind: `Bool` and `Text` from the value type; a number is `Float` if its path is in `FLOATS`, else `Int` (many `f64` keys have an integer default, for example `safety.max_removed_percent = 20`); an empty array is `List(Text)`; `Choice` and `List(Choice)` from `CHOICES`. `router_defaults.dhcp.servers` is not a key: the DHCP servers are a group of the router (Task 6).
  - `default`: the value text without its decor (`Value::to_string()` holds the spaces and the comment; use `value.clone().decorated("", "")`).
  - Help: the trailing comment is `value.decor().suffix()`; the comment lines above a key are `key.leaf_decor().prefix()` (`Table::key(k)`). Remove `#` and the spaces. Comment lines after the last key of a table belong to the prefix of the next table header: ignore a table header prefix.
  - Both `router_defaults.<k>` and `routers.*.<k>` get metadata (the user file can set `[router_defaults]`); `routers.*.<k>` has the default of `router_defaults.<k>`.
  - Add the required router keys `routers.*.name`, `address`, `username`, `password_file` (`Text`, help text in the code: the example block is the trailing text of the document and is not walked). `password_file` default for a new router: `/etc/lqosync3/secrets/<name>.password` (spec 6.3).
  - Add the optional keys with a help text in the code, each `optional: true`: `routers.*.hotspot.download_limit_mbps`, `upload_limit_mbps`, `download_factor`, `upload_factor` (`Float`); `nodes.ppp.type`, `nodes.ppp_plan.type`, `nodes.hotspot.type`, `nodes.dhcp_site.type`, `nodes.dhcp_plan.type` (`Text`; `nodes.client.type` is required and is in the defaults file); `routers.*.dhcp.servers.*.name` (`Text`, required), `.mode` (`Choice`, default from `server_mode`), `.speed_sources` (`List(Choice)`), `.default_plan_down_mbps`, `.default_plan_up_mbps`, `.download_factor`, `.upload_factor` (`Float`).
  - `runtime.state_dir`: `read_only: true`, help "change it by hand; copy the folder first".
- `pub const FLOATS: &[&str]`: the paths of the `f64` and `Option<f64>` fields of `config.rs`.
- `pub const CHOICES: &[(&str, &[&str])]` for the 10 enums: `runtime.mode`, `runtime.log_level`, `router_access.offline_action`, `libreqos.target`, `network.mode`, `speed.rate_limit_order`, `speed.unitless_unit`, `speed.pppoe_sources` (PppSource), `speed.dhcp_per_site_sources` and `speed.dhcp_per_plan_sources` and `routers.*.dhcp.servers.*.speed_sources` (DhcpSource), `router_defaults.dhcp.server_mode`, `routers.*.dhcp.server_mode` and `routers.*.dhcp.servers.*.mode` (DhcpMode).
- `pub fn find(path: &str) -> Option<&'static KeyMeta>` (a router index in `path` matches `*`).
- Ruling (spec gap, spec 6.3 asks for the range in the help): the range of a number is not a table. The help shows the range text of the defaults comment if it has one; for the other keys, the validate error of `Config::validate` gives the range. So no range rule is copied.

- [ ] Tests (`meta.rs`):
  - `each_default_key_has_meta`: each leaf key of `DEFAULTS_TOML` (outside the comment block) has a `find` result.
  - `each_enum_choice_loads_and_a_wrong_value_fails`: for each `CHOICES` entry and each choice, `Config::from_toml_str` of a minimal file with that value is `Ok`; the value `"wrong"` is `Err` and the error names the key (a router key appears as `routers[0]....` in the error). 10 enum types are covered. `lease_comment` is not allowed for per-site sources (config.rs `validate`): test it only with `mode = "per_plan"` for a server, and expect `Err` for `speed.dhcp_per_site_sources`.
  - `number_kinds_match_the_config_types`: for each numeric key, `<key> = 1.5` gives a type error ("invalid type") only if its kind is `Int`.
  - `help_comes_from_the_comment`: `find("runtime.mode").help` contains `shadow | apply`; `find("runtime.id_length").help` contains `4 to 32`.
  - `state_dir_is_read_only` and `router_keys_use_router_defaults` (`find("routers.*.port").default == Some("8729")` and `find("router_defaults.port")` exists).
  - `tui_preview_max_age_default` in `config.rs`.
- [ ] Commit: `feat(tui): add the settings key metadata`.

### Task 2: The draft (edit model)

**Files:** create `src/tui/settings/draft.rs`.

**Produces:**
- `pub enum Seg { Key(String), Index(usize) }`, `pub type KeyPath = Vec<Seg>`, `pub fn path_text(p: &KeyPath) -> String` (`routers.0.pppoe.enabled`), `pub fn meta_path(p: &KeyPath) -> String` (`routers.*.pppoe.enabled`).
- `pub struct Draft { original: String, pub start_digest: String, doc: toml_edit::DocumentMut, made: HashSet<String>, undo: Vec<(String, HashSet<String>)> }`. `made` holds the tables that the editor made. `undo` holds the TOML text and `made` before each change (no secret is ever in the TOML text).
- `Draft::new(text: &str) -> anyhow::Result<Draft>`; `get(&self, p) -> Option<toml_edit::Item>` (the value in the file, `None` = default); `set(&mut self, p, v: toml_edit::Value)`; `reset(&mut self, p)` (remove the key; remove a table that becomes empty only if the editor made it); `add_router(&mut self, name, address, username, password_file) -> usize`; `remove_router(&mut self, i)`; `add_server(&mut self, router: usize, name) -> usize`; `remove_server(&mut self, router, i)`; `list_push`, `list_remove`, `list_move(&mut self, p, from, to)`; `undo(&mut self) -> bool`; `text(&self) -> String`; `changed(&self) -> Vec<String>` (the `path_text` of each leaf that differs from `original`, also added and removed leaves); `diff_lines(&self) -> Vec<(char, String)>` (`' '`, `'-'`, `'+'` lines; an LCS line diff, the files are small).
- `set`: if a table on the path does not exist, insert `Item::Table(Table::new())` for it first (and mark a parent that has only subtables as implicit, `set_implicit(true)`), then set the key. Do not use `doc["a"]["b"] = value(..)` on a missing table: toml_edit then makes an inline table (`runtime = { .. }`) at the top of the file. A new `[runtime]` prints after the last `[[routers]]`; the TOML stays valid and gives the same config. A `[[routers]]` entry is `ArrayOfTables`; a new router subtable (`[routers.pppoe]`) prints under its entry. On an existing key, keep the decor of the old value (`decor().clone()`), so the trailing comment stays.
- `reset` also removes the comment lines above the key (its prefix decor); this is a known effect.
- `Draft::new` on a file with a TOML error is an `Err`: the Settings tab then shows the error and the text read-only, and says to fix the file by hand.
- Router name of `add_router`: `[A-Za-z0-9._-]{1,64}`, not `.` or `..` (the name goes into the secret file name).

- [ ] Tests (`draft.rs`):
  - `a_change_keeps_comments_and_key_order`: a file with comments; `set(runtime.mode, "apply")`; all other lines are the same, and the line is `mode = "apply" # shadow | apply` (the trailing comment stays).
  - `set_in_a_minimal_file_adds_the_table` (Review Focus 5): file with only `[[routers]]`; `set(runtime.scan_interval_idle, 60)`; the text has a `[runtime]` line (not an inline table) and `scan_interval_idle = 60`, and `Config::from_toml_str` gives 60 and the same router.
  - `set_a_router_subtable_goes_under_its_entry`: two routers; `set(routers.0.pppoe.enabled, true)`; `[routers.pppoe]` prints between the two `[[routers]]`, and only router 0 has pppoe on.
  - `a_bad_router_name_is_rejected` (`../x`, `a/b`, `..`).
  - `reset_removes_the_key` and `reset_removes_a_table_that_the_editor_made`.
  - `add_and_remove_a_router`, `add_and_remove_a_dhcp_server`.
  - `list_push_remove_move`.
  - `undo_goes_back_one_change` (also the `made` set).
  - `changed_counts_the_keys`: two sets and one reset of an existing key give three paths.
  - `diff_lines_show_old_and_new`.
- [ ] Commit: `feat(tui): add the settings draft`.

### Task 3: Validate and secrets

**Files:** create `src/tui/settings/secrets.rs`, `src/tui/settings/validate.rs`; modify `src/cli.rs`, `src/files.rs`.

**Produces:**
- `cli::check_with(cfg: &Config, owner_uid: u32, typed: &HashSet<PathBuf>) -> Vec<CheckLine>`: the same lines as `check`, but a password file or token file in `typed` counts as readable. `check` calls `check_with(cfg, owner_uid, &HashSet::new())`. The existing check tests do not change.
- `pub fn parse(text: &str) -> Result<Config, String>`: `Config::from_toml_str` only (no IO, about 1 ms). The UI thread runs it at each commit of a field.
- `pub fn validate(text: &str, typed: &HashSet<PathBuf>) -> Result<(Config, Vec<String>), String>`: `parse`, then `check_with(.., 0, typed)`: `Error` lines make an `Err`, `Warn` lines are the second value. It has IO (dpkg status, secret files, a probe write in `state_dir`), so it runs only in the preview worker (`v`). Note in the help: an env variable in `bot_token_env` or `chat_id_env` is read from the TUI process, not from the service.
- `pub struct Secret(String)` with `Debug` that prints `Secret(***)` and no `Display`. `pub struct Pending { pub path: PathBuf, pub value: Secret }`.
- `files::write_secret(path: &Path, bytes: &[u8]) -> anyhow::Result<()>`: as `write_private` (0600 temporary file, rename), plus `sync_all` of the file and of the folder, and owner root (`fchown(0, 0)`). If the process is not root (the tests), `fchown` is not called. A trailing newline is added.
- `pub fn write_secrets(list: &[Pending]) -> anyhow::Result<SecretUndo>` and `SecretUndo::rollback(self)`: before each write, keep the old bytes (or "did not exist"). On an error in the list, roll back the files already written, then return the error. `rollback` runs in reverse order, removes a new file, and writes back the old bytes of a replaced file. The view keeps one `Pending` for each path (a new typed value replaces the old one). `write_secret` does not make the folder (INSTALLATION.md makes it).

- [ ] Tests:
  - `validate_names_the_key` (`runtime.mode = "x"`), `validate_accepts_a_typed_password_file` (the `password_file` does not exist, it is in `typed`: no error), `validate_returns_warnings` (plain API warning; the password file is in `typed`, because a test is not root and owner 0 fails). The tests use a temp `state_dir`, an existing `libreqos.directory`, and `target = "v2_pair"`.
  - `secret_debug_hides_the_value` (`format!("{:?}", Secret("pw".into()))` has no `pw`).
  - `write_secrets_mode_0600_and_rollback` (Review Focus 2): a new file and a replaced file in a temp folder; `rollback` removes the new one and gives the old bytes back.
  - `a_failed_secret_write_rolls_back_the_earlier_ones` (the second path is in a folder that does not exist).
- [ ] Commit: `feat(tui): validate the draft and write secret files`.

### Task 4: Preview

**Files:** create `src/tui/settings/preview.rs`; modify `src/runner.rs` (refactor only).

**Produces:**
- `runner::cycle_inputs(cfg: &Config, now: u64) -> anyhow::Result<Inputs>` with `pub struct Inputs { pub live: files::Live, pub state: State, pub lqos_dir: PathBuf }`: the code of `cycle_inner` after the target resolve (`first_run`, `read_live`, `load_state`), moved, not copied. `cycle_inner` resolves the target and sets `status.target` first, as now, then calls `cycle_inputs`. So a read error still shows the resolved target. No write.
- `pub fn snapshot_routers(text: &str) -> anyhow::Result<(u64, HashMap<String, RouterResult>)>`: `taken`, and for each router its result (`online` with the tables, `failed`, `unreachable`, `offline`).
- `pub enum Preview { Skipped(String), Ran(Ran), Failed(String) }`, `pub struct Ran { pub text: String, pub counts: [usize; 6], pub warnings: Vec<String> }` (circuits added, removed, changed; nodes added, removed, changed).
- `pub fn output_keys_changed(changed: &[String]) -> bool`: true if a path is not `telegram.*`, `runtime.log_level`, or `tui.*` (spec 6.3).
- `pub fn preview(old: &Config, new: &Config, snapshot: Option<&str>, now: u64, max_age: u64) -> Preview`:
  - `cycle_inputs(old, now)`: the files and the state of both runs. An error gives `Failed(reason)`. The target and `v3` of each run come from its own config (`detect::resolve`). A change of `libreqos.directory`, `network.preserve_network_config`, `runtime.static_comment`, or (with no `state.json`) a `circuits.*` key gives the warning "the preview reads the files of the current config; the effect of <key> is not shown". The old config of the preview is the file on disk; if its digest is not the `config_digest` of `status.json`, the warning "the file on disk is not the loaded config".
  - Routers of each config in its order: from the snapshot by name. No snapshot for a router: `Failed` result for that router and the warning "effect of router X is not known". A source (`pppoe`, `hotspot`, `dhcp`) that is on in `new` and off in `old` for a router: the warning "effect of router X <source> is not known (not read at the last cycle)".
  - No snapshot text, or `now - taken > max_age`: the warning "snapshot.json is <n> s old" (or "no snapshot.json"). The preview still runs.
  - `assemble` for `old` and for `new`, `force = false`, the same `now`, and an ID counter that starts again for each run (`PV000001`...; never a fixed text: the collision loop of `cycle.rs` needs new values). After the run, the Circuit ID, the Device ID, and (in `flat_promotion`) the Parent Node ID of a circuit that is not in `state.circuits` become `(new)` in both outputs, so the counter order does not show as a change.
  - `diff::diff_pair` on the two rendered CSV texts (`Columns::V3` if the target is v3) and the two node trees. The counts come from its last line. For `v3_network`, the alerts of `render_libreqos_network(new rows)` go to `warnings`. The `alerts` of the `new` report go to `warnings`.
  - Held circuits: the count and the names show as "held: removed after one held cycle" (the guard removes them at the next cycle), not only as a warning.
  - A change of `runtime.mode` to `apply` gives the warning "apply mode writes the live LibreQoS files; run lqosync3 diff first". A change of `libreqos.target` gives the warning "the target changes; the column change is not shown".
  - Snapshot age: `now.saturating_sub(taken)`.
- The TUI never shows "0 changes" for `Skipped` or `Failed`: it shows the reason.

- [ ] Tests (`preview.rs`, with temp folders and a snapshot text; no router):
  - `same_snapshot_same_config_gives_no_change` (counts all 0, `Ran`).
  - `a_rate_change_gives_the_expected_circuit_changes` (`bandwidth_defaults.min_rate_percentage` 0.5 to 0.25 on a PPP user with `5M/20M`: one `~ circuit` line with the new Download Min).
  - `an_old_snapshot_gives_the_stale_warning`.
  - `a_new_router_effect_is_not_known`.
  - `a_source_turned_on_is_not_known`.
  - `a_failed_preview_is_failed_not_zero` (a bad `network.json` in the directory of `old`, with `network.preserve_network_config = true`: `Failed`, and the text has no `+0`).
  - `held_circuits_show_as_removed_later` and `mode_change_to_apply_warns`.
  - `flat_promotion_new_circuits_show_no_false_change`.
  - `output_keys_changed_skips_telegram_log_level_and_tui`.
  - `cycle_inputs_writes_nothing` (the temp folder has the same files after the call).
  - The runner tests and `tests/scenarios.rs` pass unchanged.
- [ ] Commit: `feat(tui): add the settings preview`.

### Task 5: Apply and verify

**Files:** create `src/tui/settings/apply.rs`. After `apply` returns `Ok`, `src/tui/mod.rs` calls `dashboard::send(Kind::Reload, &app.data, now, fresh_pid(&cfg))` (no change in `dashboard.rs`).

**Produces:**
- `pub enum ApplyError { Changed { diff: Vec<(char, String)> }, Io(String) }`.
- `pub fn apply(path: &Path, start_digest: &str, new_text: &str, secrets: &[Pending], state_dir: &Path, keep: usize, now: u64) -> Result<String, ApplyError>` (returns the new digest):
  1. Read the file. If `config_digest(bytes) != start_digest`: `Changed` with the line diff of the disk text and `new_text`. Nothing is written.
  2. Copy the old file to `<state_dir>/config-backups/<now:012>.toml` (mode of the old file). Keep the newest `keep` (`runtime.keep_backups`) files whose stem has 12 digits and whose extension is `toml`; never remove the new one. New code: `files::backup` works on folders and cannot be used.
  3. `write_secrets(secrets)`.
  4. `files::write_if_changed(path, new_text)`. On an error: `SecretUndo::rollback`, then `Io`.
- `pub enum Verify { WaitReload, Rejected(String), Loaded, Done(String), NotVerified { want: String, have: String } }`.
- `pub fn verify(sent: &Sent, want_digest: &str, loaded_seq: Option<u64>, status: Option<&Value>, now: u64, timeout: u64) -> Verify`:
  - `trigger = "reload"`, `updated >= sent.at`, `config_digest == want`: `Loaded`. Then a status with `cycle_seq` higher than `loaded_seq` (same `pid`): `Done` with the router states, the counts, and the error.
  - `trigger = "reload"`, `updated >= sent.at`, `config_error` set: `Rejected(config_error)` ("the old config stays").
  - After `timeout` without `Loaded` or `Rejected`: `NotVerified` with the wanted digest and the digest in `status.json` (or `unknown`). A status with another `pid` (a restart): `NotVerified` with the text "the service restarted".
  - Note: the reload status keeps `cycle_seq`; the next cycle has `trigger = reload` and `cycle_seq + 1`. If the TUI sees only that cycle status, it shows `Loaded`, then `Done` after one more cycle. Safe, but slow.
  - The text "applied" exists only for `Loaded` and `Done`.

- [ ] Tests (`apply.rs`):
  - `a_changed_file_on_disk_stops_the_apply` (Review Focus 1): no backup, no secret file, the file is the same.
  - `apply_writes_a_backup_and_keeps_n` (`keep = 2`, three applies with three different `now`).
  - `a_toml_write_error_rolls_back_the_secrets` (Review Focus 2): a real config file, and a folder at `<dir>/.<name>.tmp-<pid>` so the temporary file of `write_if_changed` cannot be made; the secret files are back to the old state, and the config file is the same.
  - `verify_needs_the_new_digest` (Review Focus 3): a reload status with the old digest and no error stays `WaitReload`, then `NotVerified` at the timeout.
  - `verify_rejected_shows_config_error`, `verify_loaded_then_done_after_a_higher_cycle_seq`.
- [ ] Commit: `feat(tui): apply the config and verify the reload`.

### Task 6: Settings screen

**Files:** create `src/tui/settings/view.rs`; modify `src/tui/app.rs` (the Settings tab, the key capture, the quit confirmation), `src/tui/mod.rs` (the preview worker and the apply call).

**Produces:** `pub struct SettingsView` with the draft, the group list, the key list, the focus panel, the input state, and the apply stage. The title shows the number of changed keys. Layout of spec 6.3: groups left (each TOML section, `router_defaults`, each router as `router <name>` with its `pppoe`, `hotspot`, `dhcp`, `exclude`, and each DHCP server), keys center (value in the file or `default`, and the new value), help right (help, default, `optional`, `read-only`).

Keys:
- `Up`/`Down` select, `Left`/`Right` change the panel (`Tab` stays the tab key of 4a).
- An open field, dropdown, list editor, or modal takes all keys (as the 4a search boxes), so `q`, `1`-`5`, `S`, `R`, and `Tab` go into the field.
- `Enter` or `Space` on a key: `Bool` toggles; `Choice` opens a dropdown (`Up`/`Down`, `Enter`, `Esc`); `Int`/`Float`/`Text` opens a field (`Enter` commits, `Esc` cancels); `List` opens a list editor (`a` add, `d` delete, `K`/`J` move; a `List(Choice)` item uses a dropdown).
- A commit runs `parse` (Task 3). An error stays as a message that names the key; the value stays in the draft so the operator can fix it.
- `r` reset, `u` undo, `+`/`-` add or remove a router (name, address, username, then a masked password; the remove confirmation names the password file and says that the file stays), `+`/`-` on a DHCP server group, `P` set a new password (router) or token (`telegram.bot_token_file`).
- `v` validate and preview: the preview runs in a worker thread; the screen shows "preview running", then the TOML line diff, the counts, the circuit and node lines, and the warnings, or the reason of `Skipped`/`Failed`.
- `A` apply: only after a preview of the same draft text and the same secret list. `Ran` and `Skipped` allow apply. `Failed` blocks it (fix the cause, then `v` again). A change of secrets only (the TOML text is the same) is allowed: the apply writes the secrets and sends `SIGHUP`, and verify waits for a reload status with `updated >= sent.at` and no `config_error`. A modal shows the number of changed keys, the circuit and node counts, the secret files that will be written, and all warnings; `y` applies. Then the verify stage shows `WaitReload`, `Loaded` ("cycle running"), `Done`, `Rejected`, or `NotVerified`.
- `q` or `Ctrl+C` with changes or typed secrets: a modal "discard N changes? y".
- A masked field shows one `*` for each character. The typed text is only in `Secret`.
- Read-only mode: each edit key gives the message "read-only (not root)". `runtime.state_dir`: the message of its help.

- [ ] Tests (`view.rs`, `TestBackend`):
  - `settings_tab_renders_groups_keys_and_help`, `narrow_terminal_puts_help_under_the_keys`.
  - `space_toggles_a_bool`, `enter_on_an_enum_shows_its_choices` (all 6 `network.mode` choices in the buffer).
  - `a_bad_number_shows_the_key_error`.
  - `a_typed_password_is_never_in_the_buffer` (Review Focus 4): type `S3cret!` in the password field, draw: no `S3cret` in the buffer, in `draft.text()`, in the undo texts, or in `message`.
  - `apply_needs_a_preview_of_the_same_text`, `apply_asks_for_a_confirmation`, `a_failed_preview_blocks_apply`.
  - `quit_with_changes_asks_first` (also `Ctrl+C`), `read_only_blocks_an_edit`, `state_dir_cannot_be_edited`.
  - `an_open_field_takes_q_and_digits`.
  - `a_broken_config_file_shows_the_error`.
- [ ] Commit: `feat(tui): add the settings editor screen`.

### Task 7: Docs, review, graph

- [ ] `docs/OPERATIONS.md` "Terminal view": the Settings keys and the apply procedure. `docs/CONFIGURATION.md`: `preview_max_age`. `brain/HANDOFF.md`.
- [ ] Release check: `cargo test --release -- --ignored perf`. The TUI in a pty (`script`) on a temp config: the Settings tab draws (read-only, because the session is not root). Edit, preview, and apply are covered by the unit tests of Tasks 2 to 6; the first real apply is on the shaper by the operator.
- [ ] Whole-branch review by a fresh reviewer, then graphify update, then commit and push.

## Review rulings (fact-check of 2026-11-01)

- Finding 1 (blocker): `set` makes standard tables, not inline tables.
- Findings 2 to 18: fixed in the tasks above.
- Finding 19: the slow `Loaded` path is accepted; a restart gives `NotVerified`.
- Finding 20: reverse rollback, one `Pending` for each path, no folder make.
- Finding 21: `made` is in the undo entry; a broken file is shown read-only; the disk-not-loaded warning is in the preview.
- Finding 22: the range in the help is a spec gap (ruling in Task 1). The required router keys get a help text in the code.
- Finding 23: `SIGHUP` after apply is in Task 5; the `v3_network` alerts and the saturating age are in Task 4.
