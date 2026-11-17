---
type: session
created: 2026-11-17
tags: [topology, enrollment, recovery, reliability]
status: complete
related: ["[[2026-11-17-topology-migration-staging]]", "[[HANDOFF]]"]
---

# Runner enrollment and automatic recovery

The user requested a commit and push, then the next topology task.
Committed the preceding migration slice as `c9890b2` and pushed it to `origin/lqosync3`.
Only the topology entries from the mixed Home and HANDOFF files entered that commit.
Unrelated operational edits remain in the worktree. This new controller slice remains local and uncommitted.

## Owner lifecycle

`Runner::lock()` now checks for a pending migration under the existing instance lock.
It recovers before receipt initialization, startup workers, normal cycles, or manager publication.
Normal startup without a journal does not open or create a topology registry.
The ordinary path adds one presence check. It does not decode registry or source policy on each cycle.

Recovery clears cached registry state and transport sessions before it opens the store with the same held lock.
Only success replaces the cached store. A failed write or cleanup cannot leave an older registry document available for later use.
The Runner keeps publication generations and an existing receipt tracker.

A first-lock failure releases the temporary lock and leaves receipt initialization incomplete.
An existing owner keeps its lock and receipt tracker during failure. The cached registry remains absent.
A later operation can retry after explicit conflict repair.
Corrupt journals and unknown source files block startup or the current cycle before normal work.

Recovery can finish after a mode change because it installs policy only.
It does not claim topology writer ownership or activate the service loop.
Legacy cycles keep their previous behavior after successful policy recovery until separate manager activation.

## Explicit enrollment

The existing `Runner::prepare_topology(max_age)` now has a public interface for preview and enrollment.
Its v3 pair apply target, receipt recovery, current collection, and identity authorization gates remain in force.
It prepares a fresh candidate without a registry commit, shaping write, reload, or owner claim.

Added `Runner::enroll_topology(pair, manifest, protected_roots, attachments)`.
It checks the target and receipt owner, then reads the current live CSV and network files.
The reader reuses the compiler's regular-file helper and enforces a combined raw budget of 16 MiB.
Missing files, directories, final-component symlinks, malformed content, and oversized inputs fail.

Configured adoption checks the current snapshot, manifest, source filters, and legacy rate policies.
A changed endpoint or network field blocks installation. Expired evidence also fails.
The Runner stages the sealed proposal and completes it through the same recovery boundary as startup.
It returns the installed registry revision. No shaping output or reload occurs.

The existing absent-source and absent-owner rules still protect first adoption.
A repeated completed enrollment cannot duplicate a revision. Interrupted enrollment instead resumes on the next lock operation.
Old prepared candidates cannot be reused after the registry revision changes.

## Code and proof

Added `topology_manager/enrollment.rs` and `runner/tests/enrollment.rs`.
Extended Runner owner initialization, the staging presence helper, and compiler reader visibility.
Reused the existing static snapshot and preflight test fixtures with small test-only visibility changes.
No dependency, registry schema, CLI, or TUI changed.

The first red test showed that startup did not recover the staged registry.
The API red run failed because `enroll_topology` did not exist.
A separate regression test exposed cached registry memory after a recovery error.
The final owner path clears that memory before it starts recovery.

The final full suite passed: 1,193 tests, zero failures, and 13 ignored tests across 13 result groups.
All 12 new enrollment tests and 445 active topology tests passed.
Format, Clippy with warnings denied, and whitespace checks passed.
The optional installed-library configured-source test also passed in temporary directories without live writes.

Tests cover startup, same-owner recovery, conflict repair, receipt owner reuse, and unchanged normal startup.
They also cover changed snapshots, filters, legacy policies, expiry, target/lock gates, unsafe files, and combined raw size limits.
Author review only. No production performance benchmark occurred.
The STE checker is heuristic. Its findings do not certify compliance.

Logs: `.superpowers/sdd/2026-11-17-topology-enrollment/`.
Proof: `full-final.log`, `clippy.log`, `format.log`, and `native-source.log`.

## Remaining work

An enrollment interface and manifest editor still need the existing preview and enrollment APIs.
The manager service loop remains gated. Native implicit commitments and planner/live proof remain prerequisites for activation.
Schedules, TUI, existing-source replacement, active-owner migration, and schema-2 circuit ID migration remain separate work.
External writers still need to respect the instance lock. No new per-batch operator approval was added.

No live source read, router write, live shaping output, native apply, service action, or deployment occurred.
The graph remains the earlier baseline. Current files take precedence.
Plan: `docs/superpowers/plans/2026-11-17-topology-enrollment.md`.
Related: [[2026-11-17-topology-migration-staging]], [[2026-11-17-topology-shared-adoption]], [[Home]].
