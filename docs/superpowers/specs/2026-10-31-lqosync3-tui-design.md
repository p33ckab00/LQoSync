---
type: spec
created: 2026-10-31
status: for owner approval
related: ["[[ADR-0008-terminal-ui]]", "[[2026-10-31-lqosync3-runtime-design]]", "[[2026-10-31-lqosync3-telegram-design]]"]
---

# lqosync3 terminal UI (Plan 4)

## 1. Intent

The operator wants one terminal screen to watch and control lqosync3 on the shaper, over SSH. The look follows btop (dark panels with borders, compact cards, small graphs). The system of btop is not copied.

The most important parts are correct settings changes, the ShapedDevices table, and the network topology. A settings change must never surprise the operator: each change shows its effect before it is applied, and the tool never reports a false success.

## 2. Goals, non-goals, anti-goals

Goals:
- Five tabs, selected with the mouse or the keys `1`–`5`.
- A settings editor for all keys of the config, with a dropdown for each value that has fixed choices.
- A read-only ShapedDevices table and a network tree.
- A journal view with line selection and copy.
- A dashboard with the LibreQoS version and the PID, start time, elapsed time, memory, and CPU of the lqosync3 service.

Non-goals:
- Edit one subscriber row in the TUI. The routers are the source of the rows.
- Change MikroTik settings.
- Change the cycle rules (rates, nodes, removal guard).

Anti-goals (a failure even if the code works):
- A settings change that is applied without a preview and a confirmation.
- A "success" message when the service did not load the new config.
- A secret on the screen, in the clipboard, or in a log.
- A TUI that the service needs: the service must operate the same with no TUI.
- Calculation code in the TUI that is a copy of the cycle code.

## 3. Constraints

- `ratatui` 0.30 and `crossterm` 0.29 (ADR-0008). `toml_edit` for the config file, so that comments and key order stay.
- The TUI is the command `lqosync3 tui` in the same binary. It is a separate process from the service.
- Root is necessary for the settings editor, the preview (`snapshot.json` is `0600`), and the actions. Without root, the TUI opens read-only and shows a banner. `ShapedDevices.csv`, `/etc/lqosync3/lqosync3.toml`, and `status.json` are `0644`, and the `adm` group can read the journal, so viewing does not need root.
- A password goes only into its `*_file` with mode `0600` and owner root, never into the TOML file and never on the screen.
- All defaults in `config/defaults.toml`, also the new TUI keys (section 9).
- The production shaper operates in `apply` mode. A change to the service for the TUI must not change the cycle output.

## 4. Approaches

1. **Independent operation handler** (a root daemon or a systemd unit for each operation, a socket, durable operation IDs, a state machine). Rejected. Apply takes less than one second: one atomic write and one `SIGHUP`. ADR-0008 rejected a local socket API.
2. **Simple apply in the TUI with a loaded-config digest** (selected). The TUI validates, previews, confirms, writes the file atomically, sends `SIGHUP`, and waits until `status.json` shows the new config digest. If the TUI stops between the write and the signal, the file on disk and the loaded config are different, and the TUI and `lqosync3 check` show this at the next start.
3. **Preview from new router reads in the TUI.** Rejected. The TUI would open a second session to each router, and two reads at different times do not give a fair comparison.

## 5. Changes to the service (Plan 4a)

1. `status.json` gets these fields:
   - `config_path` and `config_digest`: `fnv1a64` of the bytes of the config file that the service loaded, as 16 hex characters. A reload with an error keeps the old digest (the old config stays).
   - `cycle_seq`: a number that increases by one for each cycle of this process. `pid`: the process ID.
   - `trigger`: why the status was written: `start`, `timer`, `sync_now` (`SIGUSR1`), or `reload` (`SIGHUP`).
   - `program_version`: the version of the service binary (`CARGO_PKG_VERSION`).
   - `telegram`: the result of the last send: `disabled`, `ok`, or the error text.
   - The service writes `status.json` at once after a reload (good or bad), before the next cycle starts. A cycle can take up to `libreqos.v2_timeout`, so the digest must not wait for it.
2. After each router read, the service writes `<state_dir>/snapshot.json` (mode `0600`): for each router, its result (`online` with the tables, or `failed`, `unreachable`, `offline`) and the time of the read. It holds subscriber data, but no secret.
3. `lqosync3 check` reports a warning if the digest of the file on disk is different from `config_digest` in `status.json`.

## 6. Screens

Common parts:
- Header: service state, mode, target, last cycle age, next scan.
- Tabs: `1 ShapedDevices`, `2 Network`, `3 Settings`, `4 Journal`, `5 Dashboard`. The TUI opens on ShapedDevices.
- Footer: the keys of the current screen.
- A tab keeps its focus, selection, filter, and scroll position when the operator goes to another tab.
- On a narrow terminal, the side panels go under the main panel.
- Color shows state, but each state also has a word (`online`, `failed`).

