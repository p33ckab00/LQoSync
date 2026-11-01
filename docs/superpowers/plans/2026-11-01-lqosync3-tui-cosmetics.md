# LQoSync3 TUI Cosmetics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task. Use `superpowers:subagent-driven-development` only if the operator selects delegation. Steps use checkbox syntax for progress.

**Goal:** Implement all A1–A5 and B6–B10 cosmetics requirements without changes to operational behavior.

**Architecture:** Add one shared palette module and pass its resolved palette to the existing renderers.
Keep the current view state, data sources, and apply state machine.
Compute counts and capacity scales when their source data changes.

**Tech Stack:** Rust 2024, ratatui 0.30.2, existing crossterm, serde, and toml_edit dependencies.

**Spec:** [TUI cosmetics design](../specs/2026-11-01-lqosync3-tui-cosmetics-design.md).
Baseline: `cf2c959`, branch `lqosync3`, worktree `/home/pi/LQoSync3.0`.
Status: Plan complete. Source implementation has not started.

## Global Constraints

- Preserve all current keys, tab order, focus, mouse actions, and selection behavior.

- Preserve circuit identity, flags, source precedence, rate calculations, and topology.

- Preserve Settings validation, preview, backup, secrets, apply, and result verification.

- Preserve read-only checks, confirmations, signal checks, journal filters, and raw copy text.

- Add only `tui.theme = "dark"`, with values `dark`, `light`, and `mono`.

- Apply a new theme at TUI restart. Do not add live theme changes or a key binding.

- Keep the service status schema, sampler, refresh rate, file limits, and 120 history samples.

- Use the current dependencies. Do not add a generic widget framework or refactor unrelated Settings code.

- Keep draw code free of I/O, subprocesses, and writes.

- Do not deploy, signal services, connect to routers, or change production files for this work.

- Keep the unrelated deferred fixes outside this plan.

Commands below assume `cargo` is in PATH.
In this environment, use `/home/pi/.cargo/bin/cargo` if necessary.
For each task, run new tests before the implementation and check the expected failure.
After the implementation, require exit code 0 from its focused gate before its commit.

## Review Focus

1. Resize during an input or pending action: preserve state and clear stale mouse targets. Task 2 tests this.

2. Selected warning badges in light or mono: preserve selection and text meanings. Tasks 1, 3, and 7 test this.

3. Network search reveals a deep node: preserve search semantics, ancestor guides, and the full-tree scale. Task 4 tests this.

4. Journal prefixes contain Unicode or fail recognition: preserve raw bytes and selection copy. Task 5 tests this.

5. A removed DHCP child disappears from the group list: attribute changes to a surviving owner without index-prefix collisions. Task 7 tests this.

## File Map and Shared Interfaces

| File | Responsibility |
|---|---|
| `src/config.rs`, `config/defaults.toml` | Typed theme and embedded default |
| `src/tui/theme.rs` (new) | Palette, semantic styles, blocks, badges, and key spans |
| `src/tui/mod.rs`, `src/tui/app.rs` | Startup palette, shell, sizing, dispatch to renderers |
| `src/tui/devices.rs` | Device table and cached circuit counts |
| `src/tui/network.rs` | Tree presentation metadata and cached rate scale |
| `src/tui/journal.rs` | Log spans and empty mouse rectangle method |
| `src/tui/dashboard.rs` | Six cards, gauges, and existing histories |
| `src/tui/settings/meta.rs`, `src/tui/settings/view.rs` | Theme choice, field styles, group counts, dialogs |
| `docs/CONFIGURATION.md` | Theme values, default, and restart rule |

Keep tests in the existing `#[cfg(test)]` modules of these files.
Task 1 updates renderer signatures and their test callers in one compile-safe change.
Later tasks implement each renderer through those signatures.

## Task 1: Theme Configuration and Shared Palette

