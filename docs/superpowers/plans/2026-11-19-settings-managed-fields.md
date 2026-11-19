# Settings managed fields implementation plan (Phase 2c-1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show which system owns the `nodes.`, `bandwidth_defaults.`, and `backstop_caps.` Settings fields: a note after enrollment, read-only after activation.

**Architecture:** `status` adds `writer` from `topology_manager::runtime::topology_owns`. The app forwards the last topology `status` answer to the Settings view, which adds the note to the field help and treats managed fields as read-only when the writer is `"topology"`.

**Tech Stack:** Rust 2024, ratatui, serde_json. No new dependencies. MSRV 1.89.

**Spec:** `docs/superpowers/specs/2026-11-19-settings-managed-fields-design.md`

## Global Constraints

- Do not stop, restart, deploy, or reconfigure `lqosync3.service`. Do not touch `/etc/lqosync3`, `/var/lib/lqosync3`, `/opt/libreqos/src`, or routers.
- No new dependencies. Keep `cargo +1.89 build --locked` working. No change to `config.toml` handling or the legacy loop.
- Gate for every task: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`. Run cargo with `nice -n 19`, `CARGO_BUILD_JOBS=2`.
- Separate worktree, branch `settings-managed-fields` from `lqosync3`. Do not edit `docs/OPERATIONS.md`.
- Persisted text follows ASD-STE100.
- Managed prefixes: `nodes.`, `bandwidth_defaults.`, `backstop_caps.`.
- Exact notes (spec): "the legacy loop uses this now; after activation the Topology Manager owns it (Topology > Policies or Builder)" and "managed by the Topology Manager: edit it in Topology > Policies or Builder".

## Review Focus

1. No service or no status answer: Settings works as today, no note, no lock. Test in Task 2.
2. A corrupt or unreadable owner file: `writer` is `"unknown"`, and Settings shows the enrolled note without a lock. Test in Task 1 and Task 2.
3. `reset` and `edit` on a locked managed field change nothing and show the managed note. Test in Task 2.
4. A field outside the three sections (for example `runtime.mode`) is never noted or locked. Test in Task 2.
5. The writer changes from `"legacy"` to `"topology"` while the Settings view is open: the next status answer locks the fields. Test in Task 2.

---

### Task 1: `writer` in the status answer

**Files:**
- Modify: `src/runner.rs` (`api_status`)
- Test: `src/runner/tests/api.rs` (`mod operations`)

**Interfaces:**
- Produces: `status` adds `"writer"`: `"topology"` when `topology_owns(state_dir)` is `Ok(true)`, `"legacy"` when `Ok(false)`, `"unknown"` on `Err`.

- [ ] **Step 1: Write the failing tests:** `status_reports_the_legacy_writer_without_an_owner_file`, `status_reports_the_topology_writer_with_the_owner_file` (use `topology_manager::runtime::claim` in a prepared state directory), `status_reports_an_unknown_writer_for_a_bad_owner_file` (Review Focus 2).
- [ ] **Step 2: Run** `cargo test --locked --lib runner::tests::api` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(api): report the shaping writer in status`.

### Task 2: Notes and locks in Settings

**Files:**
- Modify: `src/tui/settings/view.rs`, `src/tui/app.rs`, `src/tui/topology/mod.rs` (a status getter and a status request)
- Test: `src/tui/settings/view.rs`, `src/tui/app.rs`

**Interfaces:**
- Consumes: Task 1 `writer`.
- Produces:
  - `SettingsView::set_topology_state(&mut self, enrolled: bool, writer: &str)`.
  - `TopologyView::status(&self) -> Option<&Value>` and `TopologyView::request_status(&mut self)` (queues one `status`).
- Behavior: a managed field gets the enrolled note in its help when `enrolled` and `writer != "topology"`. When `writer == "topology"`, a managed field is read-only (the same path as a static `read_only` meta) and its help and message show the managed note. Not enrolled or no state: no change.
- App: when the Settings tab opens, call `topology.request_status()`; on each tick pass `topology.status()` (`enrolled`, `writer`) to the Settings view.

- [ ] **Step 1: Write the failing tests:**
  - `managed_fields_have_no_note_before_enrollment_or_without_status` (Review Focus 1).
  - `an_enrolled_legacy_writer_adds_the_note_and_keeps_fields_editable`.
  - `an_unknown_writer_adds_the_note_without_a_lock` (Review Focus 2).
  - `a_topology_writer_locks_managed_fields_and_reset_and_edit_change_nothing` (Review Focus 3).
  - `fields_outside_the_managed_sections_are_not_affected` (Review Focus 4).
  - `a_new_status_answer_locks_an_open_view` (Review Focus 5).
  - `opening_settings_asks_for_status` (app test).
- [ ] **Step 2: Run** `cargo test --locked --lib tui::settings tui::app` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests and the full gate — Expected: PASS.
- [ ] **Step 5: Commit** `feat(tui): show Topology Manager ownership in Settings`.

### Task 3: Records and final verification

**Files:**
- Modify: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`, `brain/30-knowledge/topology-manager-status.md`
- Create: `brain/10-sessions/2026-11-19-settings-managed-fields.md`

- [ ] **Step 1: Run** `cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked && git diff --check` — Expected: all pass.
- [ ] **Step 2: Update** the records: the `writer` field, the managed sections, and the 2c-2 next step.
- [ ] **Step 3: Commit** `docs(topology): record Settings managed fields`.
