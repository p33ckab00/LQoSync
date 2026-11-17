---
type: session
created: 2026-11-17
tags: [topology, sources, identity, runtime, integrity, reliability, ste]
status: isolated-source-coordinator-complete
related: ["[[2026-11-17-topology-runtime-owner]]", "[[HANDOFF]]"]
---

# Configured topology source coordinator

Time: 2026-11-17T11:19:15+08:00 (Asia/Shanghai).
Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`.

Base commit: `16d2b80`. The owner authorized the next implementation task after that GitHub checkpoint.
These changes remain local and uncommitted.

## Goal

Connect configured source reads to identity authorization and the controlled runtime writer.
Preserve complete fresh batches and one writer owner.

## Changes


- Added `src/topology_manager/coordinator.rs` and `coordinator_tests.rs`.

- Extended `reader.rs` with a selected-role plan. The discovery API preserves its full catalog-role behavior.

- Extended `runner.rs` with the explicit `sync_topology(compiler, max_age, now)` API and task tests.

- Reused the instance lock, registry owner, controlled compiler, receipt tracker, and automatic scheduler request.

- Updated design, discussion, plan, Home, and handoff. No dependency changed.

Selected catalog router names match exactly one configured connection.
A missing or ambiguous connection blocks the batch before source reads.
Selected protocols require the catalog role and enabled source setting.
Only selected PPP, DHCP, and Hotspot tables enter this batch. WAN discovery remains separate.
Configured WAN capacity does not require a live WAN interface read.

Parallel tasks share one deadline from before the connection phase.
Each router gets one try. Another sync collects every required source again under a new generation.
Failures and cancellation discard incomplete evidence. The coordinator never stores subscriber endpoints between batches.

Successful sessions can survive. Changed connection settings, removed sources, and config reloads close obsolete sessions.

The first connection regression reproduced data from the previous router after a change to its name/address binding.
The corrected session owner checks the current connection fields before reuse.
Fixed error categories exclude raw transport and normalization text from returned diagnostics.
A restarted runner advances past the topology generation in its recovered receipt.

## Verification


- Full suite: 1,084 passed, zero failed, and 12 ignored. Added 15 active tests.

- All 13 coordinator tests pass. They include PPP, DHCP, Hotspot, parallel batches, deadlines, cancellation, and connection changes.

- Two runner tests prove shared lock ownership and mode/target rejection before preparation.

- The optional installed-library public sync test passes in temporary directories.

- That test checks active publication, failed reads, restart, complete disconnect, and reconnect with changed IP/MAC values.

- The authorized Subscriber ID remains stable. The final output contains only the current endpoint.

- Format, Clippy with warnings denied, and whitespace checks pass.

- Author review only. No independent reviewer or production benchmark participated.

Logs: `.superpowers/sdd/2026-11-17-topology-source-coordinator/`.
Proof: `full.log`, `clippy.log`, `coordinator-verified.log`, `native-sync.log`, and the red regression logs.
Native test command: `LQOSYNC3_RUNTIME_BINDING=/opt/libreqos/src cargo test --lib runner::tests::installed_binding_runs_configured_source_sync_without_live_writes -- --ignored --exact --nocapture`.

## Limits and next action

The legacy service loop remains unchanged. The explicit API does not select manager mode automatically.
No live RouterOS read, native DPDK apply, service action, router write, deployment, commit, or push occurred in this task.
The native test copied the installed library and example config into temporary directories.
Transport allocation still follows the current RouterOS implementation. Deadline and pair limits do not prove a hard transport memory ceiling.

Next: prove parity with legacy policies and activation preflight behavior.
Exclusions, DHCP address overrides, minimum/factor policies, static-input authority, and shared static branches remain gates.
Independent identity enrollment, native planner bounds, and live DPDK acceptance remain gates.
TUI, schedules, and complete recovery from a corrupt store remain separate tasks.
The graph remains the earlier baseline. Current files and tests take precedence.

Plan: `docs/superpowers/plans/2026-11-17-topology-source-coordinator.md`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.