**Files:** Create `src/tui/theme.rs`. Change `src/config.rs`, `config/defaults.toml`, `src/tui/mod.rs`, `src/tui/app.rs`, all five view modules, `src/tui/settings/meta.rs`, and `docs/CONFIGURATION.md`.

**Interfaces:**

- `config::Theme`: `Dark`, `Light`, `Mono`. Derive `Debug, Clone, Copy, PartialEq, Eq, Deserialize`. Use lowercase serde names.

- `config::Tui::theme: Theme` and `App::palette: Palette`.

- `theme::Role`: `Base`, `Secondary`, `Accent`, `Ok`, `Warning`, `Error`.

- `theme::Palette::new(theme: Theme) -> Palette`.

- `Palette::role(&self, role: Role) -> Style` and `Palette::selection(&self) -> Style`.

- `Palette::row(&self, index: usize, selected: bool) -> Style`.

- `Palette::panel<'a>(&self, title: Line<'a>, focused: bool) -> Block<'a>`.

- `Palette::badge<'a>(&self, text: &'a str, role: Role, selected: bool) -> Span<'a>`.

- `Palette::keys(&self, hint: &str) -> Line<'static>`: style bracketed key spans, preserve all text.
- Append `palette: &Palette` to existing `draw` methods on DevicesView, NetworkView, JournalView, DashboardView, and SettingsView.

- Preserve `App::new` arguments. Initialize its palette to Dark, then set it from `cfg.tui.theme` in `tui::run`.

- [ ] Add `tui_theme_values_and_default` in `config.rs`. Assert `defaults().tui.theme == Theme::Dark`, valid user values load, and `theme = "wrong"` fails with a theme error.

- [ ] Add `palette_roles_selection_and_mono` in `theme.rs`. Assert Cyan accent for Dark, Blue for Light, rounded corners, and Reset colors for Mono.

- [ ] Assert Light warning badges use Yellow on Black. Assert selected badges keep the selection background in all three themes.

- [ ] Add `keys_preserve_text_and_highlight_brackets`. For `[/] search  [o] sort  [q] quit`, assert exact plain text and accent/bold key spans.

- [ ] Run `cargo test --lib tui_theme` and `cargo test --lib tui::theme::tests`. Confirm the new contract fails before implementation.

- [ ] Implement the interfaces and the exact role table in spec section 3. Use DIM for secondary text where the foreground matches normal text.

- [ ] Add the default, `TUI_THEME` choice, metadata help, and config documentation. Update the distinct choice count from 10 to 11.

- [ ] Register the theme module and wire startup and renderer callers. Keep rendering otherwise unchanged in this task.

- [ ] Run `cargo test --lib`. Require all tests to pass, including metadata coverage and `tui.*` preview classification.

- [ ] Commit only this task's files: `feat(tui): add shared theme styles`.

## Task 2: Shell, Key Chips, and Minimum Terminal Size

**Files:** `src/tui/app.rs`, `src/tui/journal.rs`, `src/tui/settings/view.rs`.

**Interfaces:** Consume Task 1's palette.
Add `JournalView::clear_area(&mut self)` to set its private mouse rectangle to `Rect::default()`.
Keep `SettingsView::hint(&self) -> String`, but preserve the bracketed output of its current `bar()`.
Make the local App `dialog` helper consume `&Palette`.

- [ ] Add `shell_pills_at_minimum_size` with `App` fixtures for active/failed, apply/shadow, unknown, and read-only.

- [ ] At 80×24, assert all relevant pills fit in the first two rows, including `READ-ONLY`, target, cycle age, and next scan.

- [ ] Assert buffer styles for state roles. Add long Unicode state/target strings and a config error without loss of `READ-ONLY`.

- [ ] Add `minimum_size_preserves_state_and_clears_mouse_targets`. Draw 79×24, 80×23, 1×1, and 0×0 after a normal draw.

- [ ] Assert the fallback text where it fits, empty tab/Settings/Journal rectangles, and unchanged draft, input, pending action, selection, and search.

