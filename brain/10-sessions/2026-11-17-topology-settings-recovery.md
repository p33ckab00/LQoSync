---
type: session
created: 2026-11-17
tags: [topology, settings, migration, recovery, integrity, reliability, ste]
status: settings-recovery-complete
related: ["[[2026-11-17-topology-source-policy]]", "[[HANDOFF]]"]
---

# Durable topology settings and forward recovery

The owner directs continuation with durable source settings and migration/recovery.
Agent: Codex `/root`. Timestamp: 2026-11-17T04:40:31+08:00, Asia/Shanghai.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `489acc7` plus prior local slices.
All new work remains local and uncommitted. Unrelated operational work remains intact.

## Changes

Added `settings.rs`, `settings_tests.rs`, and `store_recovery_tests.rs` under `src/topology_manager/`.
Extended model/store to schema 3. Added strict serialization to source selections, reference authority, profile bindings, and exact rate formats.
The format adapter reuses the existing config enums without changes to the legacy configuration module.
Added `authorize_configured` and changed the public restart proof to use saved settings.
Updated design, discussion, plan, Home, and handoff.

Source settings and the catalog commit in one checked revision.
Static validation requires exact scopes, distinct selectors, valid profile bindings, and no service overlap.
Hash indexes check service and profile membership. Canonical order prevents revisions for order-only edits.
Unconfigured authority can remain in a saved draft. Configured authorization blocks it before collection acceptance.
Missing settings do not enable defaults or legacy speed priorities.

Schema-1/2 adapters preserve IDs, references, counters, catalog policy, and subscriber intent.
Open leaves the old primary bytes unchanged. A checked commit writes schema 3.
Before replacement, the store requires an exact private backup of the confirmed disk bytes.
A synced temporary file becomes a backup through a non-replacing hard link.
The store reads the backup again and compares exact bytes. Corruption, links, or conflicts block replacement.

History keeps 16 recent schema-3 backups and the backups before migration.
Normal forward upgrades create at most one backup for each old schema.
Optional cleanup errors set `history_cleanup_pending` without a false policy-write failure.
Open repeats the check, so unresolved cleanup errors remain visible after restart.

`restore_policy` restores catalog/source settings as a forward revision.
It keeps current subscriber records, verified identities, pins, and the shared ID counter.
Current reference checks reject incompatible old policy. Ordinary edits cannot remove configured settings.
A corrupt primary still blocks open. An older backup cannot prove the latest identity history or counter.
Full corrupt-store reconstruction requires independently verified complete identity evidence.
Current endpoints, sessions, credentials, and raw rates do not enter this storage path.

## Checks and review

The initial migration/backup run failed three checks. The initial settings run failed three checks and passed one.
Configured authorization, history retention, and cleanup status after restart each had a behavioral red run before their fix.
A recovery fixture first duplicated a source instance under one parent. It now uses a manual node.
The first configured integration fixture needed a delimiter correction before its behavioral red run.

This slice adds 20 checks. Full suite: 993 passed, zero failed, 9 ignored.
Format, Clippy with denied warnings, and whitespace checks pass.
The source now contains 276 topology checks. A 10,000-profile case produces equal canonical output after reversed input.
That synthetic check does not establish production throughput.
The public restart test covers saved Local and SourceRead settings, fresh return data, the saved pin, and a 50/10 Mbps maximum.
Existing checks cover failures before/after primary rename and require a disk recheck before further use.

Author review covered strict decoding, reference coverage, backups, interrupted writes, history, and forward recovery.
No independent reviewer participated. No code finding remains open in this slice.
The STE checker is heuristic. Manual review does not establish certified compliance.
No dependency, legacy config/parser/reader, runner, cycle, deployment, router, or service changed.
No commit or push occurred. The project graph remains the earlier baseline.

Plan: `docs/superpowers/plans/2026-11-17-topology-settings-recovery.md`.
Logs: `.superpowers/sdd/2026-11-17-topology-settings-recovery/`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Next action

Prepare the publication boundary and explicit legacy-policy parity migration.
Preserve and recheck source deadlines. Prove native rate bounds, export precision, admission, and correlated DPDK application.
Then connect the TUI and scheduler after their remaining time-boundary decisions.
Live enrollment and full corrupt-store reconstruction also remain activation gates.
Do not activate or deploy manager mode before its gates pass.
