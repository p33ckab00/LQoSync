# lqosync3 runtime design (Plan 2)

This document adds the runtime decisions to the core design (`2026-10-31-lqosync3-core-design.md`).
The core stays a pure function. Plan 2 adds the parts that touch routers, files, the clock, and signals.

## 1. Scope

| In Plan 2 | Not in Plan 2 |
|-----------|---------------|
| Router reads (`mikrotik-rs`), TLS, timeouts, retries, router states | Telegram (Plan 2b) |
| Files: lock, live inputs, first-run seed, state, atomic writes, backups, `shadow` and `apply` | v2/v3 detection and v3 writers (Plan 3). `target = "auto"` means `v2_pair`. A v3 target stops `check` with "not yet". |
| LibreQoS 2.x reload (`v2_command v2_args`) | TUI (Plan 4) |
| `status.json`, CLI `check`, `once`, `run`, `diff`, `defaults`, signals, systemd unit | Installer (Plan 5) |

## 2. Dependencies

`tokio` (rt-multi-thread, macros, time, signal, process, fs, sync), `mikrotik-rs = "=0.9.0"` with `tokio-tls`, `rustls = 0.23` (`ring`), `rustls-pemfile = 2`, `webpki-roots = 1`.
`mikrotik-rs` 0.7.0 is not allowed: its `mikrotik-tokio` 0.1.0 cancels a command when more than 16 replies arrive in one burst, so a large table comes back partial. 0.9.0 (`mikrotik-tokio` 0.3.0) uses an unbounded channel. A test with a mock router proves that 300 rows arrive.
`rust-version = "1.89"` (for `std::fs::File::try_lock`).

## 3. Router reads

- A `RouterSession` trait has one method: `print(path, proplist) -> Result<Vec<RosRow>, ReadError>`. The tool only sends `<path>/print` with `=.proplist=`. It never sends a command that changes the router.
- Paths and fields (only for enabled sources):

| Path | `.proplist` |
|------|-------------|
| `/ppp/secret` | `name,profile,comment,disabled,caller-id` |
| `/ppp/active` | `name,address,caller-id,comment` |
| `/ppp/profile` | `name,comment,rate-limit` |
| `/ip/hotspot/active` | `user,address,mac-address` |
| `/ip/dhcp-server` | `name,comment,disabled` |
| `/ip/dhcp-server/lease` | `server,address,active-address,mac-address,host-name,comment,status` |

- An attribute with no value becomes `""`. `!empty` gives an empty table. `!trap` and `!fatal` are read errors.
- Each call has `router_access.api_timeout`. A connect (TCP, TLS, login) has `router_access.connect_timeout`.
- A failed read: close the session, wait `retry_delay`, connect again, and read all tables again. Up to `read_retries` more attempts.
- Sessions stay open between cycles. All routers are read at the same time (one task for each router).

## 4. Router state

| Outcome of the cycle | `RouterResult` | `RouterMemo.unreachable_since` |
|----------------------|----------------|-------------------------------|
| All tables read | `Online(snapshot)` | `None` |
| Login refused, TLS error, `!trap`, timeout, or read error after all attempts | `Failed` | unchanged |
| TCP connect refused or timed out, `unreachable_since = None` | `Unreachable` | `Some(now)` |
| Same, and `now - since < offline_after` | `Unreachable` | unchanged |
| Same, and `now - since >= offline_after` | `Offline` | unchanged |

A TLS certificate error is `Failed`, not `Unreachable`, because it is a config problem. The report gets a warning with the reason.

## 5. TLS

- New router keys: `tls_verify = true`, `tls_ca_file = ""` (in `[router_defaults]`).
- `tls = true`, `tls_verify = true`: verify the router certificate. With `tls_ca_file`, use only that CA (PEM). Without it, use the `webpki-roots` CAs.
- `tls = true`, `tls_verify = false`: encrypt, but accept any certificate. Each connect writes a warning.
- `tls = false`: plain API. `check` writes a warning (the password goes in clear text).

## 6. Files

All paths come from the config. `<state>` is `runtime.state_dir`. `<lqos>` is `libreqos.directory`.

| File | Use |
|------|-----|
| `<state>/lqosync3.lock` | `File::try_lock`. A second process stops with "another lqosync3 is running". |
| `<state>/state.json` | The core state. Atomic write. |
| `<state>/status.json` | The status for the TUI (section 7). Atomic write after each cycle. |
| `<state>/candidate/ShapedDevices.csv`, `network.json` | The output in `shadow` mode. |
| `<lqos>/ShapedDevices.csv`, `<lqos>/network.json` | Live input each cycle (static rows, preserve tree). The output in `apply` mode. |
| `<state>/applied/<UTC time>/` | In `apply` mode: the old LibreQoS files before each write. Keep the last 10 folders. |

- First run (no `state.json`): `seed_state` from the live CSV rows that are not static, and `seed_nodes` from the live `network.json`.
- A cycle writes nothing if `assemble` returns an error. The status shows the error.
- Write order: `state.json` first, then the LibreQoS files. If the process stops between them, the next cycle gives the same files from the same IDs.
- A file is written only if its bytes change. Atomic write: a temporary file in the same folder, `sync_all`, rename, then `sync_all` on the folder.
- `apply` mode: if `ShapedDevices.csv` or `network.json` changed, run `v2_command` with `v2_args`, time limit `v2_timeout`. A failure goes into the status and the report. The files stay.

## 7. Status file

`status.json` holds: `version`, `updated` (Unix time), `mode`, `target`, `cycle_ms`, `next_scan_seconds`, `routers` (name, state, error, row counts for each table, `read_ms`), `report` counts and lists (`added`, `updated`, `removed`, `held`, `duplicates`, `warnings`, `alerts`), `files_changed`, `reload` (`skipped` | `ok` | error text), `error` (cycle error or `null`).

## 8. Process

- `run`: loop. Each cycle: lock is held, read routers, `assemble`, write, reload, status. Sleep `scan_interval_active` after a change, else `scan_interval_idle`, after an error `error_retry_interval`.
- `SIGUSR1`: start a cycle now with `force = true` (it releases a guard hold).
- `SIGHUP`: read the config again. If the new config fails `check`, keep the old one and write the error to the status.
- `SIGTERM`, `SIGINT`: finish the current cycle, then stop.
- `once [--force]`: one cycle, then stop. Exit code 0 on success, 1 on a cycle error.
- `check`: load the config, read each secret file (mode `0600`, owner `root`), check that `<state>` is writable and that `<lqos>` exists, warn about a router with no enabled source, a plain API router, and `tls_verify = false`. Exit code 0 or 1.
- `diff`: compare `<state>/candidate/*` with the live files: circuits added, removed, and changed (field by field), and nodes added, removed, and changed. Text output.
- `defaults`: print the embedded `config/defaults.toml`.
- Log lines go to stderr with a level word. systemd stores them in the journal.

## 9. Secrets

`password_file` is read each connect. The file must exist, have no group or other permission bits, and belong to `root`. The tool trims one final newline. A secret is never written to a log, the status, or the state.

## 10. Tests

- Pure parts (router state, retries with a fake session, diff, status shape) have unit tests.
- A mock RouterOS server (a TCP listener that speaks the API words) tests the real `mikrotik-rs` session: login, `.proplist`, 300 rows in one write, `!empty`, `!trap`, and a stalled reply that must hit `api_timeout`.
- File tests use temporary folders. The runner test runs two cycles with fake sessions and checks the files, `state.json`, `status.json`, the lock, and the backups.
- A manual step reads the production router in `shadow` mode. It needs the operator's approval and a read-only RouterOS user.
