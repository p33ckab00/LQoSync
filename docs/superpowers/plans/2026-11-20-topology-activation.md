# Topology Manager activation implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Topology Manager the only writer after a gated `topology activate`, keep the last good pair on a manager failure, and give a `topology deactivate` rollback that keeps the IDs.

**Architecture:** `runtime-owner.json` is written only by `activate` and removed only by `deactivate`. `Runner::cycle` keeps all parts that do not write the pair, and in topology mode it replaces the legacy write step with `sync_topology`. A failed manager write leaves the pair on disk and sets new status fields, a health component, and one notice per state change.

**Tech Stack:** Rust 2024, `tokio`, `serde_json`, `anyhow`. No new dependency.

**Spec:** `docs/superpowers/specs/2026-11-20-topology-activation-design.md`

## Global Constraints

- Rust 1.89 compatible. No new dependency.
- Routers stay read-only.
- One writer: in topology mode the legacy write step never runs, also on a manager failure.
- Window commands (`activate`, `deactivate`) need the instance lock. Without `--yes` they are a dry run with exit status 2. A refusal is exit status 1 and writes nothing.
- `runtime-owner.json` content stays `{"schema_version": 1, "writer": "topology"}`. Only `activate --yes` writes it. Only `deactivate --yes` removes it.
- New `status.json` fields: `writer` (`"topology"` | `"legacy"`), `manager_failed` (bool), `manager_error` (string or null), `manager_failed_since` (unix seconds or null).
- Health: `Domain::Apply` component `writer`: `Healthy`/`topology`, `Healthy`/`legacy`, `Degraded`/`manager_failed`.
- Notices: one when the manager starts to fail, one when it works again, through `crate::notify::outbox::enqueue`. None on each failed cycle. None after a restart that keeps the same state.
- `activate` prints `activated: the Topology Manager is the writer`.
- Persisted text (docs, comments, commits) in ASD-STE100.

## Review Focus

- The service restarts while the manager fails: no second "failing" notice, and `manager_failed_since` keeps the first time. Test in Task 3.
- An invalid `runtime-owner.json` (`topology_owns` is `Err`): the cycle writes nothing and never runs the legacy write. Test in Task 3.
- A manager failure after a partial step (compile passed, admission refused): the pair files on disk keep their bytes. Test in Task 3.
- `deactivate` on a pair with a `tm-sub-...` circuit: the next legacy cycle keeps that Circuit ID and Device ID. Test in Task 4.
- `activate` when a legacy row has no alias (a client came back with a new legacy ID): refused, and the message names the row and says to run `propose --replace` and `enroll --replace`. Test in Task 2.

---

### Task 1: Owner claim, live-test revision, and the default mode

**Files:**
- Modify: `src/runner.rs` (`publish_compiled`, around line 1720: remove the `runtime::claim` call)
- Modify: `src/topology_manager/runtime.rs` (module comment; `claim` stays for Task 2; add `pub(crate) fn release(state: &Path) -> anyhow::Result<()>` that removes the owner file and fsyncs the directory)
- Modify: `src/runner/live_test.rs` (`LiveTestReport` gets `pub registry_revision: Option<u64>`, set from the registry document at the prepare step)
- Modify: `config/defaults.toml` (`[network] mode = "root_promotion"`, keep the comment list of modes) and the config docs or example configuration that show the default
- Test: existing tests in `src/runner.rs` that assert a claim after `sync_topology` (around line 3318) change to assert no claim; `runtime.rs` tests; live-test tests; a config default test

