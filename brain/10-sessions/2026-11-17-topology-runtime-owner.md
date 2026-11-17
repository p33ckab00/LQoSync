---
type: session
created: 2026-11-17
tags: [topology, compiler, runtime, integrity, reliability, ste]
status: isolated-runtime-owner-complete
related: ["[[2026-11-17-topology-legacy-adoption]]", "[[HANDOFF]]"]
---

# Controlled compiler and runtime owner

Time: 2026-11-17T10:58:55+08:00 (Asia/Shanghai).
Agent: Codex `/root`.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`.

Base: `b25e0cf53162a4d05e217b2e6fece39fe4efa130`.
The owner authorized implementation. Current changes remain local and uncommitted.

## Goal

Bind compilation to current library and config bytes.
Use one instance lock, registry owner, and receipt tracker for manager publication.

## Changes

- Added `compiler.rs`, `compiler_sandbox.rs`, `compiler_tests.rs`, and `runtime.rs` under `src/topology_manager/`.
- Extended `files.rs`, `runner.rs`, registry ownership, admission, publication, receipts, and apply evidence.
- Added the public `Runner::publish_topology` API. The current service loop does not call it yet.
- Shared guards check the state directory and lock file identities. Replaced paths lose write authority.
- The compiler copies exact library bytes into a private cache. Each run gets a new private workspace.
- Compilation has a deadline and byte limits. Landlock restricts content writes. Seccomp denies socket operations.
- Unsupported kernel controls block compilation. These controls isolate trusted compiler operations, not arbitrary hostile programs.
- Native config migration can change private config values. Admission preserves topology mode, source paths, and exact pair bytes.
- Receipts record original compiler identity and the normalized effective config digest.
- A changed compiler identity creates a new receipt even when the pair bytes match.
- Publication and acknowledgement checks reject drift in the original library or config.
- A durable ownership marker blocks legacy publication after restart. Older executables need an explicit downgrade migration.
- Publication requests the current scheduler import automatically. Retries keep the receipt identity and pending state.
- Updated design, discussion, plan, and receipt regression tests. No dependency changed.

## Native discovery

The installed library needs to migrate its private config copy.
The first scratch test exposed this requirement. The compiler now permits writes within the whole private run workspace.
It checks the effective config after the child exits. It does not change the original config.

The scratch tests used `/opt/libreqos/src/liblqos_python.so` from package `3.0.7.202611162151`.
Its SHA256 is `b62cf25ed8c25b443c7a7676d46941cd94558a96195e2ae4881de34d96bbaab2`.
The host has x86_64 and Landlock ABI 4. ARM paths have no execution proof in this task.

## Verification

- Full suite: 1,069 passed, zero failed, 11 ignored. Topology subset: 337 passed.
- Both optional native tests pass: isolated compilation and the public manager API.
- The public API test covers active and disconnected subscriber batches in temporary directories.
- Format, Clippy with warnings denied, and whitespace checks pass.
- Regression tests cover lock replacement, ownership restart, config drift, receipt identity, compiler boundaries, and automatic reload requests.
- Author review only. No production benchmark or independent review occurred.
- Logs: `.superpowers/sdd/2026-11-17-topology-runtime-owner/`.
- Final suite log: `full-verified.log`. Static check log: `clippy-final.log`.

No live activation, service action, router write, native DPDK apply, or deployment occurred.
All native test writes went to temporary directories. Other operational edits remain separate.

## Next action

Connect configured source reads and identity authorization to the service loop.
Then prove parity with legacy policies and check activation requirements.
Exclusions, address-based overrides, minimum/factor policies, and shared static branches remain gates.
Independent identity enrollment, native planner bounds, and live DPDK proof also remain gates.
TUI and schedules remain separate tasks. Complete recovery from a corrupt store also remains a separate task.
The project graph remains the earlier baseline. Current code and tests take precedence.

Plan: `docs/superpowers/plans/2026-11-17-topology-runtime-owner.md`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.
