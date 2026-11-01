---
type: handoff
created: 2026-10-31
tags: [handoff]
status: active
related: ["[[2026-10-31-project-setup-and-v3-simulation]]"]
---

# Handoff


## 2026-11-01T08:26:04+00:00 — btop TUI, review fixes, new LibreQoS installation

| Item | Value |
|------|-------|
| Goal | Make the TUI like btop++, fix the review findings, and support a new LibreQoS installation |
| Last agent | Claude Code (Opus 5.5), with Sonnet reviewers |
| Branch and source revision | `lqosync3`, `fa7ffd9` before the brain notes |
| Worktree | `/home/pi/LQoSync3.0` |
| Changed files | `src/tui/*`, `src/runner.rs`, `src/cli.rs`, `src/config.rs`, `config/defaults.toml`, docs, specs, brain |
| Checks | 437 library tests and all integration tests passed. Clippy and fmt passed. Final reviews: PASS. |
| Deploy | The operator installed the release build and restarted the service. |
| Unfinished work | The approved process chart (CPU, RSS, I/O, network). Real-terminal review of the TUI. |
| Next action | Build the process chart. I/O and network need `IOAccounting=yes` and `IPAccounting=yes` on the service. |

Session: [[2026-11-01-btop-tui-and-fresh-install]].
Decision: [[ADR-0010-first-full-libreqos-run]].

## 2026-11-01T12:45:09+08:00 (Asia/Shanghai) — TUI cosmetics implementation

| Item | Value |
|------|-------|
| Goal | Implement the approved cosmetics plan for all A1–A5 and B6–B10 items |
| Last agent | Codex, with one fresh read-only reviewer |
| Branch and source revision | `lqosync3`, `7ca02c9` |
| Worktree | `/home/pi/LQoSync3.0` |
| Changed files | Theme config/defaults/docs, shared palette, shell, all five view modules, metadata, tests, and these notes |
| Checks | 430 tests passed, 2 intentionally ignored. Clippy, fmt, release timing, debug build, and PTY smoke passed. |
| Review | Three compact-layout findings and one popup canvas finding fixed with regression tests. No deferred code findings. |
| Unfinished work | Human inspection of palette contrast and glyphs on the operator terminal |
| Next action | Review the TUI in the operator terminal. Deploy only when the operator requests it. |

Spec: `docs/superpowers/specs/2026-11-01-lqosync3-tui-cosmetics-design.md`.
Plan: `docs/superpowers/plans/2026-11-01-lqosync3-tui-cosmetics.md`.
Session: [[2026-11-01-tui-cosmetics-implementation]].
No production file, service state, dependency, or status schema changed.
The project graph remains at the earlier baseline.

## 2026-11-01T12:03:25+08:00 (Asia/Shanghai) — TUI cosmetics plan

| Item | Value |
|------|-------|
| Goal | Prepare the implementation plan, review it against the spec, then commit and push the documents |
| Last agent | Codex |
| Branch and baseline | `lqosync3`, `cf2c959` before the documentation commit |
| Worktree | `/home/pi/LQoSync3.0` |
| Changed files | Cosmetics spec and plan, this handoff, `brain/Home.md`, and the cosmetics session note |
| Review | PASS for plan coverage. All A1–A5 and B6–B10 requirements have implementation and test owners. |
| Checks | Fresh TUI baseline: 160 passed, 0 failed, 1 ignored. Documentation structure, links, and diff checks. |
| Unfinished work | Source implementation, full implementation gates, and real-terminal checks |
| Next action | Read the spec and plan, then implement after the operator authorizes source changes |

Plan: `docs/superpowers/plans/2026-11-01-lqosync3-tui-cosmetics.md`.
The operator authorized the documentation commit and push after a successful plan review.
No source changes or deployment form part of this step.
The commit that contains this entry identifies the published documentation revision.

## 2026-11-01T11:54:22+08:00 (Asia/Shanghai) — TUI cosmetics design

