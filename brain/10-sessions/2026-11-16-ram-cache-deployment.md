---
type: session
created: 2026-11-16
tags: [telemetry, performance, deployment]
status: complete
related: ["[[2026-11-12-worker-ram-cache]]", "[[HANDOFF]]"]
---

# RAM-cache deployment

Verified: 2026-11-16T09:09:44+08:00 (Asia/Shanghai). Agent: Codex `/root`.
Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`.
The owner authorized deployment, restart, and production verification.

## Release and deployment

Commit `e8da2303505ef5891d862cb2439ec2b156759564` is pushed to `lqosync3`.
Release `3.0.182+ge8da230` came from a clean detached source checkout.
The full suite passed 700 tests. CI and the Rust 1.89 job passed.
The running binary matches the verified artifact hash:
`29bf39359771341b9b6c8355cc92aa1db69ac9ed3ec917d8548bae9416d5fde3`.

The first deployment attempt stopped before installation because native metadata changed.
The earlier empty slot map had recovered before the second attempt.
The successful preflight reports 28 slot names, 28 leaf operations, and native peer PID 206616.
The corrected script requires fresh subscriber telemetry, advancing circuit samples, and current policy proof before installation.
It uses no empty-map exception and no fixed subscriber count.

The operator completed the deployment at 2026-11-16T09:03:10+08:00.
The script restarted only `lqosync3.service`. Its new PID is 207228.
Configuration remains unchanged. Apply mode and the `v3_pair` target remain unchanged.
Backup: `/var/backups/lqosync3/ram-e8da230-m3u7afiz`.
The rollback path preserves durable Telegram delivery receipts.

## Live verification

All six telemetry sources passed, including native subscriber mapping and three advancing circuit samples.
The collection counts match the raw cycle snapshot.
All seven installed TUI routes passed. The terminal exited normally and restored its screen.
The unprivileged reader sees fresh worker data through the existing public path.
The worker cache now resides on tmpfs with file mode 0644 and directory mode 0755.
Its public symlink retains the same inode across publications.
A second independent verification passed after the observation window.

The native policy proof passes. Telegram delivery health is HEALTHY.
Overall health remains DEGRADED only because native capacity is 64 and generated capacity is 1000.
The owner still defers that upstream beta issue.
The bridge journal was not readable from this agent session. No journal-wide error claim applies.

## Five-minute observation

The observer collected 151 samples across 300 seconds and two completed sync cycles.
Cycle errors, stale telemetry samples, cache read errors, expired health components, and automatic restarts were zero.
Average CPU use was 0.171 percent of one core. Sync cycles took 136–137 ms.
Cgroup memory ranged from 4.57 to 6.90 MiB. Memory event counters showed no OOM event.

Service disk writes averaged 3.92 KiB/s, versus 197.94 KiB/s in the earlier baseline.
The observed difference is 98.02 percent across these two windows.
These windows are sequential production observations, not a controlled workload benchmark.
The new process has shorter chart histories after restart.
The baseline had stale subscriber telemetry. The final observation has fresh subscriber telemetry.
Do not claim a sustained reduction, a CPU speedup, or a long-term memory reduction from this sample.

Evidence: `/home/pi/.local/state/lqosync3/ram-e8da230/`.
Key results: `DEPLOYED.json`, `independent-verification.json`, `installed-tui-smoke.json`, and `performance-comparison.json`.
The `perf-before/` and `perf-after/` directories contain metadata-only observations.
No private subscriber data enters Git.

## Remaining work

No required deployment work remains.
A longer observation after the one-hour histories fill can measure the sustained write rate.
Preserve the unrelated profile/WAN Knowledge note and its Home link.
Deployment documentation remains local unless a later commit includes it.
