---
type: handoff
created: 2026-10-31
tags: [handoff]
status: active
related: ["[[2026-10-31-project-setup-and-v3-simulation]]"]
---

# Handoff

## 2026-10-31 (Asia/Shanghai) — documentation phase

| Item | Value |
|------|-------|
| Goal | Rust replacement for `updatecsv.py` with TOML settings, for LibreQoS 2.2 and 3.x |
| Last agent | Claude Code (Opus 5.5) |
| Directory | `/home/pi/LQoSync3.0` (repository root) |
| Repository | `https://github.com/p33ckab00/LQoSync.git`, branch `lqosync3` (no shared history). The crate, `docs/`, and `brain/` are at the root. |
| Backup | `/home/pi/LQoSync3.0-backup-2026-10-31.tar.gz` (the files before the first move) |
| Phase | Plan 1 (core) and Plan 2 (runtime) done. The binary has `check`, `once`, `run`, `diff`, `defaults`. Plans 2b, 3, 4, 5 are not written. |
| Checks | `cargo test` (88 + 1 + 1), `cargo clippy --all-targets -D warnings`, `cargo fmt --check`. `tools/v3sim/check.sh`: the v3 3.0.7 compiler accepts all nine scenario outputs. |

### Files

- `README.md`
- `docs/ARCHITECTURE.md`, `docs/CONFIGURATION.md`, `docs/INSTALLATION.md`, `docs/OPERATIONS.md`, `docs/LIBREQOS-V3-INGRESS.md`
- `brain/` vault: [[ADR-0001-one-model-many-writers]], [[ADR-0002-v3-target-selection]], [[libreqos-v3-ingress-facts]], [[v3-offline-simulation-method]]
- `graphify-out/`: knowledge graph of the documents and the vault

### Decisions that the operator made

- [[ADR-0003-router-failed-vs-offline]]: a failed router keeps its subscribers. An offline router loses them after `offline_after` (default 300 seconds, confirmed by the operator).
- [[ADR-0004-toml-policy-python-defaults]]: all policy is in TOML. The defaults give the `updatecsv.py` output.
- Name: branch, crate, binary, and service are `lqosync3`. Config: `/etc/lqosync3/lqosync3.toml`. State: `/var/lib/lqosync3`.
- PPPoE speed order: comments first, then `profile_rate_limit`, then `profile_name`, then `default`.
- [[ADR-0005-topology-follows-v3]]: the nodes follow the v3 rules: stable node `id`, `Parent Node ID`, 14/15 CSV columns, max >= 0.2, `flat_promotion` parent fix. Open for a v3 test shaper: `compile_mode` effect and empty Parent Node.
- [[ADR-0006-v3-guards-and-node-types]]: `no_parent` with a v3 target fails `check`. A `compile_mode` other than `full` gives a warning. On v3, group nodes are type `site`.
- [[ADR-0007-fresh-design-not-a-port]]: a new design. `updatecsv.py` is a guide only. No byte parity. Scenario tests in place of golden tests.
- Speed rules: default `rate_limit_order = "routeros_rx_tx"`, `unitless_unit = "bps"` (RouterOS-correct). The operator selected this.
- Removal guard on: hold a cycle that removes more than 20% or more than 50 rows.
- [[ADR-0008-terminal-ui]]: `lqosync3 tui` with monitor, actions, and a config editor (Plan 4).

### Decisions that the operator must make

None at this time.

### Next action

1. Cutover done on 2026-10-31 15:36 UTC: `lqosync3.service` operates in `apply` mode and writes the live LibreQoS files. `lqosync-updatecsv.service` (Python) is stopped and disabled. Backup of the last Python files: `/var/backups/lqosync/20261031T153517Z`. Rollback: OPERATIONS.md, "Rollback to the Python service".
2. Plan 2b (Telegram) is done and operates on the shaper: `[telegram] enabled = true`, chat `810741035`, token in `/etc/lqosync3/secrets/telegram.token`. The test message arrived at 15:24 UTC.
3. Plan 3 (LibreQoS v3) is done in code: `src/detect.rs`, `src/writer/v3network.rs`, `lqosync3 detect`. The production shaper (2.2, fixed `target = "v2_pair"`) does not use it yet. The `v3_network` output passed the offline 3.0.7 parser (spec "Offline check result"). Open: a test on an operating v3 shaper, the `lqosd` accept check (`v3_status_timeout`), `libreqos-v3-preflight`.
4. Plan 4a (TUI, read-only screens and actions) is done in code: `lqosync3 tui`, `src/tui/`. The service writes `status.json` version 3 and `snapshot.json`. The installed binary on the shaper is still the Plan 3 build: deploy the new build to get the new status fields. Plan: `docs/superpowers/plans/2026-10-31-lqosync3-tui-4a.md`.
5. Plan 4b (Settings editor) is done in code: `src/tui/settings/` (meta, draft, validate, secrets, preview, apply, view). The first real apply is on the shaper by the operator. Plan: `docs/superpowers/plans/2026-11-01-lqosync3-tui-4b.md`.
6. Plan 4c (DHCP client overrides) is done in code: `[[routers.dhcp.clients]]` by MAC, `routers.dhcp.shape_unassigned`, the shared lease list `sources::dhcp_leases`, and the DHCP view of the Settings tab. Spec: `docs/superpowers/specs/2026-11-01-lqosync3-dhcp-clients-design.md`.
7. Next: the minors A and B of the owner list, then TUI cosmetics, then Plan 5 (installer).

