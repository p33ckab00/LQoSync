---
type: session
created: 2026-11-17
tags: [topology, bounds, factors, migration, integrity, reliability, ste]
status: dynamic-bounds-complete
related: ["[[2026-11-17-topology-dhcp-overrides]]", "[[2026-11-17-topology-legacy-preflight]]", "[[HANDOFF]]"]
---

# Dynamic subscriber bounds

Time: 2026-11-17T17:38:44+08:00 (Asia/Shanghai).
Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`.

The owner authorized the next task after the DHCP identity overrides.
The last pushed checkpoint remains `7d3c95f`. Both implementation slices remain local and uncommitted.

## Goal

Preserve the legacy subscriber minimum/maximum policy for source-read profiles without reliance on current sample rates.

## Changes

- Added `legacy_bounds.rs` and `legacy_bounds_tests.rs` under `src/topology_manager/`.

- Extended rate rules, catalog validation, configured legacy preflight, and module exports.

- Added tests for dormant bounds, inherited controls, store reopen, and a changed source plan after disconnect.

- Extended the DHCP reconnect test to combine independent identity overrides with imported bounds.

- Updated design, discussion, plan, Home, and handoff. Preserve unrelated operational edits.

`legacy_bounds::import` produces four explicit profile overrides. It does not edit a registry or configuration.
The existing revision-checked store persists the completed edit.
The new `LegacyRoundedBase` rule preserves the legacy three-decimal Mbps rounding function and configured floor.
The ordinary fixed/fraction rules and new policy defaults keep their behavior.

Imported factors must be positive, at most one, and exactly representable at six decimal places.
The importer requires the current configuration floors: minimum at least 0.1 Mbps and maximum at least 0.2 Mbps.
Integer bps and legacy Mbps conversions require an exact round trip.
The catalog rejects reversed factors/floors even without current subscribers.

Configured legacy adoption compares all four effective controls after inheritance.
It computes expected controls once. Integer products compare rational factors without a tolerance.
Matching imported rules remove the former unconditional dynamic-bound blocker.
Changed controls, unmatched floors, and unsupported precision keep that blocker.
Independent source-priority, group-factor, native-minimum, ownership, and exact snapshot checks remain mandatory.

Current authorized source evidence supplies each base plan.
A disconnect removes exported rows. A reconnect preserves the verified subscriber identity and uses fresh endpoints and source speeds.

The PPP test changes 50/10 Mbps to 23/7 Mbps.
At a 10% factor, minimums become 2.3/0.7 Mbps and maximums remain 23/7 Mbps.
The DHCP test combines identity-bound 20/7 Mbps with minimums of 2/0.7 Mbps.
No stored session rate supplies these calculations.

Schema 4 remains current. Existing variants keep their serialized shape.
An older reader rejects the new variant. No dependency changed.
The live service loop remains unchanged.

## Verification

- Full suite: 1,127 passed, zero failed, and 12 ignored. Nine new active tests.

- All 392 topology tests pass.

- Numerical checks compare 252 base/factor/floor cases with the legacy function. Literal cases check normal and low rates.

- Tests cover rounding boundaries, exact-rule behavior, conversion limits, strict serialization, inherited controls, and store reopen.

- Initial tests reproduced the missing rule, dormant preflight blocker, reversed-control acceptance, and unchecked import floors.

- The installed-library configured sync test passes in temporary directories. It checks pipeline compatibility without live writes.

- Format, Clippy with warnings denied, and whitespace checks pass. Author review only. No production benchmark occurred.

Logs: `.superpowers/sdd/2026-11-17-topology-dynamic-bounds/`.
Proof: `structural-red.log`, `catalog-red.log`, `limits-red.log`, `topology.log`, `full-complete.log`, `clippy-complete.log`, and `native-sync.log`.

## Limits and next action

Native export precision and planner admission still apply. This change does not prove live DPDK acceptance.
Native group minimums, authoritative static inputs, shared branches, enrollment UI, and live acceptance remain gates.
Schedules, service activation, and the Topology Manager TUI remain unfinished.

Next: native group minimum commitments. Preserve WAN ceilings and do not substitute capacity floors for guaranteed minimums.

No live source read, router write, service action, native apply, deployment, or mode switch occurred.
Unrelated operational edits remain intact. The graph remains the earlier baseline.

Plan: `docs/superpowers/plans/2026-11-17-topology-dynamic-bounds.md`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.
