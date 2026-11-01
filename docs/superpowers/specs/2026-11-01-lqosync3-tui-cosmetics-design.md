# LQoSync3 TUI cosmetics design

Date: 2026-11-01 (Asia/Shanghai).
Status: Design for review. Implementation has not started.
Scope: All items A1–A5 and B6–B10 from the operator request.

Reference baseline: `cf2c959`, branch `lqosync3`.

## 1. Goal and limits

Give all five tabs a consistent appearance inspired by btop.
Use clear status colors, rounded borders, compact labels, and readable data.
Keep the current Rust, ratatui 0.30.2, and crossterm stack.
Do not add dependencies for this work.

This document supplements [the TUI design](2026-10-31-lqosync3-tui-design.md).
It changes presentation only, with one explicit configuration addition: `tui.theme`.
The operator requested a design spec without an ASCII mockup.

Preserve these behaviors:

- Tab order, initial tab, keys, mouse actions, search, sort, and selection.
- Circuit identity, flag calculation, topology, source precedence, and rate calculation.
- Read-only restrictions, signal checks, and action confirmation.
- Settings validation, preview, backup, secret handling, apply, and result verification.

- Journal filters, follow mode, selection, raw copy text, and OSC 52 behavior.
- File limits, refresh intervals, worker threads, and the service status schema.

Do not deploy a binary, restart a service, or change production configuration during this task.
Do not include unrelated deferred fixes from `brain/HANDOFF.md`.

## 2. Current implementation

| Area | Current source and behavior |
|---|---|
| Shell | `src/tui/app.rs`: one header row, tab row, body, and footer row |
| Borders | Individual `Block` values use plain borders |
| Devices | `src/tui/devices.rs`: visible rows only, selected row, plain flag text |
| Network | `src/tui/network.rs`: flattened tree with depth and `+` / `-` markers |
| Journal | `src/tui/journal.rs`: raw `short-iso` lines with whole-line severity colors |
| Dashboard | `src/tui/dashboard.rs`: section text and two existing sparklines |
| Process | `src/tui/proc.rs`: CPU percent of one core and RSS in kB |
| Settings | `src/tui/settings/view.rs`: groups, keys, help, dialogs, and draft change paths |
| Configuration | `src/config.rs`, `config/defaults.toml`, and `src/tui/settings/meta.rs` |

The process history already has a 120-sample limit.
Network rates describe configured bandwidth. They do not measure live traffic.
Process data has no physical-memory total or configured memory limit.

Use these facts when labels and scales refer to capacity.

## 3. Theme contract — A1 and A5

Add this key to the embedded defaults:

```toml
[tui]
theme = "dark" # dark, light, or mono
```

Use a typed `Theme` enum with lowercase serde names.
Reject other values through the normal configuration error path.
An existing file without this key must load with `dark` through the existing defaults merge.
Add the choice and help text to Settings metadata.

Update the enum coverage test and its current count of 10 distinct choice sets.
Add the key to `docs/CONFIGURATION.md` during implementation.

Resolve the active palette at TUI startup.
An edited theme follows the existing preview/apply flow and takes effect after the TUI restarts.
Show this rule in the key help. Do not add a theme key binding or live preview mode.

Keep the existing classification of all `tui.*` keys as keys without output effects.

Put shared styles in `src/tui/theme.rs` and register the module in `src/tui/mod.rs`.
Use semantic roles instead of individual colors in each renderer.
Small helpers for panel blocks, badges, and key chips are enough.

The table specifies ratatui named colors. Terminal palettes can change their appearance.

| Role | dark | light | mono |
|---|---|---|---|
| Canvas / normal text | Black / Gray | White / Black | Reset / Reset |
| Accent and panel title | Cyan, bold title | Blue, bold title | Reset, bold title |
| OK | Green | Green | Reset, label text |
| Warning or changed key | Yellow | Yellow | Reset, bold plus text marker |
| Error | Red | Red | Reset, bold plus text marker |
| Secondary text | Gray | DarkGray | Reset, dim where readable |
| Inactive border | DarkGray | DarkGray | Reset |
| Alternate row background | DarkGray | Gray | Reset |
| Selected row | Black on Cyan, bold | White on Blue, bold | Reset, reverse plus bold |

Use cyan as the accent family. Use Blue in light mode for stronger contrast on White.
In light mode, put warning badges and changed key labels on a Black background with Yellow text.
Use Black foreground on Green and Yellow badge backgrounds in dark mode.

Use White foreground on Red backgrounds for error badges in both color themes.
For plain state text, use the semantic foreground without a badge background.
Check the actual terminal palette during visual review. Adjust shared roles if text is hard to read.

