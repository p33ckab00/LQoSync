# Planner admission and live acceptance implementation plan (Phase 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an offline planner admission check and a one-shot `topology live-test` command that writes the manager pair to LibreQoS, reads back the planner, dataplane, and native acknowledgement, and restores the legacy pair byte for byte, plus the window runbook.

**Architecture:** A pure `topology_manager::live_check` module models the observed LibreQoS rules, checks implicit commitments, parses the fresh LibreQoS artifacts, and diffs them against the model. A `live_test` orchestrator in the runner runs the steps with injected file, import, and acknowledgement ports, so tests use a fake LibreQoS state directory. The CLI adds two commands. A runbook in `docs/operations/` lists the window steps.

**Tech Stack:** Rust 2024, Tokio, serde_json. No new dependencies. MSRV 1.89.

**Spec:** `docs/superpowers/specs/2026-11-19-topology-live-acceptance-design.md`

## Global Constraints

- Do not stop, restart, deploy, or reconfigure `lqosync3.service`, and do not write `/etc/lqosync3`, `/var/lib/lqosync3`, `/opt/libreqos/src`, `/opt/libreqos/state`, or routers during the build. Reading LibreQoS state files to learn their format is allowed. The live window is a separate, owner-approved step after the merge.
- Do not copy production files into the repository. Test fixtures are synthetic, with the same JSON shape as the production artifacts and invented names, IDs, and addresses.
- No new dependencies. Keep `cargo +1.89 build --locked` working.
- Memory on the host is tight: `nice -n 19`, `CARGO_BUILD_JOBS=1`, tests with `-- --test-threads=2`. Gate for every task: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked -- --test-threads=2`.
- Separate worktree, branch `topology-live-acceptance` from `lqosync3`. Do not edit `docs/OPERATIONS.md`.
- Persisted text follows ASD-STE100.
- The live test never calls `runtime::claim`, never writes `runtime-owner.json`, never uses the write receipt tracker, never commits the registry, and sends no notification.
- Rule model (spec Facts): node commitment = explicit minimum, else floor(sum of child commitments in whole Mbps); circuit minimum rounded up to whole Mbps; node ceiling rounded up to whole Mbps; circuit ceiling clamped to the parent ceiling (a warning, not a difference).
- After each pair write, write the import request with `files::request_v3_import`.

## Review Focus

1. The process is killed between the manager write and the restore: `live-test-restore --backup <dir>` brings back the byte-identical legacy pair. Test in Task 3.
2. LibreQoS never becomes `ready` with a new generation: the import timeout restores the legacy pair and the report says "import timeout". Test in Task 3.
3. A manager pair that loses a circuit against the legacy pair: no live write at all. Test in Task 3.
4. A stale artifact (for example `planner_state.json` from 2026-11-02) next to fresh ones: the parser reads only the fresh artifacts and ignores the stale ones. Test in Task 1.
5. The native acknowledgement is unavailable: the test fails with "native acknowledgement unavailable" and restores. Test in Task 3.

---

### Task 1: Rule model, implicit commitments, and the artifact parser

**Files:**
- Create: `src/topology_manager/live_check.rs`, `src/topology_manager/live_check_tests.rs` (register in `mod.rs`), synthetic fixtures under `tests/fixtures/live_state/`
- Test: `src/topology_manager/live_check_tests.rs`

**Interfaces:**
- Produces:
  - `pub struct Expected { pub circuits: BTreeMap<String, ExpectedCircuit>, pub nodes: BTreeMap<String, ExpectedNode> }` with kbps values, built by `pub fn expected(rows: &[CsvRow], network: &Value) -> anyhow::Result<Expected>` from the rule model.
  - `pub fn admission(expected: &Expected) -> Vec<String>`: nodes whose sibling commitments exceed the parent ceiling, as "<node>: commitments <n> kbps exceed ceiling <m> kbps".
  - `pub struct Observed` parsed by `pub fn read_observed(state: &Path) -> anyhow::Result<Observed>` from `topology/topology_runtime_status.json`, `topology/topology_import.json`, `shaping/topology_compiled_shaping.json`, `shaping/shaping_inputs.json`, and `shaping/dataplane_transaction.json` only.
  - `pub struct Diff { pub differences: Vec<String>, pub warnings: Vec<String> }` from `pub fn compare(expected: &Expected, observed: &Observed) -> Diff`.

- [ ] **Step 1: Write the failing tests:** `implicit_node_commit_is_the_floored_child_sum`, `circuit_minimum_and_node_ceiling_round_up`, `a_circuit_above_its_parent_is_clamped_with_a_warning`, `admission_names_a_node_whose_commitments_exceed_its_parent`, `the_parser_reads_only_fresh_artifacts` (Review Focus 4), `compare_finds_missing_unexpected_and_changed_entries`, `a_matching_state_has_no_differences`.
- [ ] **Step 2: Run** `cargo test --locked --lib topology_manager::live_check -- --test-threads=2` — Expected: FAIL.
- [ ] **Step 3: Implement.** Read the production artifact shapes (read-only) to match field names; fixtures use invented values.
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): model LibreQoS planner rules and read back its state`.