**Interfaces:**
- Produces: `runtime::claim(state)` (existing) and `runtime::release(state)`; `LiveTestReport.registry_revision`.

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test] async fn sync_topology_does_not_claim_the_writer()    // after a successful sync_topology, topology_owns is Ok(false)
#[test] fn release_removes_the_owner_file()                            // claim, release: topology_owns Ok(false); release without a file is Ok
#[tokio::test] async fn the_live_test_report_records_the_registry_revision() // report.registry_revision == Some(document revision)
#[test] fn the_default_network_mode_is_root_promotion()               // Config defaults: network.mode == NetworkMode::RootPromotion
```

- [ ] **Step 2: Run them and make sure that they fail**

Run: `CARGO_BUILD_JOBS=1 nice -n 19 cargo test --locked --lib -- sync_topology_does_not_claim release_removes registry_revision default_network_mode`
Expected: FAIL or compile errors.

- [ ] **Step 3: Implement**

- [ ] **Step 4: Run the full checks** (the four commands in Task 5 Step 5). Expected: all pass and clean.

- [ ] **Step 5: Commit** `feat(topology): claim the writer only by activation`

### Task 2: `topology activate`

**Files:**
- Modify: `src/cli.rs` (`TopologyCommand::Activate { report: PathBuf, yes: bool }`, parse `activate --report <file> [--yes]`, usage text)
- Modify: `src/topology_cli.rs` (print the gate result, exit status 2 for a dry run, 1 for a refusal)
- Modify: `src/runner.rs` (`pub async fn activate_topology(&mut self, report: &Path, max_age: Duration, install: bool) -> anyhow::Result<ActivationOutcome>`)
- Test: `src/runner/tests/activation.rs` (new, registered next to `enrollment.rs`), `cli.rs` parser test

**Interfaces:**
- Consumes: `runtime::claim` (Task 1), `LiveTestReport.registry_revision` (Task 1), `Runner::prepare_topology_for(max_age, true)`, `topology_manager::enrollment::read_live_pair`, the enrollment parity (`migration::compare_mapped` through the same path `enroll_with_mapping` uses), `ControlledCompiler::open` + `check_config` + `compile` as `NativeIo::admit` does.
- Produces: `pub struct ActivationOutcome { pub activated: bool, pub rounded: Vec<..same type as EnrollOutcome.rounded..>, pub multi_identity: usize }`.

Gates in this order (spec section 1). Error texts:

| Gate | Error |
|---|---|
| not enrolled | `the topology registry is not enrolled` |
| owner file present | `the Topology Manager is already active` |
| a legacy row without an alias | `legacy row <circuit_id> / <device_id> has no alias; run topology propose --replace and topology enroll --replace` |
| parity difference | `the manager pair differs from the legacy pair: <differences>` |
| LibreQoS not ready or compile failure | the underlying error, with context `the LibreQoS compiler check failed` |
| report missing, not passed, or other revision | `the live test report does not prove the current registry revision <N>` |

`multi_identity` is the count of registry subscribers with more than one identity.

- [ ] **Step 1: Write the failing tests** in `src/runner/tests/activation.rs` with the enrollment fixture:

```rust
#[test] fn activate_dry_run_writes_nothing()                  // gates pass, install=false: activated false, no owner file
#[test] fn activate_writes_the_owner_file()                   // install=true: topology_owns Ok(true)
#[test] fn activate_refuses_a_registry_that_is_not_enrolled()
#[test] fn activate_refuses_when_already_active()
#[test] fn activate_refuses_a_legacy_row_without_an_alias()   // change one legacy Circuit ID in the live CSV: error names it and says "propose --replace"
#[test] fn activate_refuses_a_report_of_another_revision()    // report registry_revision = current - 1, and passed=false: both refused
```

Use a fake compiler step as the live-test tests do, so no `python3` runs. Plus a parser test for `activate --report F --yes`.

- [ ] **Step 2: Run them and make sure that they fail**

Run: `CARGO_BUILD_JOBS=1 nice -n 19 cargo test --locked --lib -- activation cli`
Expected: FAIL or compile errors.

- [ ] **Step 3: Implement**

- [ ] **Step 4: Run the full checks**. Expected: all pass and clean.

- [ ] **Step 5: Commit** `feat(topology): add the gated activate command`

### Task 3: The cycle in topology mode

**Files:**
- Modify: `src/runner.rs` (`cycle` / `cycle_inner`: the write step split; status fields; restart state)
- Modify: `src/status.rs` (the four new `Status` fields, serde default for old files)
- Modify: `src/write_receipt.rs` or the health publish point (the `writer` component)
- Test: `src/runner/tests/topology_mode.rs` (new)

**Interfaces:**
- Consumes: `Runner::sync_topology` (existing), `runtime::topology_owns`, `crate::notify::outbox::enqueue(dir, tg, text, now)`.
- Produces: `Status { writer: String, manager_failed: bool, manager_error: Option<String>, manager_failed_since: Option<u64> }`.

Rules (spec section 2):

1. `topology_owns` `Ok(true)`: run every part of `cycle_inner` up to the router collection, the collection report, and the Settings snapshot as today. Skip `assemble`, the legacy `state.json` write, the legacy file write, and the legacy import-wait block. Run `sync_topology` with a `ControlledCompiler` that the runner opens once and keeps (open it again after a `check_config` error).
2. Manager success: `writer = "topology"`, `manager_failed = false`, clear the error fields. The write receipt gets the same `Frozen` change notice as a legacy write.
3. Manager `Err`: no file write. `writer = "topology"`, `manager_failed = true`, `manager_error = format!("{e:#}")`, `manager_failed_since` = the first failed cycle time.
4. A router read failure in topology mode: keep the pair, report it in `status.routers` and the collection report as today. It does not set `manager_failed`.
5. `Ok(false)`: legacy cycle as today, `writer = "legacy"`.
6. `Err`: keep today's behavior (write nothing, report the error). Never run the legacy write.
7. Notices: when `manager_failed` changes false to true, enqueue `lqosync3: the Topology Manager failed; the last pair stays. <error>`. When it changes true to false, enqueue `lqosync3: the Topology Manager works again.` At service start, read `manager_failed`, `manager_failed_since`, and `writer` from the previous `status.json`, so a restart does not count as a change.

- [ ] **Step 1: Write the failing tests** in `src/runner/tests/topology_mode.rs`, with the enrollment fixture plus a claimed owner file and a fake compiler:

```rust
#[tokio::test] async fn topology_mode_writes_the_manager_pair_once()        // legacy assemble not called (no state.json circuit change), writer "topology", health writer Healthy/topology
#[tokio::test] async fn a_manager_failure_keeps_the_pair_and_reports_it()    // fake admission refusal: pair bytes unchanged, manager_failed, manager_error set, health writer Degraded/manager_failed
#[tokio::test] async fn the_next_good_cycle_clears_the_failure()             // failure then success: manager_failed false, two notices total
#[tokio::test] async fn a_restart_during_a_failure_sends_no_second_notice()  // new Runner reads status.json: one failing notice only, manager_failed_since kept
#[tokio::test] async fn an_invalid_owner_file_writes_nothing()               // owner file with writer "other": no pair write, no legacy write
#[tokio::test] async fn a_router_failure_is_not_a_manager_failure()          // fake router error: pair kept, manager_failed false
```

- [ ] **Step 2: Run them and make sure that they fail**

Run: `CARGO_BUILD_JOBS=1 nice -n 19 cargo test --locked --lib -- topology_mode`
Expected: FAIL or compile errors.

- [ ] **Step 3: Implement**

- [ ] **Step 4: Run the full checks**. Expected: all pass and clean.

- [ ] **Step 5: Commit** `feat(topology): write only the manager pair after activation`

### Task 4: `topology deactivate`

**Files:**
- Modify: `src/cli.rs` (`TopologyCommand::Deactivate { yes: bool }`, parse `deactivate [--yes]`)
- Modify: `src/topology_cli.rs`
- Modify: `src/runner.rs` (`pub fn deactivate_topology(&mut self, install: bool) -> anyhow::Result<DeactivationOutcome>`)
- Modify: `src/state.rs` only if a helper is needed to write circuits and nodes
- Test: `src/runner/tests/activation.rs`

**Interfaces:**
- Consumes: `runtime::release` (Task 1).
- Produces: `pub struct DeactivationOutcome { pub deactivated: bool, pub seeded: usize, pub skipped_multi_identity: usize }`.

Steps of `deactivate --yes` (spec section 3): read the pair on disk, write each row into `state.json` `circuits` under the legacy key, update the node entries, then `runtime::release`. The legacy key is the circuit name that the legacy `assemble` gives that account: reuse the naming code that `assemble` uses (`crate::config::CircuitNames` and the source naming), do not write a second rule. A subscriber with more than one identity or device is skipped and counted. Refuse when no owner file exists: `the Topology Manager is not active`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn deactivate_dry_run_changes_nothing()                       // owner file stays, state.json unchanged
#[test] fn deactivate_seeds_the_legacy_state_and_releases()           // owner file gone; state.json has the manager Circuit ID and Device ID of each row
#[tokio::test] async fn the_legacy_writer_keeps_a_tm_sub_id_after_deactivate() // next legacy cycle: CSV row of that account keeps tm-sub-... ids
#[test] fn deactivate_refuses_when_not_active()
#[test] fn deactivate_counts_multi_identity_subscribers()             // a subscriber with two identities: skipped_multi_identity == 1
```