Use `BorderType::Rounded` for panels and dialogs.
Use accent titles on all tabs, including help panels and popups.
Use an accent border for the focused panel and an inactive border elsewhere.
Keep the current focus model. Do not create new focus targets.

Style priority, from highest to lowest:

1. Selection must remain clear in all themes.
2. Error, warning, changed, and read-only meanings must keep text labels.

3. Default or inherited values use secondary text.
4. Zebra rows provide background separation.

Do not let a badge overwrite the selection background.
Keep its text and bold style on a selected row.
Mono uses no explicit chromatic foreground or background colors.
Use bold text on alternate mono rows and reverse style on the selected row.

Do not add emoji, images, animation, or font requirements.

## 4. Header, tabs, footer, and minimum size — A2–A4

### Header

Use two rows at all supported sizes. Pack whole pills and keep each label complete.
Keep this display order:

1. `lqosync3`, `service <state>`, `<mode>`, and `READ-ONLY` when applicable.
2. `target <target>`, `cycle <age>`, and `next <remaining>`.

Use the first row for the first group and the second row for the second group.
Use compact time units and short age labels so the fields fit at 80 columns.
Keep the current source fields and countdown calculation.
Unknown data must say `unknown`. Do not present it as zero or OK.

Ellipsize an unexpectedly long state or target by display width.
Keep all other pills visible.

Use green for `service active` and `apply`.
Use red for `service failed`.
Use yellow for `shadow`, non-active transitional service states, and `READ-ONLY`.
Use secondary text for `unknown`.

Keep the actual service state visible, including inactive states.
The mode pill describes runtime mode. It does not show whether a write succeeded.
Keep configuration errors visible in an error banner or the existing message area.

Do not let an error displace the `READ-ONLY` pill.

### Tabs and footer

Keep `1 ShapedDevices`, `2 Network`, `3 Settings`, `4 Journal`, and `5 Dashboard` in their current order.
Use bold accent text for the active tab and secondary text for inactive tabs.
Recompute mouse rectangles from the actual displayed tab positions.

Use key chips such as `[/] search`, `[o] sort`, and `[q] quit`.
Highlight the bracketed key. Keep the action label in normal text.
Render only the keys valid in the current tab, input, or modal.

Preserve key case: `[S]` and `[R]` differ from lowercase keys.
For long hints, use two footer rows and remove low-priority hints as needed.
Keep quit visible in normal mode. Keep confirm, cancel, and text-input completion keys visible in their respective modes.

Do not bind new keys to match a chip.
Keep the existing precedence for action messages, Settings messages, and loader errors.
Give messages a separate row when needed so they do not hide modal keys.

### Minimum size

The minimum terminal size is 80 columns by 24 rows.
If either dimension is smaller, draw only `terminal too small: min 80x24` within the available rectangle.
Clip this text safely in extremely small rectangles, including 0×0.
Clear cached tab, Settings, and Journal mouse rectangles while this screen shows.

Do not reset the active tab, draft, pending action, selection, or search.
Keep the existing key dispatch and background refresh active.
Restore the full view automatically when the terminal reaches 80×24.
This screen does not confirm, cancel, or resend an action.

## 5. ShapedDevices — B6

Alternate row backgrounds by position in the filtered and sorted list.
Use the absolute visible-list position so stripes remain stable during scroll.
Render only the visible window, as the current implementation does.

Align download and upload min/max rate cells to the right.
Keep names, addresses, identifiers, source, router, and flags left-aligned.
Do not change numeric parsing or sort order.

Render each flag as a separate compact text badge:

| Existing flag | Badge | Role |
|---|---|---|
| `FLAG_DUP_IP` | `dup IP` | Error |
| `FLAG_NO_PARENT` | `no parent` | Warning |
| `FLAG_RATE` | `bad rate` | Error |

Keep all simultaneous flags. Use plain separators between badges.
Do not color the entire row as a warning.
Keep full flag details in the circuit panel when the table cell is too narrow.

The title must distinguish device rows from unique circuits.
Show `rows <filtered>/<total>` and `circuits <filtered distinct>/<total distinct>` when space permits.
Count distinct `circuit_id` values. Do not treat each device row as a circuit.

Calculate these counts when data or the filter changes, outside the draw loop.
At narrow widths, keep the circuit total and put other counts in the detail panel.
Keep the search query and sort indicator visible in the panel or its summary line.

Keep the current side-by-side breakpoint of 100 columns.
Below it, keep the table above the circuit detail panel.
At 80 columns, show Circuit Name, IPv4, Down min/max, Up min/max, and Flags.
Move the other column values to the existing detail panel, which retains every field.

Do not change the sort cycle when a column is hidden.

## 6. Network — B7

Replace the `+` / `-` tree prefix with `├─`, `└─`, and ancestor `│` guides.
Compute sibling continuation from the currently visible tree.
Handle multiple roots, nested last children, collapsed nodes, search results, and a scrolled window.
Do not infer continuation from depth alone.

