---
type: session
created: 2026-11-17
tags: [topology, migration, preflight, compatibility, integrity, reliability, ste]
status: conservative-preflight-complete
related: ["[[2026-11-17-topology-source-coordinator]]", "[[HANDOFF]]", "[[ADR-0009-dhcp-discovery-and-exclusions]]"]
---

# Legacy configuration preflight

Time: 2026-11-17T14:54:12+08:00 (Asia/Shanghai).
Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`.

The owner authorized the source-coordinator commit and push, then the next implementation task.
The checkpoint is `e222547`, pushed to `origin/lqosync3`.
The current preflight changes remain local and uncommitted.

## Goal

Require configured sync and legacy adoption to expose unsupported policies.
Check dormant policy declarations as well as current output.

## Changes

- Added `src/topology_manager/preflight.rs` and `preflight_tests.rs`.

- Extended the coordinator, its tests, module exports, ownership adoption, and the optional native runner test.

- Added `ownership::adopt_configured_legacy`. It checks configuration before the current exact snapshot adoption path.

- Updated implementation design, discussion, task plan, Home, and handoff. No dependency or registry schema changed.

The first source regression reproduced collection despite an excluded IP range.
The configured coordinator now requires a clear source report before it connects.
The report checks global/router IP exclusions, selected PPP profile exclusions, selected DHCP server exclusions, and DHCP client overrides.
Unrelated protocol settings do not block a scope.

DHCP membership requires bound leases and current enabled server evidence.
The fresh path requires `require_bound = true` and `shape_unassigned = false`.
Without legacy discovery, explicit service names must equal the configured server set.
The legacy default for unassigned leases can thus produce a compatibility issue.

The adoption report checks configured source priorities and sustained base authority.
It compares local profile bounds through the current legacy rate function and exact integer-bps conversion.
It also checks formulas, effective factors, backstop caps, fixed capacities, and native minimum declarations for selected groups.
Different DHCP service policies require separate scopes. The comparison retains the legacy first-per-plan-server factor rule.

Factor comparison uses integer products without float tolerance.
This first checker requires exact configured conversion at six decimal places.
All `SourceRead` bounds remain unproved, also when a source priority could provide a constant rate.
A capacity floor does not preserve a native minimum commitment.

The report contains scope IDs, setting paths, catalog IDs, and typed issue kinds.
It does not copy endpoint overrides, credentials, or source comments into diagnostics.
Indexes share profile and subscriber-assignment data across source checks.
These checks do not prove production throughput or a hard memory bound for the transport.

## Verification

- Full suite: 1,096 passed, zero failed, and 12 ignored. Added 12 active tests.

- All 11 preflight tests pass. They cover dormant bounds, priority/base authority, factors, caps, minimums, and DHCP rules.

- All 14 coordinator tests pass. The new regression blocks unsupported source policies before connection.

- The optional installed-library public sync test passes entirely in temporary directories.

- The native test rejects an excluded-source configuration before any pair exists. Active/disconnected/reconnected batches still pass after that conflict clears.

- Format, Clippy with warnings denied, and whitespace checks pass.

- Author review only. No independent reviewer participated.

Logs: `.superpowers/sdd/2026-11-17-topology-legacy-preflight/`.
Proof: `full.log`, `clippy.log`, `verified.log`, `coordinator.log`, `native-sync.log`, and the red regression logs.

## Limits and next action

The report exposes unsupported behavior. It does not implement exclusion or endpoint-override migration.
A clear report does not authorize service activation or prove native DPDK acceptance.
Neither adoption adapter commits the registry or writes a shaping pair.
The low-level snapshot adapter remains available for isolated diagnostics.

No live RouterOS read, router write, service action, native apply, deployment, or automatic mode switch occurred.
Native test writes went to temporary directories. Unrelated operational edits remain intact.

Next: implement exclusion filters against fresh source evidence with parity tests.
Keep MAC overrides blocked until independent identity mappings exist.
Then extend dynamic bounds and native node minimums without silent rate changes.
Static-input authority, shared branches, enrollment, native planner bounds, and live acceptance remain gates.
The graph remains the earlier baseline. Current code and tests take precedence.

Plan: `docs/superpowers/plans/2026-11-17-topology-legacy-preflight.md`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.