Router library: `mikrotik-rs =0.9.0` (`mikrotik-tokio` 0.3.0) in this project only. The other project in `/opt/LQoSync3.0/v3` is not changed.

Plan 2 and defaults review minor items: fixed on 2026-10-31 (one test for each). TCP keepalive is not possible: `mikrotik-tokio` 0.3 makes the TCP socket and has no keepalive option. In its place, a kept session that fails gets one reconnect at once, and this reconnect does not use a retry.

Deferred minor items from the Plan 2b review (2026-10-31):
- The error memory uses the full error text. If LibreQoS stderr changes in each run (time, PID), each reload retry sends a new alert. A duplicate-IP alert with a count that changes does the same. Fix: compare only the first line, or a key without numbers.
- `include_computation` was removed. An old config with this key does not load (`deny_unknown_fields`). The live config does not have it.
- `post_json` cannot read a chunked answer. If Telegram sends one, the send waits until `timeout` and logs a false failure.

Deferred minor items from the Plan 3 review (2026-10-31):
- Rows of one Circuit ID with different parents or rates: `v3_network` uses the first row and gives no alert.
- A dpkg version with an epoch (`1:3.0.7`) is "not supported" (fails safe).
- A `v3_network` skip alert uses the Telegram title "removal guard held a write".
- `diff` when the detection fails: it compares the pair files.
- `v3_network` with `preserve_network_config = true` reads the old `network.json` each cycle, but does not write it.

Deferred minor items from the Plan 4a review (2026-10-31):
- The action wait (`tui.apply_verify_timeout = 120`) can end before a long v2 cycle (`libreqos.v2_timeout = 300`) ends. The TUI then shows "no result" and the action completes later.
- Just after a service restart, the old `pid` in `status.json` gives a false "the service restarted" message for an action.
- The digest check compares with the `--config` text of the TUI. That file can be a different file from `config_path`, or empty if the TUI cannot read it.
- A `status.json` read or parse error shows no reason (all values show `unknown`).
- IPv4 sorts as text (`10.0.0.10` before `10.0.0.9`).
- The Network detail "circuits" counts device rows.
- The perf test does not time the name sort or the full `App::draw`.
- The Journal selection anchor moves when old lines drop from a full buffer.

Deferred minor items from the Plan 4b review (2026-11-01):
- `r`, `u`, and `P` act on the first key of a group while the focus is on the groups panel.
- Config backups are named by second: two applies in one second overwrite one backup.
- The temporary file of `write_secret` has a fixed name, no `O_EXCL`/`O_NOFOLLOW`, and stays after an error. `password_file` paths are not checked.
- Each `v` starts a new preview thread, also while one runs.
- A bad number closes the field and loses the typed text.
- A list move that changes nothing, and `20` against `20.0`, count as changes.
- During `Loaded`, the header says "waiting" while the footer says "applied".
- The preview does not warn about routers that were failed, unreachable, or offline in the snapshot.
- Each draw of the Settings tab calls `text()` and `diff_lines()`: cache them on a change.
- `add_server` and `remove_server` change the draft before their index checks.
- `Secret` push and pop, and `SecretUndo`, can leave secret bytes in freed memory.

Settings: all defaults are in `config/defaults.toml` (embedded). Plan 5 must install a `/etc/lqosync3/lqosync3.toml` with `[[routers]]` and comments that point to the defaults file.

### Production state

- `lqosync-updatecsv.service` (Python) is stopped and disabled since 2026-10-31 15:35 UTC.
- `lqosync3.service` operates in `apply` mode since 2026-10-31 15:36 UTC. It writes `/opt/libreqos/src/ShapedDevices.csv` and `network.json`, and runs `LibreQoS.py --updateonly`. Config: `/etc/lqosync3/lqosync3.toml`. Secret: `/etc/lqosync3/secrets/FSEHub-Distro.password` (`0600` root). Refer to [[2026-10-31-first-live-shadow-run]].
- LibreQoS 2.2 operates. Nobody changed it.