Use `▸` for a collapsed branch and `▾` for an expanded branch after the connector.
Leaves have no disclosure marker.
Keep Space, Left, Right, `e`, `c`, and search behavior unchanged.
Keep selection by node name across refreshes.

Show the existing node type as text.
Use accent for `site`, OK for `ap`, warning for `plan`, and secondary text for an unknown type.
Preserve the current default of `site` when the source has no type.

Show a short configured-rate bar per node when horizontal space permits.
Label the pair `D/U Mbps`. This is configured capacity, not live utilization.
Use download capacity for the bar value.

Use the highest positive finite download capacity in the loaded tree as the shared scale.
Label the tree summary `bar: download / max <N> Mbps`.
Compute the scale on data change. Keep it stable across collapse, search, and scroll.

Missing or invalid rates show `unknown` without a filled bar.
A zero rate shows zero without a filled bar. Avoid division by zero when all rates are zero.
Keep upload capacity as a numeric value beside download capacity.

Hide the bar before the node name when space is short. Keep exact rates in the detail panel.

## 7. Journal — B8

Style the timestamp and hostname as secondary text.
Keep the message and program prefix at normal brightness, with the current severity role.
Preserve every character of the original log line.

Identify spans only for a recognized `journalctl -o short-iso` prefix.
For banners, malformed prefixes, or other formats, render the complete raw line without a metadata split.
Do not change `spawn_journal`, `level_of`, filtering, or raw line storage.
Do not insert badge characters into copied text.

Selection style must override the secondary metadata style where needed for readability.
Keep warning and error messages legible in all themes.

## 8. Dashboard — B9

Replace the section text layout with six independently bordered cards.
Use the full body width rather than the Devices main/side split.

| Card | Values from the existing Dashboard |
|---|---|
| Service | Service state, mode, target, and current action context |
| Last cycle | Time, age, duration, sequence, trigger, write/reload result, errors, and Telegram result |
| Routers | Name, state, row count, read duration, and error |
| Counts | Added, updated, removed, held, duplicates, and guard alerts |
| Versions | LibreQoS, lqosd, service/TUI versions, mismatch warning, and config-load state |
| Process | PID, start, elapsed, CPU, RSS, gauges, and existing history |

At 120 columns or more, use three columns and two card rows.
Place Service, Last cycle, and Routers in the first row.
Place Counts, Versions, and Process in the second row.
At 80–119 columns, use two columns and three card rows, in the same reading order.

Use a one-cell horizontal gutter. Share vertical borders or use no vertical gutter at minimum height.
At 80×24 with a one-row footer, the body has 20 rows.
Use card-row heights of 6, 7, and 7, including borders.

Keep the Process card at seven rows so its five inner rows fit both gauges and traces.
If extra banners reduce body height, keep the card title and highest-priority values.

At minimum height, use compact summaries for Last cycle, Versions, and Process.
Keep service state, last error, guard alerts, router failures, CPU, RSS, and both history traces.
Put errors before routine detail. Ellipsize long values by display width.

For a long router list, show failed/offline routers first within the visible summary and show `+N more`.
This display order must not change the stored router order or worker behavior.
At larger heights, display all existing diagnostic fields as space permits.
Do not add navigation keys, scrolling controls, or new dialogs for the cards.

Use green for `online`, yellow for `failed`, red for `offline`, and secondary text for `unknown`.
Keep `failed` and `offline` distinct: a failed router keeps subscribers under the current policy.
Always show the state word. Color alone does not carry the meaning.

### Process gauges and history

Reuse the existing `DashboardView` histories and 120-sample limit.
Do not change the sampler or add `/proc` reads.
Keep each gauge beside its numeric value and its sparkline.

- CPU: use 100% of one core as the gauge scale. Clamp the fill to 0–100%.
- If CPU exceeds 100%, keep the full numeric value and show `>100% of one core`.
- Clamp only the drawn CPU trace to that scale. Keep the stored samples unchanged.

- RSS: use the maximum RSS in the stored history as the gauge scale.
- Label RSS `current / window peak`, in MiB. This is not a memory limit or host-memory percentage.
- A zero RSS peak produces an empty bar. Do not divide by zero.

- Keep CPU history in tenths of a percent and RSS history in kB internally.
- Label CPU sparkline scale and RSS window peak. Use the same scale for a gauge and its corresponding history.
- The first unknown CPU sample says `unknown`. Do not add a synthetic zero sample.

- If current process data is absent, show current values as `unknown` and label stored traces `history only`.

Do not add CPU/RSS alarm thresholds. The existing data defines no such policy.

## 9. Settings — B10