### 6.1 ShapedDevices

- A table of the live `ShapedDevices.csv` (in `shadow` mode, the candidate file): Circuit Name, Circuit ID, Device Name, MAC, IPv4, Download/Upload Min and Max, Parent Node, Comment.
- From `state.json`: the source (PPPoE, Hotspot, DHCP, static) and the router of each circuit.
- Search (`/`) on name, IP, and MAC. Sort by a column. The selection stays on the same circuit after a refresh.
- A detail panel for the selected circuit. Flags: a duplicate IP, a parent that is not in `network.json`, a rate that LibreQoS rejects.
- `Enter` on a circuit opens its parent node in the Network tab.

### 6.2 Network

- Left: the tree of `network.json`, expandable (`Space`), with expand all and collapse all.
- Right: the selected node: type, `id`, rates, number of child nodes, number of circuits, and the circuits under it.
- Search on node names.

### 6.3 Settings

Layout: the groups on the left (the TOML sections and each router), the keys of the group in the center, help on the right.

For each key:
- The value: current (from the file, or "default"), and the new value if changed.
- Input: a toggle for a boolean, a dropdown for a value with fixed choices, a number field with range check, a text field, or a list editor for an array.
- Help: the comment of the key in `config/defaults.toml`, the default value, and the range.
- `r` resets the key to the default: the editor removes the key from the file.

Lists:
- `[[routers]]`: add, remove, and edit a router. A new router asks for a password (refer to "Secrets").
- `[[routers.dhcp.servers]]`: add, remove, and edit a server entry.

