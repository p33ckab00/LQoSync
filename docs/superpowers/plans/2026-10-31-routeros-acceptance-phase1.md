# RouterOS Acceptance Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove RouterOS address-list add, same-ID move, and remove on `RB5k9Core` with one test host.

**Architecture:** One binary, `v3/src/bin/core-acceptance.rs`. It uses the existing `RouterTransport` for the three writes. Pure check functions in the same file compare full reads. Unit tests in the binary cover the checks.

**Tech Stack:** Rust, tokio, the existing `lqosync_v3` library. No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-31-routeros-acceptance-phase1-design.md`

## Global Constraints

- Router `RB5k9Core` only. Lists `WAN1-CUSTOMERS` (wan1) and `WAN2-CUSTOMERS` (wan2).
- Test address `192.0.2.254`. Subscriber identity in the marker: `acce5500-0000-4000-8000-000000000001` (owned markers require a canonical UUID).
- Marker: `core_plan::marker(owner, "acceptance-test", wan)`, with `owner = settings.core_lists.owner`.
- Without `--execute`, send no write. Stop at the first failed check. Never retry a request.
- No change to `ShapedDevices.csv`, `network.json`, settings, services, or other router rows.
- Data root: `LQOSYNC3_DATA`, with the same `/opt/libreqos` and `/etc/libreqos` refusal as `core-validate`.
- Report: `data/state/acceptance/phase1-<unix-seconds>.json`, written with `store::atomic_write`.

## Review Focus

- A row with address `192.0.2.254` or a comment with the test UUID exists before the test: stop before any write.
- Another writer changes the address-list between the guard read and the send: the transport preflight stops the step.
- `add` returns no `.id`: stop, and print the removal instruction by address and marker.
- RouterOS returns volatile fields in mangle or route rows: the final baseline check fails and names the table. The test row is already removed at that point.
- A failure after the add: print the test row `.id` and `/ip firewall address-list remove <id>`.

---

### Task 1: Pure checks with unit tests

**Files:**
- Create: `v3/src/bin/core-acceptance.rs` (checks and `#[cfg(test)] mod tests` only; `main` prints usage)

**Interfaces:**
- Consumes: `lqosync_v3::core_migration::Member`, `lqosync_v3::routeros::Row`.
- Produces:
  - `fn check_clear(rows: &[Member]) -> Result<()>`
  - `fn check_added(before: &[Member], after: &[Member], id: &str, expected: &Member) -> Result<()>`
  - `fn check_moved(before: &[Member], after: &[Member], id: &str, list: &str, comment: &str) -> Result<()>`
  - `fn check_same(before: &[Member], after: &[Member]) -> Result<()>`
  - `fn strip_counters(rows: &[Row]) -> Vec<Row>` (removes `bytes` and `packets`)
  - `const TEST_ADDRESS: &str = "192.0.2.254"`, `const TEST_SUBSCRIBER: &str = "acceptance-test"`

- [ ] **Step 1: Write the failing tests** with a helper `m(id, list, address, comment) -> Member` (flags false, timeout `None`):
  - `clear_rejects_test_address`: a row with address `192.0.2.254` → `check_clear` returns `Err`. Also `192.0.2.254/32` → `Err`.
  - `clear_rejects_test_marker`: comment `lqosync3:o:acceptance-test:wan1` → `Err`.
  - `clear_accepts_other_rows`: `10.15.0.0/24` rows → `Ok`.
  - `added_accepts_one_new_row`: before = [A]; after = [A, new `*20`] with expected fields → `Ok`.
  - `added_rejects_changed_other_row`: A changes comment in after → `Err`.
  - `added_rejects_wrong_fields`: new row in wrong list → `Err`.
  - `moved_accepts_list_and_comment_change`: same `.id`, address, flags; new list and comment → `Ok`.
  - `moved_rejects_new_id`: row has a different `.id` after the move → `Err`.
  - `moved_rejects_changed_address`: → `Err`.
  - `same_detects_any_change`: `check_same` on equal lists → `Ok`; on one changed field → `Err`.
  - `strip_counters_ignores_bytes_and_packets`: two rows that differ only in `bytes` and `packets` → equal after strip.