| Item | Value |
|------|-------|
| Goal | Complete the cosmetics design spec for all A1–A5 and B6–B10 items |
| Last agent | Codex |
| Branch and baseline | `lqosync3`, `cf2c959` |
| Worktree | `/home/pi/LQoSync3.0` |
| State at takeover | Clean working tree. No cosmetics spec or implementation changes. Claude Code remained open, without a visible build or edit subprocess. |
| Changed files | New cosmetics spec, this handoff, `brain/Home.md`, and the linked session note |
| Checks | TUI baseline: 160 passed, 0 failed, 1 ignored. STE heuristic review and manual requirement review. |
| Unfinished work | Review and implementation of the design. Full build gates and terminal visual checks belong to implementation. |
| Next action | Read the spec and confirm current ownership before implementation edits. Preserve existing keys and the apply flow. |

Spec: `docs/superpowers/specs/2026-11-01-lqosync3-tui-cosmetics-design.md`.
Session: [[2026-11-01-tui-cosmetics-design-handoff]].
No source code or production state changed during this takeover.
The older sections below contain historical context. Verify their claims against current files.

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

Deferred minor items from the Plan 2b review (2026-10-31; the error memory and chunked answer items were fixed on 2026-11-01):
- `include_computation` was removed. An old config with this key does not load (`deny_unknown_fields`). The live config does not have it.

Deferred minor items from the Plan 3 review (2026-10-31):
- Rows of one Circuit ID with different parents or rates: `v3_network` uses the first row and gives no alert.
- A dpkg version with an epoch (`1:3.0.7`) is "not supported" (fails safe).
- A `v3_network` skip alert uses the Telegram title "removal guard held a write".
- `diff` when the detection fails: it compares the pair files.
- `v3_network` with `preserve_network_config = true` reads the old `network.json` each cycle, but does not write it.

Deferred minor items from the Plan 4a review (2026-10-31):
- Just after a service restart, the old `pid` in `status.json` gives a false "the service restarted" message for an action.
- The digest check compares with the `--config` text of the TUI. That file can be a different file from `config_path`, or empty if the TUI cannot read it.
- A `status.json` read or parse error shows no reason (all values show `unknown`).
- IPv4 sorts as text (`10.0.0.10` before `10.0.0.9`).
- The Network detail "circuits" counts device rows.
- The perf test does not time the name sort or the full `App::draw`.
- The Journal selection anchor moves when old lines drop from a full buffer.

Deferred minor items from the Plan 4b review (2026-11-01):
- Each draw of the Settings tab calls `text()` and `diff_lines()`: cache them on a change.
- `add_server` and `remove_server` change the draft before their index checks.
- `Secret` push and pop, and `SecretUndo`, can leave secret bytes in freed memory.

Deferred minor items from the Plan 4c review (2026-11-01):
- The clients group shows "active lease" also for a lease that is skipped (excluded IP, not bound).
- A router that is online with DHCP off, or a draft that does not parse, gives misleading texts in the clients group and the DHCP note.
- A real DHCP server named `unassigned` shares the node of the `unassigned` group.
- The warning "N DHCP leases have no server" is not in OPERATIONS.md.

Deferred minor items from the review of the minor fixes (2026-11-01):
- A chunked Telegram answer whose last chunk has an extension or trailers (`0;x=y`) is read only at the close (as before the fix).
- The Telegram error memory merges errors that differ only in digits or after the first line ("router 1 down" and "router 2 down").
- A TUI that is killed between the temporary secret file and the rename leaves a `.<name>.tmp-<pid>-<n>` file (mode 0600).

Settings: all defaults are in `config/defaults.toml` (embedded). Plan 5 must install a `/etc/lqosync3/lqosync3.toml` with `[[routers]]` and comments that point to the defaults file.

### Production state

- `lqosync-updatecsv.service` (Python) is stopped and disabled since 2026-10-31 15:35 UTC.
- `lqosync3.service` operates in `apply` mode since 2026-10-31 15:36 UTC. It writes `/opt/libreqos/src/ShapedDevices.csv` and `network.json`, and runs `LibreQoS.py --updateonly`. Config: `/etc/lqosync3/lqosync3.toml`. Secret: `/etc/lqosync3/secrets/FSEHub-Distro.password` (`0600` root). Refer to [[2026-10-31-first-live-shadow-run]].
- LibreQoS 2.2 operates. Nobody changed it.