Secrets (builder's call, settled in review):
- A new router asks for a password (masked input). "Set new password" is also available for an existing router and for the Telegram token.
- The typed secret stays in memory until Apply. Validate treats a secret file that the operator typed as present.
- Apply writes the secret file first (default `/etc/lqosync3/secrets/<name>.password`, mode `0600`, owner root), then the TOML file. If the apply stops after the secret write and before the TOML write, the TUI removes a secret file that it made new in this apply, and restores a secret file that it replaced.
- An existing secret file is replaced only when the operator typed a new secret for it.
- A rename of a router does not move its secret file: `password_file` is a normal key.
- The removal of a router does not delete its secret file. The confirmation names the file, so the operator can delete it by hand.
- Arrays of values (for example `exclude.ips`, `speed.pppoe_sources`): add, remove, and move items. An array of fixed choices uses a dropdown for each item.

Apply procedure (no key skips a step):
1. **Edit.** Changes stay in memory. The tab shows the number of changed keys. `u` undoes the last change. Leaving the TUI with changes asks for a confirmation.
2. **Validate.** `Config::from_toml_str` on the full new text, the same check as `lqosync3 check` (also the secret files and the paths). An error names the key and blocks the next step.
3. **Preview.**
   - The diff of the TOML file (old and new lines).
   - If a key that changes the output changed: run `cycle::assemble` two times on the same `snapshot.json`, `state.json`, and live files, one time with the old config and one time with the new config. Show the result as a ShapedDevices diff (added, removed, changed circuits with `old -> new` values) and a node diff, with counts.
   - If `snapshot.json` is older than `tui.preview_max_age` seconds, or a router has no snapshot, the preview says so. For a router with no snapshot (for example a new router), the preview shows "effect of router X is not known".
   - If the preview cannot run, it shows the reason. It never shows "0 changes" for a preview that did not run.
4. **Confirm.** A modal shows the number of changed keys, the circuit and node counts, the secret files that will be written, and all warnings. The operator types `y`.
5. **Apply.**
   - Read the file again. If its digest is different from the digest at the start of the edit, stop: another person changed it. Show the new diff.
   - Copy the old file to `<state_dir>/config-backups/<unix-seconds>.toml` (keep `runtime.keep_backups`).
   - Write the new text with `files::write_if_changed` (temporary file, fsync, rename; mode and owner stay).
   - Send `SIGHUP` to the MainPID of `lqosync3.service`.
6. **Verify.**
   - Wait until `status.json` shows `trigger = reload` with the new `config_digest`, at most `tui.apply_verify_timeout` seconds (the service writes it at once after the reload).
   - If `config_error` is set, show it: the old config stays.
   - Then show "cycle running" until a status with a higher `cycle_seq`, then its result: the router states, the counts, and any error.
   - If the time ends, show "not verified" and the two digests. Never show "applied" without the new digest.

`runtime.state_dir` is read-only in the editor (builder's call, settled in review). A new folder has no `state.json`, so the service would start again from the live files and lose the circuit sources, the router memos, the guard hold, and the backups. The help text tells the operator to change it by hand and to copy the folder first. So the TUI never restarts the service.

Keys that change the output: all keys except `telegram.*`, `runtime.log_level`, and the `tui.*` keys. The builder can make this list smaller if a test shows that a key has no effect on `assemble`.

### 6.4 Journal

- The last `tui.journal_lines` lines of `journalctl -u lqosync3.service`, then new lines while the tab is open.
- Filter by level (error, warning, info). Search. Pause and follow.
- Select lines with the mouse or with `Shift` and the arrows. `y` copies them with OSC 52.
- A terminal does not confirm an OSC 52 copy. So the TUI says "sent to the terminal clipboard (not confirmed); if it did not arrive, press s and select with the terminal". `tui.osc52 = false` turns OSC 52 off (builder's call, settled in review).
- `s` (select mode) releases the mouse to the terminal, so the normal terminal selection works. `s` again takes the mouse back.
- The process arguments are a list, never a shell text.

### 6.5 Dashboard

- Cards: service state (systemd), mode, target, last cycle (time, duration, result), routers (state, rows, read time), counts of the last cycle, removal guard state, last Telegram send, reload result, and the trigger of the last cycle.
- Versions: LibreQoS (the installed `libreqos` package and the state of `lqosd`), lqosync3 (`program_version` of the service, and the TUI version; a difference means that the service needs a restart).
- Process (MainPID from systemd): PID, start time, elapsed time, RSS memory, CPU percent. The CPU percent is sampled between two refreshes.
- Small graphs of CPU and memory from the samples of this TUI session only. The graph starts empty.
- The config digest check: "loaded" or "the file on disk is not loaded".
- An unknown value shows `unknown`, never `0`.

## 7. Actions

- `S` sync now: send `SIGUSR1` (force cycle, releases a guard hold). Asks for a confirmation, because it releases a hold.
- `R` reload: send `SIGHUP`.
- Each action shows its result from `status.json`, not from the signal call: `S` waits for a status with `trigger = sync_now` and a higher `cycle_seq`. `R` waits for `trigger = reload`.

## 8. Data sources and refresh

| Data | Source | Refresh |
|------|--------|---------|
| Status | `<state_dir>/status.json` | `tui.refresh_ms` |
| Rows | `<lqos_dir>/ShapedDevices.csv` or the candidate | when the file time changes |
| Tree | `network.json` (pair targets) | when the file time changes |
| Circuit source | `<state_dir>/state.json` | when the file time changes |
| Preview input | `<state_dir>/snapshot.json` | at each preview |
| Journal | `journalctl` | while the tab is open |
| Process | `/proc/<MainPID>/stat`, `/proc/<MainPID>/status` | `tui.refresh_ms` |
| Versions | dpkg status file, `systemctl is-active lqosd` | at start and each minute |

- File reads and process calls run outside the draw loop. A slow read never stops the screen.
- A file that is too large (more than `tui.max_file_mb`) shows an error, not a partial table.
- The `v3_network` target has no pair files. The ShapedDevices and Network tabs then read `libreqos_network.json`.

## 9. New config keys (`[tui]` in `config/defaults.toml`)

- `refresh_ms = 1000`
- `journal_lines = 2000`
- `apply_verify_timeout = 120` (seconds)
- `preview_max_age = 300` (seconds)
- `max_file_mb = 50`
- `osc52 = true`

## 10. Phases

- **Plan 4a:** the service changes (section 5), the TUI frame, the tabs, the ShapedDevices, Network, Journal, and Dashboard screens, and the actions. All read-only except the actions.
- **Plan 4b:** the Settings screen: the key metadata, the editor, the lists, validate, preview, confirm, apply, verify.

## 11. Tests

- Key metadata: each of the 10 enums has a list of choices, and a test checks that each choice loads and that a wrong value fails. Each key of `config/defaults.toml` has an editor type.
- Editor: a change keeps the comments and the key order of the file. Reset removes the key. Add and remove of a router and of a DHCP server.
- Preview: the same snapshot with the same config gives no change. A rate change gives the expected circuit changes. An old snapshot gives the stale warning. A failed preview never shows 0 changes.
- Apply: a changed file on disk stops the apply. A config error shows `config_error` and no success. A time-out shows "not verified".
- Screens: render tests with the ratatui test backend for each tab, a narrow terminal, and unknown values.
- Terminal: the terminal is restored after a panic.
- Secrets: a typed secret never appears in the screen buffer, the TOML text, the preview, or a log. A stopped apply removes or restores the secret files.
- No test connects to a router, to `/run/lqos/bus`, or writes outside a temporary folder.

## 12. Left to the builder

- The exact layout sizes, colors, and key names other than the ones above.
- The structure of the TUI modules.
- The journal reader method (`journalctl` options).
- The diff view style.
