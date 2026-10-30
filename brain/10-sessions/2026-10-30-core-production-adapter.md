---
type: session
created: 2026-10-30
tags: [lqosync-v3, core, migration, rollback]
status: complete
related: ["[[2026-10-30-core-migration-engine]]"]
---

# Core production adapter

The operator authorized implementation and isolated tests. Live migration remains a separate authorization.
Branch: `codex/core-adapter`. Worktree: `/tmp/lqosync-core-adapter`. Base: `b5717dd`.

Added a local maintenance CLI with private operator-owned inputs and exact revision approval.
It prepares a review envelope with explicit operations. It requires a plan-bound external writer acknowledgement.
The console holds a shared kernel lease. Maintenance requires an exclusive lease and a durable recovery marker.

The CLI also detects legacy consoles for the same data directory. External RouterOS writers require an operator maintenance window.
The CLI does not create a router-side transaction or stop external writers.

The adapter uses existing credentials and TLS. Its mutation method accepts only scoped address-list additions, address changes, and removals.
A local protocol patch preserves completion attributes and accepts either tag order.

Additions require the actual returned row ID. Missing replies and missing IDs keep pending state.
Known-unsent preflight errors clear only their unsent intent. Unknown outcomes block automatic retries and rollback.

Bootstrap uses observed WAN membership and real session handles and uptimes. Temporary pins exist only in memory.
It rejects conflicting manual pins, source changes, session changes, stale evidence, and expired approval.
A durable two-state bootstrap receipt permits recovery before or after the candidate CAS save.
Fresh actual membership and ten validation gates run in a separate validation directory before maintenance clears.
The CLI does not publish the application current generation or apply LibreQoS output.

Fresh review found a bootstrap receipt interruption gap and false pending state for known-unsent preflight failures.
Regression tests reproduced both. Corrections and follow-up review found no remaining material defect.
More tests cover inherited descriptors, legacy console exit races, private file ownership, expiry, and both completion tag orders.

Updated: 2026-10-30T18:15:32+08:00 (Asia/Shanghai).
Fresh verification: 232 tests passed, zero failures, 12 manual tests ignored.
All 60 parallel lease stress runs passed. Format, strict Clippy, diff, documentation links, and Bash syntax checks passed.
The final read-only review found no material defect.
No live router writes, credential changes, service restarts, or LibreQoS apply occurred.
Next: regenerate private production evidence and review exact protected-row operations before any live migration authorization.
See `v3/CORE_MAINTENANCE.md` for the procedure. The previous audit JSON is not executable by this CLI.
