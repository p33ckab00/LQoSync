---
type: session
created: 2026-10-31
tags: [lqosync-v3, core-worker, verifier, offline]
status: complete-offline-stage
---

# Offline guarded worker and fresh verifier

Updated: 2026-10-31T06:14:13+08:00 (Asia/Shanghai).
Base: `b6efb03`. Branch: `codex/core-worker`.
Worktree: `/home/pi/lqosync-core-worker`. Target: `lqosyncv3` in `/opt/LQoSync3.0`.

The operator selected code and mock tests. No RouterOS lab is available.
The temporary worktree and logs were absent after interruption. Reconstructed the changes in this persistent worktree.
Only fresh results from the recovered work count as final verification.

Added `core_worker` with RouterSource, Prepared, GuardedVerifier, expiring Approval, and Worker.
The worker connects actual-read verification to the existing scoped transport and exclusive offline executor.
Exact approval binds the canonical root, settings, sessions, rates, health, and routing policy.
Known owned-host progress changes actual rows separately. The executor still checks the complete expected row set.
All selected policy modes receive session identity checks, including manual and planned-load sources.
Health deadlines retain completed-sample freshness. Settings confirmation follows awaited collection and parsing.
Pause, revocation, source changes, policy changes, stale inputs, and reboot evidence stop further sends.

The source reads at most four routers concurrently and never saves candidates or inventory.
CPU parsing runs in the blocking pool. Verification does not rerun reconciliation for every send.
The guard keeps active sessions without duplicate assignment maps.
No dependencies, settings schema, maintenance lease, or console enable gates changed.

## Verification

Focused console, worker, and session checks passed before the final small memory reduction.
Strict Clippy passed on the final code. Fresh review found no concrete defects.
Final full-suite result: 295 passed, zero failures, 12 ignored.
The existing 50,000-session workload passed. Debug times: collect 592.8 ms, assign 854.9 ms, retain 1.062 s, candidate save 1.331 s, candidate load 2.448 s.
This measures the synthetic session pipeline, not end-to-end worker throughput or a paired baseline comparison.
Documentation links, whitespace, and format checks passed.
The STE heuristic ran. Technical-name and list findings require prose review, not a certification claim.

## Boundary and next action

This is an offline library. No CLI, HTTP action, scheduler, or automatic-mode setting invokes it.
The exclusive fence excludes every running console. No lease upgrade was added.
A lost reply retains the recovery marker and blocks retry and rollback.
Expired or revoked grants require separate recovery review. No cold-resume grant interface exists.
Online console coordination, journal controls, recovery approval, and isolated RouterOS acceptance remain pending.
No live router reads or writes, production settings, shaping files, package changes, or service restart occurred.
Preserve the separate frontend planning edits in the main checkout.

See `v3/CORE_WORKER.md`, the execution ledger, and [[2026-10-30-core-console-preview]].
