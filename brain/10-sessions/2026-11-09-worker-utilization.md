---
type: session
created: 2026-11-09
tags: [session, libreqos-v3, telemetry, tui]
status: deployment-pending
related: ["[[2026-11-09-node-minimums]]", "[[2026-11-09-libreqos-beta19-integration-review]]", "[[HANDOFF]]"]
---

# Actual worker utilization

## Scope

The user authorized priority task 2, performance and integrity review, checks, commit, push, and a service restart.
The live service already runs the task 1 build: `3.0.140+gdbde3d2`, with `v3_pair` and no cycle error.

## Interface

The beta19 public HTTP CPU endpoint omits the new utilization rows.
The read-only analytics socket supports these rows in the snapshot reply.
The protocol sends a CBOR request, an SCM_RIGHTS descriptor, and a CBOR reply in that file.
Source: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/ipc.rs

The client requests `kind=snapshot` and `limit=1`. It ignores host and flow records.
The request has a two-second deadline. Reply metadata has a 1024-byte bound. Snapshot bytes have a 2 MiB bound.
The client owns every received descriptor and closes them on success or failure.
The implementation uses `nix`, already in the lockfile, and adds `ciborium` for CBOR.

## Changes

One service task polls every two seconds. It publishes an atomic worker-only cache without fsync.
The public cache has mode `0644` and the name `worker-utilization.json` in the runtime state directory.
No socket request occurs during TUI draw or router sync logic.
The first producer timestamp establishes a baseline. Only a later advancing timestamp enables live values.
Errors retain last-good rows with a stale label. The TUI also expires cached values after six seconds.
Missing fields remain unavailable or unknown. Percentages above 100 and duplicate worker IDs fail validation.

The Workers tab uses key 6. It shows busy, idle, mean backlog, freshness, and sample age.
It supports scrolling and the current themes. Tabs 1 through 5 keep their functions.
The Dashboard keeps the previous journal counters. Shaping files and worker counts stay unchanged.

## Review and checks

The review covered descriptor ownership, bounded input, stale values, cache permissions, monitor lifetime, config reloads, and TUI costs.
The monitor sends one bounded request per two seconds. The table formats visible rows only.
Final review found that a stalled TUI loader could freeze the display clock.
The independent timer now advances that clock. A regression test confirms expiry without loader updates.
Review found no unresolved code issue after this fix.
The full test suite has 503 passes, zero failures, and six manual tests ignored.
Clippy with warnings denied and formatting pass.
Real local socket tests cover fragmented headers, timeouts, invalid descriptors, wrong reply kinds, and old frames.
Tests also cover failed reads, repeated timestamps, cache corruption, config reloads, and monitor shutdown.
UI checks cover all themes, narrow terminals, tab navigation, and the last worker in a long list.

The root-only production socket and live throughput were not tested in this session.
Host sudo still needs interactive authentication. Prepare the committed release and operator deployment command after push.
The graph remains unchanged. Current source and this note supersede its old monitoring descriptions.