- [ ] Resize to 80×24. Assert normal view and valid mouse rectangles return. Verify the pending action still completes through the existing `set_data` path.

- [ ] Add `footer_keys_follow_current_context`. Cover all five tabs, search inputs, both App modals, and each Settings input/modal branch of `bar()`.

- [ ] Assert `[S]` and `[R]` preserve case. Keep `[q]` in normal mode and confirm/cancel/completion chips in captured modes.

- [ ] Run `cargo test --lib tui::app::tests` and check the new expectations fail.

- [ ] Implement the two-row header, shared tabs and dialogs, and chip footer. Preserve message precedence and all key dispatch.

- [ ] Use one footer row where possible, two for long hints, and a separate message row when necessary. Remove low-priority hints first.

- [ ] Return early from draw below 80×24 after clearing stale hit rectangles. Do not stop loader updates or reset state.

- [ ] Update existing header tests that draw at height 10. Use supported dimensions for tests of normal content.

- [ ] Run `cargo test --lib tui::`. Require the existing key, read-only, mouse, action, and Settings interaction tests to pass.

- [ ] Commit: `feat(tui): style shell and handle small terminals`.

## Task 3: ShapedDevices Table

**Files:** `src/tui/devices.rs`.

**Interfaces:** Consume Task 1's palette.
Add cached `total_circuits: usize` and `filtered_circuits: usize` to DevicesView.
Update these caches through `on_data` / `rebuild`, using distinct `circuit_id` values.
Keep the existing `Column::ALL`, selection identity, and visible-row window.

- [ ] Add `circuits_count_distinct_ids`. Use three rows with two circuit IDs. Assert total 2, then filter to two devices in one circuit and assert filtered 1.

- [ ] Add `table_styles_and_alignment`. Assert alternate row styles use absolute filtered-list indexes after scroll, and rate text reaches the right edge of its cell.

- [ ] Set all three flag bits on one row. Assert `dup IP`, `no parent`, and `bad rate` remain separate styled spans.

- [ ] Select that row in Dark, Light, and Mono. Assert badge text remains visible and the selected background wins.

- [ ] Add `compact_columns_keep_all_detail`. At width 80, assert the five specified columns and every omitted value in the circuit panel.

- [ ] Assert the circuit total, query, and sort indicator remain visible. At width 100, assert the existing side-by-side breakpoint remains.

- [ ] Run `cargo test --lib tui::devices::tests`. Confirm new tests fail before the render changes.

- [ ] Implement zebra rows, aligned rate cells, flag badges, cached counts, and compact columns per spec section 5.

- [ ] Keep the sort cycle for hidden columns. Keep complete flag detail and field values in the detail panel.

- [ ] Run `cargo test --lib tui::devices::tests` and the release performance command from Task 8. Require both to pass.

- [ ] Commit: `feat(tui): style device rows and circuit counts`.

## Task 4: Network Tree and Configured-Capacity Bars

**Files:** `src/tui/network.rs`.

**Interfaces:** Consume Task 1's palette.
Add `Node::prefix: String` for connectors and `NetworkView::download_scale: Option<f64>` for the full-tree maximum.
Compute prefix metadata in the existing flatten traversal. Compute scale only when the tree data changes.

Add local `rate_fill(value: Option<f64>, scale: Option<f64>) -> Option<f64>`.
Return `None` for a missing, negative, or non-finite rate. Return `Some(0.0)` for a valid zero rate without a positive scale.
For a positive finite scale, return the ratio clamped to 0–1. Missing scale yields no fill for a positive rate.

Keep `Node::rate`, `selected_name`, `reveal`, `find`, and expanded state behavior.

- [ ] Add `tree_guides_follow_visible_siblings`. Cover multiple roots, nested last children, collapsed branches, and an offset window.

- [ ] Assert connector strings contain the expected `├─`, `└─`, and ancestor `│`. Assert branches use `▸` / `▾` and leaves have no disclosure marker.

