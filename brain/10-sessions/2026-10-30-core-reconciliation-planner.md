---
type: session
created: 2026-10-30
tags: [lqosync-v3, balancing, reconciliation, preview]
status: implemented-preview
related: ["[[2026-10-30-wan-health-preview]]"]
---

# Health-aware core reconciliation planner

Branch: `codex/core-reconcile`. Worktree: `/tmp/lqosync-core-reconcile`. Base: `e96de2a`.
Scope: Task 2, pure decisions only. No live caller, scheduler integration, or RouterOS writes.
Added Keep, Add, Move, Remove, and Blocked decisions with revision-bound desired assignments and cleanup receipts.
The planner uses fresh complete source/core inventories and caller-supplied session observations from the same reads.
It reuses dynamic scoring and the session continuity predicate. Existing assignment APIs retain their original behavior.
Retained/pinned clients count before pending arrivals and reconnects. Confirmed failure can evacuate active clients.
Manual arrivals use their default. Pin override requires an explicit option. Recovery does not force failback.

Health evidence now binds WAN and canonical settings revision. Completed-sample freshness limits downstream use independently of read timestamps.
A new regression reproduced the stale-read eligibility issue before correction.
The route analyzer can accept a structurally valid inactive default only for a freshly confirmed unhealthy source.
Healthy destinations still require valid active routes and a unique current enabled/running interface.
The old strict analyzer remains the default for existing consumers.

Cleanup needs a current selected source, two distinct complete observations, and elapsed grace time.
Whole-empty scans cannot delete or accumulate cleanup authority. Any input error requires the caller to discard absence receipts.
Receipts bind settings, exact owned member, router, and source selection.
Protected overlaps, IP reuse, cross-core moves, and IP-change transitions remain blocked.
No proposed assignment is observed membership or applied shaping.

Review found repeated source-index construction and missing health/settings binding. Both are fixed.
Later RED tests reproduced first-metadata rebalance, manual cleanup without session metadata, and a down destination interface.
Follow-up reviews found no material issue after correction. Source indices are now built once per plan.
Updated: 2026-10-30T20:07:01+08:00 (Asia/Shanghai).
Fresh verification: 256 tests passed, zero failures, 12 manual tests ignored.
Thirteen reconciliation tests passed. Format, strict Clippy, diff, documentation links, and Bash syntax checks passed.
No live router reads/writes, credential edits, production settings changes, or restarts occurred.
Next: guarded writer, durable recovery, then console controls and isolated RouterOS acceptance tests.
