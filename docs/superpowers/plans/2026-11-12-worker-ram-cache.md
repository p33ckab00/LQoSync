# RAM-backed worker telemetry cache implementation plan

**Goal:** Reduce telemetry disk writes while preserving cache readers, the two-second refresh, and all current chart history.

**Architecture:** Store worker telemetry in an owner-controlled tmpfs directory under `/dev/shm/lqosync3-<effective-uid>/<state-path-sha256>/`.
Keep `<state_dir>/worker-utilization.json` as an atomically replaced symlink to that file.
Existing readers follow it without a schema or path change. Existing writers replace it with a regular file during rollback.
Keep operation receipts, notification records, managed outputs, and the other caches in their current locations.

**Scope:** Linux, existing dependencies, no configuration keys, no service-unit change.
The runtime cache remains bounded by the existing 1 MiB limit.
History retention and source sampling remain unchanged. Tmpfs can use swap if the host permits it.

## Compatibility and failure rules

- Create directories with mode 0755. Require the effective UID as owner and no group/other write access.
- Preserve the permissions of existing private directories and use disk fallback instead.
- Reject symlink directories. Confirm tmpfs with `statfs` before publishing.
- Derive a separate directory from the canonical state path for each service instance.
- Publish the complete RAM file before replacing the public path with a symlink.
- Preserve an existing regular cache until the new file is ready.
- Preserve unknown symlink targets. Disk fallback replaces only the public directory entry.
- If initialization or publication fails, use the existing atomic disk publisher and log the reason.
- Retain disk fallback until service restart. Do not repeatedly attempt a broken RAM path every two seconds.
- Missing or stale runtime data retains the existing unavailable/stale behavior.
- Preserve durable files and their contents during migration and rollback.

## Implementation and checks

- [x] Add regression coverage for RAM placement through the unchanged public path, complete history, regular-file rollback, reboot-style missing data, and disk fallback.
- [x] Prove the first RAM-placement regression fails with the current disk publisher.
- [x] Add `src/worker_util/cache.rs` for directory validation, symlink publication, and fallback.
- [x] Reuse `worker_util::publish` for cache serialization and atomic file replacement.
- [x] Connect one cache publisher to the monitor and reset it when the state directory changes.
- [x] Test unsafe directories, state isolation, runtime-file failure, and unchanged durable records.
- [x] Run focused tests, the full test suite, Clippy, formatting, and TUI smoke.
- [x] Measure repeated publication in an isolated temporary state directory on tmpfs.
- [x] Update operations documentation and project memory with results and limits.

## Stop condition

Complete the local implementation and review. No production migration, restart, commit, push, or deployment is part of this task.
The owner already authorized implementation. Execute in this session and preserve unrelated work.

## Result

Completed locally on 2026-11-12. Review covered directory permissions, private-target preservation, temporary-file cleanup, reader stamps, and rollback.
The first RAM-placement regression failed against the disk-only publisher, then passed with the runtime publisher.
Two review regressions also failed before their permission and cleanup fixes.

Final checks: 700 tests passed, zero failed, nine manual tests ignored. Clippy, formatting, debug build, and seven-screen terminal smoke passed.
The manual cache I/O test passed separately. Fifty publications wrote zero block bytes on tmpfs and 14,336,000 bytes on disk.
The debug measurement used a 283,359-byte synthetic cache and preserved all three 1801-point histories.
No production change or deployment occurred.
