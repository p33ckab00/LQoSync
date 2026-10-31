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

1. Shadow service on the production shaper: `lqosync3.service` operates in `shadow` mode since 2026-10-31 14:51. After 2026-11-01 14:51, run `sudo lqosync3 diff`. If the result is the same as the first run (one known difference), do the cutover to `apply` mode (OPERATIONS.md, "Cutover from the Python service"). The cutover needs the operator's approval.
2. Plan 2b (Telegram) is done in code: `src/notify.rs`. To use it on the shaper, install the new binary, write `/etc/lqosync3/secrets/telegram.token`, and set `[telegram] enabled = true` with the chat ID. Refer to OPERATIONS.md, "Turn on Telegram".
3. Plan 3: LibreQoS v3 detection and writers.

Router library: `mikrotik-rs =0.9.0` (`mikrotik-tokio` 0.3.0) in this project only. The other project in `/opt/LQoSync3.0/v3` is not changed.

Plan 2 and defaults review minor items: fixed on 2026-10-31 (one test for each). TCP keepalive is not possible: `mikrotik-tokio` 0.3 makes the TCP socket and has no keepalive option. In its place, a kept session that fails gets one reconnect at once, and this reconnect does not use a retry.

Deferred minor items from the Plan 2b review (2026-10-31):
- The error memory uses the full error text. If LibreQoS stderr changes in each run (time, PID), each reload retry sends a new alert. A duplicate-IP alert with a count that changes does the same. Fix: compare only the first line, or a key without numbers.
- `include_computation` was removed. An old config with this key does not load (`deny_unknown_fields`). The live config does not have it.
- `status.json` `report.updated` changed to `[[name, [columns]]]`, but `version` is still 1.
- `post_json` cannot read a chunked answer. If Telegram sends one, the send waits until `timeout` and logs a false failure.

Settings: all defaults are in `config/defaults.toml` (embedded). Plan 5 must install a `/etc/lqosync3/lqosync3.toml` with `[[routers]]` and comments that point to the defaults file.

### Production state

- `lqosync-updatecsv.service` (Python) operates and writes the live LibreQoS files. Nobody changed it.
- `lqosync3.service` operates in `shadow` mode. It writes only `/var/lib/lqosync3/`. Config: `/etc/lqosync3/lqosync3.toml`. Secret: `/etc/lqosync3/secrets/FSEHub-Distro.password` (`0600` root). Refer to [[2026-10-31-first-live-shadow-run]].
- LibreQoS 2.2 operates. Nobody changed it.