- [ ] **Step 2: Run them and make sure that they fail**

Run: `CARGO_BUILD_JOBS=1 nice -n 19 cargo test --locked --lib -- activation`
Expected: FAIL or compile errors.

- [ ] **Step 3: Implement**

- [ ] **Step 4: Run the full checks**. Expected: all pass and clean.

- [ ] **Step 5: Commit** `feat(topology): add the deactivate rollback`

### Task 5: API, TUI, and the runbook

**Files:**
- Modify: `src/runner.rs` (`api_status`: add `manager_failed`, `manager_error`; `writer` from the last cycle status)
- Modify: the TUI status screen (`src/tui/`, the file that shows the cycle status): one line `writer: topology` or `writer: legacy`, and during a failure `manager failed <age>: <error>`
- Create: `docs/operations/TOPOLOGY_ACTIVATION.md` (the window: pre-checks, backup, stop, `topology live-test`, `activate --report <path>` dry run then `--yes`, start, observe three cycles, `deactivate` rollback, rollback to an older build)
- Modify: `brain/30-knowledge/topology-manager-status.md` (a Phase 4 entry)
- Test: the API test file (`src/runner/tests/api.rs`) and the TUI test of that screen

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn api_status_reports_the_manager_failure()   // status with manager_failed: JSON has manager_failed true and manager_error text
#[test] fn the_status_screen_shows_the_writer()        // TUI render contains "writer: topology"; with a failure contains "manager failed"
```

- [ ] **Step 2: Run them and make sure that they fail**

Run: `CARGO_BUILD_JOBS=1 nice -n 19 cargo test --locked --lib -- api_status the_status_screen`
Expected: FAIL.

- [ ] **Step 3: Implement, and write the runbook and the status entry**

- [ ] **Step 4: Run the STE checker on the new runbook**: `python3 ~/.claude/skills/asd-ste100/scripts/ste_check.py docs/operations/TOPOLOGY_ACTIVATION.md`. Fix the real findings.

- [ ] **Step 5: Run the full checks**

```bash
CARGO_BUILD_JOBS=1 nice -n 19 cargo test --locked
cargo fmt --check
CARGO_BUILD_JOBS=1 nice -n 19 cargo clippy --locked --all-targets -- -D warnings
git diff --check
```

Expected: all pass and clean.

- [ ] **Step 6: Commit** `feat(topology): show the writer and add the activation runbook`