### Task 2: Native acknowledgement check

**Files:**
- Modify: `src/apply_evidence.rs` (a public check against an expected pair, if one is not already public), `src/topology_manager/live_check.rs`
- Test: `src/apply_evidence.rs` tests

**Interfaces:**
- Produces: `pub async fn acknowledged(socket: &Path, expected: &Expected) -> anyhow::Result<()>` that reads `apply_evidence::read_applied` and checks the reported desired world and circuit capacity against the pair, with the same rule the receipt verifier uses. Errors keep the existing texts ("native acknowledgement unavailable").

- [ ] **Step 1: Write the failing tests:** `an_acknowledgement_for_the_pair_passes`, `an_acknowledgement_for_another_world_fails`, `an_unavailable_acknowledgement_fails` (use the existing IPC test helpers).
- [ ] **Step 2: Run** `cargo test --locked --lib apply_evidence -- --test-threads=2` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): check the native acknowledgement of a pair`.

### Task 3: The live test orchestrator

**Files:**
- Create: `src/runner/live_test.rs` (with `#[path]` tests), register in `src/runner.rs`
- Test: `src/runner/live_test_tests.rs`

**Interfaces:**
- Consumes: Task 1 `expected`, `admission`, `read_observed`, `compare`; Task 2 `acknowledged`; `Coordinator::prepare`; the controlled compiler; `parity::compare_legacy`; `files::write_if_changed`; `files::request_v3_import`.
- Produces:
  - `pub struct LiveTestReport` (serializable): step results, admission problems, parity issues, baseline, manager, and restore diffs, warnings, timings, and the backup directory.
  - `Runner::live_test(&mut self, import_timeout: Duration) -> anyhow::Result<LiveTestReport>` with the spec steps 1 to 8. A port trait `LiveIo` (state reads, import wait, acknowledgement) with a production implementation and a test fake.
  - `pub fn restore_pair(backup: &Path, lqos_dir: &Path) -> anyhow::Result<()>` for `live-test-restore`, which writes the backup pair (JSON first, then CSV) and checks byte equality.
- Order inside `live_test`: lock, preconditions, prepare, compile, admission, parity gate (stop before any write on a failure), backup, baseline read-back, manager write plus import request, wait for import and `ready` with a new generation, read-back and acknowledgement, restore plus import request, wait, restore read-back, byte check, report. Every failure after the manager write runs the restore.

- [ ] **Step 1: Write the failing tests:**
  - `a_passing_run_restores_the_legacy_pair_byte_for_byte`.
  - `a_parity_difference_writes_nothing` (Review Focus 3).
  - `an_import_timeout_restores_and_reports` (Review Focus 2).
  - `a_read_back_difference_restores_and_reports`.
  - `an_unavailable_acknowledgement_restores` (Review Focus 5).
  - `restore_pair_brings_back_the_backup_after_an_interruption` (Review Focus 1).
  - `the_test_never_writes_the_owner_file_receipts_or_registry`.
  - `the_test_needs_the_instance_lock`.
- [ ] **Step 2: Run** `cargo test --locked --lib runner::live_test -- --test-threads=2` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(topology): add the live acceptance test`.

### Task 4: CLI commands

**Files:**
- Modify: `src/cli.rs`
- Test: `src/cli.rs` tests

**Interfaces:**
- `lqosync3 topology live-test --report <path> [--import-timeout <s>]` (default 120) runs `Runner::live_test` and writes the report JSON plus a text summary to stdout; exit code 0 only when every step passed.
- `lqosync3 topology live-test-restore --backup <dir>` runs `restore_pair`.

- [ ] **Step 1: Write the failing tests:** `live_test_flags_parse_with_the_default_timeout`, `live_test_restore_needs_a_backup`, `unknown_live_test_flags_are_refused`.
- [ ] **Step 2: Run** `cargo test --locked --lib cli -- --test-threads=2` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(cli): add topology live-test commands`.

### Task 5: Runbook and records

**Files:**
- Create: `docs/operations/TOPOLOGY_LIVE_TEST.md`, `brain/10-sessions/2026-11-19-topology-live-acceptance.md`
- Modify: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`, `brain/30-knowledge/topology-manager-status.md`

- [ ] **Step 1: Write** the runbook with the spec window steps 1 to 7: the exact commands (pre-checks, tar backup of `/var/lib/lqosync3/` and the pair, `systemctl stop`, the enrollment commands, `live-test`, `live-test-restore`, `systemctl start`, the post-check), the "go" gate, the stop conditions, and the fallback rollback note (clear the fallback before a rollback to an older build).
- [ ] **Step 2: Run** the gate and `git diff --check` — Expected: all pass.
- [ ] **Step 3: Update** the records and commit `docs(topology): record live acceptance and add its runbook`.
