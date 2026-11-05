# E01 — LQoSync3 source and output baseline

**Base commit:** `f358e9b5b4b64ba59919caa7b54d808b6345f7db` on `lqosync4`. This commit is the source baseline before E01 edits. LQoSync4 is the planned web application; this checkout still runs the LQoSync3 CLI and TUI. This report describes observed source behavior, not a completed LQoSync4 API or UI.

## Current execution path

| Boundary | Existing source | Behavior to preserve or assess in LQoSync4 |
|---|---|---|
| CLI and service loop | [`main.rs`](../../../src/main.rs), [`cli.rs`](../../../src/cli.rs) | `check`, `detect`, `once`, `run`, `diff`, `defaults`, `version`, and `tui`. `run` schedules cycles; SIGHUP reloads config and SIGUSR1 forces a cycle. `once` and `run` use `Runner`. |
| Configuration | [`config.rs`](../../../src/config.rs), [`defaults.toml`](../../../config/defaults.toml) | Merge documented defaults with TOML, reject unknown keys, validate router and target settings. Passwords and tokens use file paths. |
| Target and existing state | [`detect.rs`](../../../src/detect.rs), [`runner::cycle_inputs`](../../../src/runner.rs), [`files.rs`](../../../src/files.rs) | Resolve fixed or installed LibreQoS target each cycle. Read live shaping files and saved `state.json`; seed first state from live input. |
| Router collection | [`routeros.rs`](../../../src/routeros.rs), [`mikrotik.rs`](../../../src/mikrotik.rs), [`routerstate.rs`](../../../src/routerstate.rs) | Read enabled PPP secret/active/profile, Hotspot active, and DHCP server/lease tables. Reuse sessions, bound each read with timeout/retries, and classify online/failed/unreachable/offline. No RouterOS write path exists. |
| Circuit model | [`sources.rs`](../../../src/sources.rs), [`cycle::assemble`](../../../src/cycle.rs), [`validate::check`](../../../src/validate.rs) | Convert PPPoE, Hotspot and DHCP records to circuits; preserve static rows, stable IDs, grace and removal guard; build topology; reject invalid rows before output. |
| LibreQoS output | [`writer/pair.rs`](../../../src/writer/pair.rs), [`writer/v3network.rs`](../../../src/writer/v3network.rs), [`runner.rs`](../../../src/runner.rs) | `v2_pair` and `v3_pair` render `network.json` plus `ShapedDevices.csv`; `v3_network` renders schema-2 `libreqos_network.json`. Shadow writes candidates only. Apply backs up changed live files and writes each changed file through a temporary file and rename. |
| Import and status | [`runner.rs`](../../../src/runner.rs), [`files::imported`](../../../src/files.rs), [`status.rs`](../../../src/status.rs) | v2 invokes a reload command; v3 pair requests scheduler reload and compares imported topology; native JSON is watched by `lqosd` but has no equivalent accepted-generation readback. `status.json` records cycle result; `snapshot.json` contains subscriber reads with mode 0600. |

The two pair files are replaced **one at a time**, not as one atomic transaction. The runner writes state first and has tests for restart and partial pair repair. A native JSON write is not proof of LibreQoS acceptance. [The v3 data-flow map](../docs/LIBREQOS_V3_DATA_FLOW.md) has the target-specific details.

## TUI removal boundary

The TUI is still wired through `Command::Tui` in `cli.rs`, the TUI branch in `main.rs`, `pub mod tui` in `lib.rs`, `Config::tui` and TUI defaults, `src/tui/`, and `ratatui` in `Cargo.toml`. Phase 1 C01 must inspect each reference before deleting it. The sync CLI, state/status files, configuration validation, and shaping cycle are core behavior, not TUI behavior.

## Reused synthetic fixtures

| Coverage | Fixture or test | Baseline evidence |
|---|---|---|
| PPPoE, Hotspot, DHCP and v2 pair | [`tests/scenarios`](../../../tests/scenarios) `flat_*`, `grouped_router_root`, `no_parent`, `root_promotion` | Config + RouterOS snapshot + prior CSV produce frozen CSV, network JSON and report. Source names, MACs and addresses are synthetic. |
| Failed/missing router | [`failed_router`](../../../tests/scenarios/failed_router) second snapshot; `routerstate` and `runner` tests | A failed read retains prior rows; unreachable/offline transitions are tested in Rust. The scenario records `failed`, not a live network outage. |
| Removal guard and restart | [`guard_hold`](../../../tests/scenarios/guard_hold), [`cycle.rs`](../../../src/cycle.rs), [`runner.rs`](../../../src/runner.rs) tests | Guard hold, state continuity and restart behavior. |
| v3 pair | [`v3_root_promotion`](../../../tests/scenarios/v3_root_promotion) | Frozen 15-column CSV, network JSON and report. Runner tests cover import pending, delayed import and repair after a partial pair update. |
| v3 native | [`libreqos_network.json`](../../../tests/scenarios/v3_root_promotion/expected/libreqos_network.json) and [`libreqos_network_alerts.json`](../../../tests/scenarios/v3_root_promotion/expected/libreqos_network_alerts.json) | New golden output and alert from the same synthetic v3 model. Seven circuits are rendered. The static `fixed-ap` circuit is skipped because it has no parent; the alert is part of current behavior, not an approved production outcome. |
| RouterOS protocol | [`mock_router.rs`](../../../tests/mock_router.rs) | Local mock server tests reads, errors and reconnect behavior without a live router. |

The fixtures contain placeholder password-file paths, not passwords or tokens. Do not copy production `snapshot.json`, customer CSV or credentials into the repository. The native golden fixture freezes the renderer's output for migration comparison; it does not approve native apply. Apply now rejects that output when any circuit is skipped, while shadow mode retains the candidate and alerts. Set an explicit valid Parent Node for a static circuit before applying. If a later fix changes this output, review the semantic difference before updating the golden file. The installed native probe and accepted-generation check remain separate gates.

## Reproduce and interpret

From the repository root, with Rust 1.89 or a compatible toolchain and a C compiler/linker:

```bash
git rev-parse HEAD
cargo test --locked
cargo test --locked --test scenarios
cargo test --locked --test mock_router
```

Do **not** set `LQOSYNC3_BLESS=1` during parity checks; that variable replaces expected files. `tests/v3_offline.rs::pair_to_libreqos_network` is an ignored manual export tool and is not part of the normal test pass. None of these commands contacts production RouterOS or writes live LibreQoS input. The scenario harness writes generated comparisons under `target/scenario-out/`.

At the pinned commit, the full test suite passed on this host: 479 passed, 5 ignored, 0 failed. After the E01 native golden check was added, the full suite passed again with the same counts. `cargo fmt --all -- --check` also passed after the edit. This host lacked Rust and a C compiler initially; the run used temporary Rust 1.89 and Zig 0.13 wrappers outside the repository. No production router or shaper was tested.

## Phase 1 handoff

Use this baseline to remove TUI and retain the core. Compare every target's output against the frozen fixtures. Preserve shadow/apply and unchanged-write behavior. Keep the v3 native production gate closed until its node-rate probe and accepted-generation verification pass. E03 performance measurements and G0 web-framework/store/auth decisions are separate tasks; this E01 report supplies their source and fixture starting point.