- [ ] Add `search_reveal_keeps_guides_and_scale`. Search for a deep node through existing keys. Assert selection remains correct and the cached full-tree maximum stays unchanged.

- [ ] Add `configured_rate_scale_handles_missing_and_zero`. Use 100, 25, zero, missing, and negative rates. Assert scale is `Some(100.0)` and 25 maps to one quarter fill.

- [ ] Assert missing/invalid rates say `unknown`, all-zero rates avoid division, and labels say `D/U Mbps` plus `bar: download / max 100 Mbps`.

- [ ] Assert `rate_fill(Some(25.0), Some(100.0)) == Some(0.25)` and `rate_fill(Some(0.0), None) == Some(0.0)`.

- [ ] Assert `rate_fill(Some(f64::NAN), Some(100.0)) == None`, with equivalent checks for infinity and negative rates. JSON fixtures cannot represent non-finite values.

- [ ] Run `cargo test --lib tui::network::tests` and check new expectations fail.

- [ ] Implement the new prefixes, type text/styles, scale, and short bars per spec section 6. Hide bars before names at narrow widths.

- [ ] Preserve exact rates in the detail panel and current default type `site`. Keep numeric upload visible when space permits.

- [ ] Replace the old `+ Site` decoration assertion with connector/disclosure assertions. Keep all search, collapse, and reveal tests.

- [ ] Run `cargo test --lib tui::network::tests`. Require all tests to pass.

- [ ] Commit: `feat(tui): add network guides and capacity bars`.

## Task 5: Journal Metadata Styles

**Files:** `src/tui/journal.rs`.

**Interfaces:** Add local `styled_line<'a>(text: &'a str, level: u8, selected: bool, palette: &Palette) -> Line<'a>`.
The function returns borrowed spans into the original text when possible.
Do not change stored lines or `selection_text`.

- [ ] Add `short_iso_metadata_is_secondary`. Use `2026-11-01T03:00:00+0000 host lqosync3[12]: message` and assert timestamp/host styles differ from the message.

- [ ] Assert `styled_line(raw, 0, false, &palette).to_string() == raw` for that line, a Unicode hostname/message, a banner, and a malformed prefix.

- [ ] Add `selected_metadata_and_severity_remain_readable`. Cover warning and error lines under all themes, including metadata inside the selection.

- [ ] Add `styled_journal_copy_is_raw`. Select multiple original lines and assert `selection_text()` equals the original joined lines exactly.

- [ ] Run `cargo test --lib tui::journal::tests` and check the new style expectations fail.

- [ ] Implement prefix recognition for the current `short-iso` output only. On failed recognition, style the whole original line by severity.

- [ ] Use shared panels and error styles. Keep selection style above metadata DIM. Do not change spawn arguments, filters, follow, or OSC 52.

- [ ] Run `cargo test --lib tui::journal::tests` and `cargo test --lib tui::app::tests`. Require both to pass.

- [ ] Commit: `feat(tui): dim journal metadata`.

## Task 6: Dashboard Cards and Process Gauges

**Files:** `src/tui/dashboard.rs`.

**Interfaces:** Consume Task 1's palette.
Add local `card_areas(area: Rect) -> [Rect; 6]`, in Service, Last cycle, Routers, Counts, Versions, Process order.
Add local `fill_ratio(value: f64, scale: f64) -> f64`: return a finite clamped ratio, or 0 for an invalid scale/value.
Keep `DashboardView::on_data`, 120 samples, and existing process measurements unchanged.

- [ ] Add `dashboard_card_geometry`. Assert six non-overlapping card rectangles within the body at 80×24, 100×30, and 120×40 through full App draw.

- [ ] At minimum shell size with one footer row, assert body height 20 and row heights `[6, 7, 7]`. Process height must be 7.

- [ ] Assert width 119 uses two columns and width 120 uses three. Cover two-row footer and a message row with no overflow.

