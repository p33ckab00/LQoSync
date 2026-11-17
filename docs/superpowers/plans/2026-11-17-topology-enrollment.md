# Controlled topology enrollment implementation plan

> Execute this plan inline with `superpowers:executing-plans`. Use tests before code changes.

**Goal:** Connect migration enrollment and automatic recovery to the existing Runner owner.

**Architecture:** Recover a pending transaction under the instance lock before receipt recovery or normal service work.
Expose an explicit Runner enrollment method. Read the current live pair and reuse configured adoption, staging, and recovery.
The service loop keeps its activation gates.

**Tech stack:** Existing Rust, Tokio, serde, and filesystem helpers. No new dependencies.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`, controlled first-adoption transaction.

## Constraints

- Keep the existing instance lock and receipt tracker as the only owners.

- Require the v3 pair apply target for new enrollment. Recovery writes policy only and can finish after a mode change.

- Recover only an existing journal. Do not create a topology registry during normal legacy startup.

- Block service work on critical conflicts. Preserve unknown files and existing outputs.

- Do not select topology runtime ownership or activate the service loop.

- Preserve unrelated operational edits. Keep the new slice local after the preceding commit and push.

## Task 1: Prove Runner recovery and enrollment

- [x] Add tests for startup and same-owner recovery, corrupt journals, retries, mode changes, and lock reuse.

- [x] Add tests for explicit enrollment, changed live evidence, configuration conflicts, expiry, and source conflicts.

- [x] Run the tests and check that the missing API or behavior fails.

## Task 2: Connect the owner boundary

- [x] Add `staging::pending` as a bounded presence check for the controller.

- [x] Reuse the held lock to reopen and recover only when a journal exists. Replace stale store memory after success.

- [x] Clear transport sessions after migration. Keep publication generations and the receipt owner intact.

- [x] Add a bounded live-pair reader in `enrollment.rs`. Reuse the compiler regular-file reader.

- [x] Expose the existing fresh preview method. Add `Runner::enroll_topology` with configured adoption, stage, and shared recovery.

## Task 3: Verify and record

- [x] Run focused tests, the full suite, format, Clippy, and whitespace checks.

- [x] Review the owner lifecycle and failure order. Keep live writes outside this task.

- [x] Update design, discussion, session, Home, and HANDOFF with results and remaining gates.
## Results

The full suite passed: 1,193 tests, zero failures, and 13 ignored tests across 13 result groups.
All 12 new enrollment tests and 445 active topology tests passed.
Format, Clippy with warnings denied, whitespace, and the installed-library configured-source scratch test passed.
Author review only. No production benchmark or live apply occurred.

Logs: `.superpowers/sdd/2026-11-17-topology-enrollment/`.
The new slice remains local and uncommitted after checkpoint `c9890b2`.