- [ ] **Step 2: Run** `cd v3 && cargo test --bin core-acceptance`. Expected: compile errors for the missing functions.

- [ ] **Step 3: Implement the checks.** Compare by `.id`. A check fails if any row except the target row differs, or if the row count is wrong.

- [ ] **Step 4: Run** `cd v3 && cargo test --bin core-acceptance`. Expected: 11 passed.

- [ ] **Step 5: Commit** `feat(acceptance): add phase 1 address-list checks`.

### Task 2: Router I/O, dry run, and execute

**Files:**
- Modify: `v3/src/bin/core-acceptance.rs` (`main`, `Fresh` verifier)
- Modify: `v3/OPERATIONS.md` (short "RouterOS acceptance, Phase 1" section: dry run, execute, manual removal)

**Interfaces:**
- Consumes: Task 1 functions. `store::load_settings`, `credentials::password`, `routeros::Client::{connect_with_password, read}`, `core_writer_transport::{RouterTransport, Verifier, members}`, `core_writer::{Transport, Operation, Evidence}`, `orchestrator::now`.
- Produces: the `core-acceptance [--execute]` command.

- [ ] **Step 1: Implement `struct Fresh { client: routeros::Client }`** with `impl Verifier`. `verify` reads `/ip/firewall/address-list/print` and returns `Evidence { context: "acceptance-phase1", captured_at: now, valid_until: now + 30, rows: {"RB5k9Core": members(..)} }`.

- [ ] **Step 2: Implement `main`** in the spec sequence:
  1. Parse args: none or `--execute`. Other args → usage error.
  2. Load settings. Find router `RB5k9Core`. Find the wan1 and wan2 bindings on it. Stop if they are absent.
  3. Read the address-list two times. `check_same` (spec question 6). `check_clear`.
  4. Read `/ip/firewall/mangle/print`, `/ip/route/print`, `/routing/table/print`. Keep `strip_counters` results.
  5. Print the plan: router, address, both lists, both markers, and row count. Without `--execute`, stop here with exit 0.
  6. Add: `transport.observe()` as guard, then `execute(Operation::Add { .. }, &guard)`. The member has an empty `id`, the wan1 list, the wan1 marker, flags false, and timeout `None`. Require `Some(id)`. Read, then `check_added`.
  7. Move: new guard, `Operation::Move` with `before` = the read row and `after` = the same row with the wan2 list and wan2 marker. Read, then `check_moved`.
  8. Remove: new guard, `Operation::Remove { before }`. Read, then `check_same` against the step 3 rows.
  9. Read the three baseline tables again. Compare with step 4. Name each table that differs.
  10. Write the report: each step, the returned `.id`, row snapshots of the test row, and each check result. Print `Phase 1 passed` or the first failure.
  On any failure after step 6, print the test row `.id` and `/ip firewall address-list remove <id>`.

- [ ] **Step 3: Run** `cd v3 && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --bin core-acceptance`. Expected: all pass.

- [ ] **Step 4: Dry run on production (read-only).** `LQOSYNC3_DATA=/opt/LQoSync3.0/v3/data target/debug/core-acceptance`. Expected: the plan prints; no write. Show the output to the operator.

- [ ] **Step 5: Get the operator's go-ahead, then execute.** `LQOSYNC3_DATA=/opt/LQoSync3.0/v3/data target/debug/core-acceptance --execute`. Expected: `Phase 1 passed`. Then run `core-validate` and confirm no `192.0.2.254` row.

- [ ] **Step 6: Commit** the binary and `OPERATIONS.md`: `feat(acceptance): run phase 1 address-list acceptance`. Do not commit the report. Record the result in `brain/HANDOFF.md`.