- [ ] Add `dashboard_keeps_priority_fields`. Assert all six titles, errors, guard alerts, failed/offline routers, CPU/RSS labels, and both traces at minimum size.

- [ ] Cover unknown/stale data, version mismatch, config not loaded, and a router overflow label `+N more`.

- [ ] Add `process_gauge_scales_and_unknowns`. Assert `fill_ratio(25.0, 100.0) == 0.25`, `fill_ratio(150.0, 100.0) == 1.0`, and zero/invalid scales return 0.

- [ ] With CPU 150%, assert full numeric value and `>100% of one core`. Clamp only the rendered trace to 1000 tenths.

- [ ] With RSS history `[1024, 2048]` kB and current 1024, assert half fill, `current / window peak`, and MiB values 1.0 / 2.0.

- [ ] Assert an unknown initial CPU adds no synthetic zero. Absent current process data says `unknown` and marks retained traces `history only`.

- [ ] Run `cargo test --lib tui::dashboard::tests` and check new layout/gauge assertions fail.

- [ ] Implement six separate cards, width breakpoints, row budgets, role styles, gauges, and labeled existing traces per spec section 8.

- [ ] Use failed/offline-first order only in the displayed router summary. Keep stored router order unchanged.

- [ ] Preserve all old diagnostic fields when space permits. Keep existing `lines` tests of values or adapt them to the equivalent card content.

- [ ] Run `cargo test --lib tui::dashboard::tests` and `cargo test --lib tui::app::tests`. Require both to pass.

- [ ] Commit: `feat(tui): add dashboard cards and gauges`.

## Task 7: Settings Field Styles and Changed Groups

**Files:** `src/tui/settings/view.rs`.

**Interfaces:** Consume Task 1's palette and the existing `KeyPath`, `Seg`, `changed`, `after_change`, and `rebuild_groups`.
Add presentation-only `Group::scope: KeyPath` and `Group::changed_count: usize`.
Add local `group_change_counts(groups: &[Group], changed: &[String]) -> Vec<usize>`, in group order.
Invoke it after groups rebuild, outside the draw loop. Keep the existing global changed/secret counts.

- [ ] Add `group_counts_match_segments`. With changes under `routers.1` and `routers.10`, assert each router group receives only its own paths.

- [ ] Add `removed_children_count_in_surviving_owner`. Remove a DHCP server/client and assert its changed paths reach the nearest surviving parent group.

- [ ] Test removed router paths when no owner group survives. Keep these paths in the existing global changed count, without a new group.

- [ ] Assert duplicate changed paths count once per group and parent/child group counts never inflate the global total.

- [ ] Add `counts_refresh_after_draft_actions`. Cover edit, reset, undo, add, remove, and the current apply-completion callback without real I/O or signals.

- [ ] Add `settings_field_styles_preserve_origin_and_selection`. Assert warning key names plus `* was`, secondary default/router-default values, and grey `(read-only)` labels.

- [ ] Cover Dark, Light, and Mono with a selected changed/read-only field. Keep secret counts separate and use no secret material in snapshots.

- [ ] Run `cargo test --lib tui::settings::view::tests` and check new count/style assertions fail.

- [ ] Populate scopes from concrete paths at group construction. Use segment comparisons, never `starts_with` on router index text or group titles.

- [ ] Count a changed path in each surviving ancestor scope that owns it. For vanished child groups, its deepest surviving owner still receives it.

- [ ] Keep client-list scope metadata even though its rows use `Extra::Client`. Deduplicate paths per group.

- [ ] Implement key/value spans, changed title counts, shared focus borders, and all Settings popup states per spec section 9.

- [ ] Pass `&Palette` to local popup helpers. Preserve input capture, footer bars, validation, preview, secrets, and apply transitions.

- [ ] Run `cargo test --lib tui::settings::` and `cargo test --lib tui::app::tests`. Require all existing behavior tests to pass.

