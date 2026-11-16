---
type: session
created: 2026-11-12
tags: [telemetry, performance, implementation]
status: complete-local
related: ["[[2026-11-12-production-performance-observation]]", "[[HANDOFF]]"]
---

# RAM-backed worker telemetry cache

Completed locally: 2026-11-12T15:19:25+08:00 (Asia/Shanghai). Agent: Codex `/root`.
The owner requested the next task after the production performance observation.

## Implementation

The worker monitor now publishes through `src/worker_util/cache.rs`.
It uses owner-controlled tmpfs directories under `/dev/shm/lqosync3-<effective-uid>/<canonical-state-path-sha256>/`.
The public state-directory cache path becomes a symlink only after a complete RAM publication.
Its JSON schema, source requests, two-second refresh, size limit, and three chart histories remain unchanged.

Old readers still use the public path. Atomic target replacements update the TUI stamp.
The previous disk writer replaces the symlink with a regular file during rollback.
The change leaves durable operation receipts, the Telegram outbox, and managed output paths intact.

Directory checks enforce ownership, public readability, no group/other write access, and tmpfs backing.
The publisher rejects directory symlinks and preserves existing private directory permissions.
Initialization or publication failure selects disk fallback and logs the reason.
Disk fallback remains in effect until a new publisher starts.
The shared atomic worker publisher now cleans temporary files after a failed write or rename.

No new dependency, configuration option, or service-unit change was necessary.
Tmpfs can use swap under host policy. Runtime cache data disappears at reboot.
The reader then shows unavailable data until publication resumes.
Restart keeps the previous cold-start history behavior.
RAM files for prior state paths can remain until reboot.

## Review and verification

The initial RAM placement regression failed against the disk-only implementation.
Two review regressions caught private-directory permission changes and a temporary-file leak.
Both passed after their fixes.
Tests cover legacy readers, complete histories, state isolation, missing runtime data, fallback, and regular-file rollback.
The TUI loader test covers cache reuse, target replacement, missing data, and rollback.

Final verification: 700 tests passed, zero failed, nine manual tests ignored.
Clippy, formatting, debug build, and all seven terminal routes passed.
The terminal smoke used live data as an unprivileged reader and saved only route labels and results.
No visual layout or screen body changed.

The manual isolated debug I/O test passed separately.
It published the same 283,359-byte synthetic cache 50 times through each path.
Steady RAM publication recorded zero block write bytes. The disk path recorded 14,336,000 bytes.
Elapsed times were approximately 0.977 and 0.987 seconds, respectively.

These timings are not a production CPU speedup claim.
The production-wide write reduction remains unmeasured until deployment.

Evidence: `/home/pi/.local/state/lqosync3/ram-cache-task/` and the `ram-cache-*` logs under the performance-observation directory.
Private logs contain no subscriber inventories, message bodies, or credentials.

## Handoff

The changes remain local on `lqosync3` in `/home/pi/LQoSync3.0`.
No commit, push, production migration, restart, or deployment occurred.
The installed service remains on `3.0.179+g12e3d57`.

Next release steps: review the final diff, commit the selected feature files, push, build a clean release, and deploy when requested.
Then check the symlink, unprivileged reader access, telemetry freshness, and total service write rate.
The owner still defers the upstream beta allocation issue.
Preserve unrelated profile/WAN notes and prior local diagnosis documentation.
