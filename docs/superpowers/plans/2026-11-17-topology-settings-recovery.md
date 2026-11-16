# Topology Settings and Recovery Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans. Execute the tasks in order.

**Goal:** Store explicit source settings with the topology and support verified backups and forward policy recovery.

**Architecture:** Extend the revisioned registry to schema 3. Use its existing lock, validation, and atomic writer. Keep source settings separate from current observations.

**Tech Stack:** Rust, serde, serde_json, existing private file helpers.

**Spec:** `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Global Constraints

- Preserve subscriber IDs, verified references, intent, and the shared ID counter.
- Do not store current endpoints, credentials, session locators, or raw rates.
- Do not infer source authority or migrate legacy speed priorities automatically.
- Preserve the live legacy path. Do not activate or deploy manager mode.
- Execute inline. Preserve unrelated local work. Do not commit or push this slice.

## Review Focus

- Old files must load without an implicit disk rewrite or invented authority.
- Invalid scope/profile/service settings must fail before durable replacement.
- Backups must preserve exact old bytes and reject links, corruption, or conflicting content.
- Interrupted writes must require a disk recheck. Backup failure must preserve the primary file.
- Policy recovery must preserve current identities and counters and reject incompatible references.

## Task 1: Durable source settings

**Files:** add `settings.rs` and `settings_tests.rs`. Extend model, identity, source_policy, source_rate, and module exports.

**Interfaces:** `SourceSettings { selections: Vec<SourceSelection> }`, registry `source_settings: Option<SourceSettings>`, and `authorize_configured`.

- [x] Write failing checks for strict round trips, catalog scope coverage, selectors, profile bindings, and absent/unconfigured authority.
- [x] Add strict serialization and static validation. Canonicalize unordered settings before commit.
- [x] Require persisted settings in `authorize_configured`. Keep the existing explicit API for isolated callers.
- [x] Run focused tests.

## Task 2: Schema migration and policy recovery

**Files:** extend `store.rs`. Add `store_recovery_tests.rs`.

**Interfaces:** `BackupId { schema_version, revision }`, `backup_current()`, and `restore_policy(expected_revision, backup)`.

- [x] Write failing tests for schema-1/2 preservation, backup verification, interrupted writes, and forward recovery.
- [x] Decode schema 1 and 2 strictly into schema 3 without disk changes. Keep settings unconfigured.
- [x] Before each changed commit, save the current bytes in a private backup. Compare the backup bytes with the original.
- [x] Reuse matching backups. Reject conflicting backup content instead of replacing it.
- [x] Restore catalog/source settings as a new revision. Preserve current subscriber records and the ID counter.
- [x] Reject corrupt current registries. Never select an older identity snapshot automatically.
- [x] Run focused tests and update old schema assertions.

## Task 3: Check and capture

- [x] Review the changed code against this plan and the design.
- [x] Run the full suite, format, Clippy, and whitespace checks.
- [x] Update design, session, Home, and handoff with results and remaining activation gates.

## Execution Notes

The owner approved this continuation. The developer instruction prohibits unsolicited subagents.
Run the work inline and do a separate author review. No independent reviewer participates.
The recovery API changes policy only. Full corrupt-store identity reconstruction requires independently verified complete evidence.
Backups contain private registry intent and verified references. They contain no current observations.
Old-schema backups support exact rollback before new settings exist. Configured schema-3 policy has no lossless old-schema conversion.

## Results

The migration/backup red run failed all three new checks. The first settings red run had three failures and one pass.
Configured authorization failed before its adapter used the saved settings.
The history-limit regression found 19 backups instead of 16 before the fix.
The restart regression showed a hidden cleanup failure before open repeated the history check.

This slice adds 20 checks. The full suite passes with 993 passed, zero failed, and 9 ignored.
Format and Clippy with denied warnings pass. The final source includes 276 topology checks.
A 10,000-profile check validates and canonicalizes reversed inputs with equivalent output. It does not establish production throughput.
The public restart integration now uses saved settings for Local and SourceRead authority.
Fresh return data preserves the saved pin and 50/10 Mbps subscriber maximum.

Author review covered strict decoding, selected scope coverage, private backup publication, interrupted writes, history limits, and forward recovery.
No independent reviewer participated. No code finding remains open in this slice.
A recovery fixture initially duplicated a source instance under one parent. The corrected fixture uses a manual node.
The first configured integration fixture also needed a delimiter correction before its behavioral red run.

The source settings and registry remain isolated from the live runner.
Legacy priority conversion, parity proof, publication/native admission, DPDK evidence, enrollment, TUI, and schedules remain separate work.
No dependency, legacy source parser, reader, runner, cycle, or live configuration changed.
No commit, push, deployment, router write, or service action occurred.
Logs: `.superpowers/sdd/2026-11-17-topology-settings-recovery/`.