- [ ] Commit: `feat(tui): mark changed settings groups`.

## Task 8: Whole-TUI Verification and Handoff

**Files:** Tests in `src/tui/app.rs`, relevant local test modules, `brain/HANDOFF.md`, and a session note under `brain/10-sessions/`.

- [ ] Add `all_tabs_themes_and_size_matrix`. Draw every tab in Dark/Light/Mono at 80×24, 100×30, and 120×40.

- [ ] Assert rounded corners, semantic styles, visible selection, safe bounds, and valid mouse targets. Include long Unicode names, empty data, stale files, and loader errors.

- [ ] Cover representative Settings field/dropdown/list/secret/client and apply/remove/discard dialogs. Keep confirm and cancel chips visible.

- [ ] Run `cargo test`. Require zero failures. Record counts and intentional ignored tests.

- [ ] Run `cargo clippy --all-targets -- -D warnings`. Require exit code 0.

- [ ] Run `cargo fmt --check`. Require exit code 0.

- [ ] Run `cargo test --release --lib tui::devices::tests::perf_20k_rows_key_press_under_16ms -- --ignored`. Require the named test to pass.

- [ ] Review each tab and representative dialogs in a real terminal at 80×24, 100×30, and 120×40, under all three themes.

- [ ] Record visual results for contrast, selection, borders, Unicode alignment, and resize. If terminal review is unavailable, record that limit explicitly.

- [ ] Inspect the complete diff. Confirm no new dependency, production change, status schema change, sampler change, or unrelated behavior fix.

- [ ] Update project notes with the implementation commit, checks, remaining work, next action, and an Asia/Shanghai timestamp.

- [ ] Commit the final checks and notes: `test(tui): verify cosmetics across themes`.

## Spec Coverage and Plan Review

Review date: 2026-11-01 (Asia/Shanghai).
Method: Inline review against every numbered spec section, interface checks, and test ownership checks.

| Spec section / acceptance | Implementation tasks | Proof |
|---|---|---|
| 1–2: limits and baseline | All tasks, Task 8 | Existing behavior tests, bounded diff review |
| 3 / A1, A5: themes | Tasks 1, 3–8 | Config choice/default tests, style assertions, complete dialog matrix |
| 4 / A2: header | Task 2 | Pills at 80×24, error/read-only coexistence, unknown values |
| 4 / A3: footer/tabs | Task 2 | Context matrix, key case, mouse geometry, modal key visibility |
| 4 / A4: minimum size | Tasks 2, 8 | 79×24, 80×23, 1×1, 0×0, preserved pending action, restored targets |
| 5 / B6: Devices | Task 3 | Distinct circuit counts, right alignment, scroll parity, compact detail |
| 6 / B7: Network | Task 4 | Sibling guides, reveal, full-tree scale, unknown/zero rates |
| 7 / B8: Journal | Task 5 | Metadata styles, malformed/Unicode fallback, exact raw copy |
| 8 / B9: Dashboard | Task 6 | Six-card geometry, priority data, CPU/RSS scale and unknown tests |
| 9 / B10: Settings | Task 7 | Segment scopes, removed children, draft callbacks, field styles |
| 10: interfaces and sequence | Tasks 1–7 | Explicit renderer palette, local helpers, cache ownership |
| 11: acceptance gates | Task 8 | Full tests, clippy, fmt, performance, real-terminal record |

Review result: PASS for plan coverage. All A1–A5 and B6–B10 items have implementation and verification owners.
All five Review Focus cases have tests in their owning tasks.
Shared signatures use the same `Theme`, `Palette`, and existing view types throughout.

The review checks the plan. It does not claim that the future implementation passes acceptance.

The review resolved the Network bar helper contract explicitly, including non-finite rates and a zero scale.
It also specified that removed router paths remain in the global changed count when no group survives.

This change prepares and publishes documentation only.
Run the implementation tasks after the operator authorizes source implementation.