Render a changed key name in the warning style.
Keep the `*` and original-value text so mono users can identify changes.
Render inherited and default values as secondary text, including their existing origin labels.
Render read-only keys in grey, with `(read-only)` text.

Selection and focus must remain clear above all these styles.

Add a changed-key count to each group title, for example `runtime (2)`.
Omit `(0)` for unchanged groups.
Count distinct concrete changed paths that belong to the group.
Match path segments, not raw prefixes: `routers.1` must not include `routers.10`.

Include changed paths for removed children in the nearest surviving owning group.
Count a changed path once per group. Do not sum overlapping group counts into the global total.
Use existing draft changes. Do not infer changes by comparing displayed strings.

Keep staged secret counts separate, as the current header does.
Do not put secret values or hashes in labels, tooltips, or tests.
Refresh counts when the draft changes, including undo, reset, add, remove, and apply completion.

Apply the shared panel and dialog styles to all Settings states.
Include DHCP clients, servers, secret prompts, validation, preview, apply, errors, and discard prompts.
Keep the current three-panel wide layout and stacked narrow layout.
Do not change draft ownership, validation order, or the apply state machine.

## 10. Implementation handoff

This section gives the implementation order. It is not authorization to deploy.
Read `brain/Home.md`, `brain/HANDOFF.md`, and this spec before edits.
Check the branch, diff, and active agent ownership again.

| Order | Files | Deliverable |
|---|---|---|
| 1 | `src/config.rs`, `config/defaults.toml`, `src/tui/settings/meta.rs`, `src/tui/theme.rs`, `src/tui/mod.rs` | Typed theme, defaults, metadata, shared styles, startup wiring |
| 2 | `src/tui/app.rs` | Header pills, tab styles, key chips, minimum-size screen |
| 3 | `src/tui/devices.rs` | Zebra rows, alignment, badges, circuit counts, compact columns |
| 4 | `src/tui/network.rs` | Tree guides, types, configured-capacity bars |
| 5 | `src/tui/journal.rs` | Metadata spans with raw-copy preservation |
| 6 | `src/tui/dashboard.rs` | Six cards, process gauges, labeled existing histories |
| 7 | `src/tui/settings/view.rs`, `docs/CONFIGURATION.md` | Changed groups, field styles, dialogs, theme documentation |

Pass the resolved theme to view renderers explicitly.
Keep rendering helpers free of file I/O, subprocesses, and configuration writes.
Avoid a generic widget framework or a large Settings refactor.
Update tests that assert old decoration, including the Network assertion containing `+ Site`.

Preserve tests of behavior and unknown-value semantics.

## 11. Acceptance and verification

Use ratatui `TestBackend` for text, geometry, styles, and selection checks.
Text-only snapshots cannot prove colors or modifiers.
Use existing fixture data. Do not connect to routers or signal the service for these checks.

| ID | Acceptance proof |
|---|---|
| A1 | All tabs and dialogs use shared rounded panels, accent titles, and semantic roles |
| A2 | Active/failed, apply/shadow, target, age, next scan, unknown, and read-only pills render correctly |
| A3 | Each existing tab/input/modal shows its valid key chips with correct case and visible completion/cancel keys |
| A4 | 79×24, 80×23, 1×1, and 0×0 show safe fallback; 80×24 restores saved state and mouse geometry |
| A5 | Omitted theme defaults to dark; all three values load; invalid values fail; metadata exposes the choice |
| B6 | Zebra parity survives scroll; rates align right; all badges coexist; two devices in one circuit count as one circuit |
| B7 | Connectors handle roots, nested siblings, collapse, search, and scroll; bar scale stays stable and labels show configured rates |
| B8 | Metadata dims; message severity remains; malformed lines survive; copied text exactly equals the original text |
| B9 | Six cards fit at 80×24 and 120×40; gauge labels/scales handle unknown, zero, CPU above 100%, and absent process data |
| B10 | Changed/default/read-only fields remain distinct; group counts handle undo, nested removals, and router index boundaries |

Also check Unicode names and empty data.
Check stale files, loader errors, and all three themes.
Use display-cell widths rather than UTF-8 byte lengths for clipping and hit rectangles.
Make sure draw work stays bounded by the visible rows and existing history limits.

Keep the 20,000-row performance test and rerun it after table changes.

Run these gates during implementation:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test --release --lib tui::devices::tests::perf_20k_rows_key_press_under_16ms -- --ignored
```

All automated gates must exit with code 0.
Review every tab and representative Settings dialogs in a real terminal at 80×24, 100×30, and 120×40.
Check dark, light, and mono. Inspect actual palette contrast, selection, borders, and Unicode alignment.

Record visual checks separately from automated checks.
If a real terminal check cannot run, record that limit in the implementation handoff.
Do not claim implementation acceptance from this design document alone.
