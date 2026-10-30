---
type: session
created: 2026-10-30
tags: [lqosync-v3, core, migration, rollback]
status: complete
related: ["[[2026-10-30-core-membership-refresh]]"]
---

# Guarded core migration engine

Updated: 2026-10-30T17:11:56+08:00 (Asia/Shanghai).
Last agent: Codex. Branch: `codex/core-migration`. Worktree: `/tmp/lqosync-core-migration`.
The operator authorized executor development and isolated tests after the migration flow explanation.
Production RouterOS writes require separate authorization.

Added the isolated `core_migration` library, synthetic fault tests, an engine guide, and an implementation plan.
The engine compiles residual intervals from explicit protected row selections and selected hosts.
It preserves observed WAN membership and the full IPv4 union of every bound list.
It uses exact-plan approval correlation, fresh context and complete row checks, and a durable private journal.
Forward execution adds host/residual rows before it narrows original broad rows.
Rollback restores original addresses before it removes exact returned addition IDs.
Unknown outcomes block automatic retries and rollback. Known partial execution can reopen safely.
An exclusive per-journal claim prevents concurrent use of the same journal.
Production transports must separately exclude other journals and external writers.

Twenty-one focused tests pass. They cover lost replies, failed storage, stale inputs, expiry, foreign edits, ID scope, and journal recovery.
Fresh review found two issues: expiry after awaited reads/storage, and ownership marker identity compatibility.
Both issues failed regression tests before correction. Follow-up review found no remaining actionable issue.
Final verification passed: 217 tests, zero failures, 12 ignored. Format, strict Clippy, and diff checks passed.
The two-WAN test rejects ambiguous membership and forced WAN changes, then verifies exact rollback.
The graph remains a baseline. Connected ownership, address parsing, and durable storage behavior were confirmed in current code.

This stage does not add a production transport, console write route, candidate bootstrap integration, continuous writer, or actual publication.
No live settings, router entries, credentials, services, or LibreQoS output changed.
Next: implement and test the production adapter, authenticated authorization, writer fence, observed-WAN bootstrap, and actual validation integration.
Do not execute the private audit JSON as commands. Regenerate fresh evidence before any later migration.

Integration: fast-forward the verified development branch into `lqosyncv3`, then push to GitHub. No live restart is part of this stage.
