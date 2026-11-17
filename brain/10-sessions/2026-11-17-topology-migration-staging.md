---
type: session
created: 2026-11-17
tags: [topology, migration, recovery, integrity]
status: complete
related: ["[[2026-11-17-topology-shared-adoption]]", "[[HANDOFF]]"]
---

# Controlled migration staging and recovery

The user approved controlled installation of the shared adoption proposal.
The preceding slice is committed and pushed as `e0afb4b` on `lqosync3`.
This slice remains local. Preserve unrelated operational edits.

## Transaction boundary

Added `staging::stage`, `recover`, and `cancel` as isolated library APIs.
A proposal now keeps its evidence deadline and a digest of its proven registry.
A change to the public registry field invalidates the seal.

The store reuses ordinary canonicalization and preservation checks to calculate the target revision.
It confirms a private registry backup before the transaction saves its journal.
The journal binds its canonical owner directory, base policy, target policy, and exact static source.
Its SHA-256 digest detects content changes. The journal limit is 48 MiB. The source limit stays 16 MiB.

This first-adoption API requires no existing static source or runtime-owner file.
Unknown or unsafe files fail without replacement. A copied journal fails in another runtime directory.
A pending journal blocks ordinary commits, writer claims, and publication through both runtime paths.
The store can reopen for inspection and recovery.

Recovery creates a temporary `migration-pending` runtime-owner record before source installation.
Earlier strict runtime readers reject that writer. It never grants topology ownership.

The transaction reuses the private backup publisher for journal, marker, and source installation.
That publisher uses a synced temporary file and a hard link. It cannot replace an existing destination.
It preserves exact bytes, without the extra newline from the ordinary secret writer.

Recovery accepts only the recorded base or target registry and exact source bytes.
It installs source revision 1, commits one target revision, and checks both before cleanup.
A restart at any durable boundary can resume without a second revision increment.
Uncertain registry writes keep the store's existing reopen requirement.
A missing or changed source after commit requires explicit repair. It never receives a guessed replacement.

Cancellation is available only before source installation, while the registry still matches the base.
After installation, recovery moves forward. It does not roll back subscriber identities or the allocator.
Completion removes the temporary owner marker and journal with directory syncs.
It does not claim runtime ownership or freeze later source revisions.

Only explicit static endpoints enter the journal's source payload.
Dynamic subscribers still need fresh observations for later publication.
A saved policy transaction can recover after its original dynamic evidence expires because recovery writes no shaping output.

## Scope and verification

No dependency or registry schema changed. The store now checks its held lock before a backup.
The normal store path keeps its prior validation and uncertain-write behavior.
New tests cover durable boundaries, cancellation, expiry, proposal changes, file conflicts, unsafe journals, and owner isolation.
They also cover source loss after commit, exact bytes, symlinks, lock replacement, and the earlier strict owner parser.

The full suite passed: 1,181 tests, zero failures, and 13 ignored tests across 13 result groups.
All 445 active topology tests passed. This slice adds 14 active tests.
Format, Clippy with warnings denied, and whitespace checks passed.
The installed-library scratch test also passed without live apply.

The initial red run failed because the new API was absent.
The first implementation exposed the secret writer's added newline. The final path reuses the exact-byte, no-replacement publisher.
The tests now check exact source bytes across every durable boundary.

The STE checker is heuristic. Its findings do not certify compliance.

Logs: `.superpowers/sdd/2026-11-17-topology-migration-staging/`.
Author review only. No production performance benchmark occurred.

## Next boundary

Connect these APIs to a controlled enrollment/recovery controller before normal service work.
Normal runtime batches remain automated. Critical conflicts block the affected migration.
An existing-source replacement path and migration of an active owner need separate contracts.
Mixed-version registry edits and uncoordinated external file edits remain unsupported.
Native implicit commitments, planner/live proof, enrollment UI, schedules, TUI, and service activation remain separate gates.

No live source read, router write, live shaping output, native apply, service action, or deployment occurred.
The graph remains the earlier baseline. Current files take precedence.
Plan: `docs/superpowers/plans/2026-11-17-topology-migration-staging.md`.
Related: [[2026-11-17-topology-shared-adoption]], [[2026-11-17-topology-static-input]], [[Home]].
