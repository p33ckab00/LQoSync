---
type: decision
created: 2026-10-31
tags: [adr, tui]
status: accepted
supersedes:
related: ["[[ADR-0007-fresh-design-not-a-port]]"]
---

# ADR-0008 — A terminal UI for monitor, actions, and config edits

## Context

The operator wants a terminal UI. The sync service operates under systemd with no terminal.

## Decision

1. Add the command `lqosync3 tui`. Use `ratatui` 0.30 and `crossterm` 0.29.
2. The TUI is a separate process. The service writes `<state_dir>/status.json` with a temporary file and a rename after each cycle. The TUI reads it.
3. Actions: "sync now" sends `SIGUSR1` to the service (a cycle with `force = true`). "Reload config" sends `SIGHUP`.
4. Screens: dashboard (router states, counts, cycle time, guard), subscribers (search, rates, parent, router), node tree, changes, shadow diff, log.
5. Config editor: forms for routers, speed rules, and the other TOML sections. Before a save, the TUI runs the same config check as `lqosync3 check`. A save writes a backup, then a temporary file, then a rename. A password goes to its `*_file` with mode `0600`, never into the TOML file.
6. The TUI is Plan 4. It starts after the runtime (Plan 2) exists.

## Alternatives

1. A local socket API between the TUI and the service. It gives live data, but it adds a server and access control.
2. A TUI inside the service. The service then needs a terminal.

## Consequences

- The config editor must keep the comments and the order of the TOML file. Use `toml_edit` in Plan 4.
- Only `root` can use the actions and the editor, because the service and the files belong to `root`.

## Status
accepted on 2026-10-31 (operator selected "monitor + actions + config editor")
