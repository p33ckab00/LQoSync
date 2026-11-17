---
type: session
created: 2026-11-17
tags: [topology, dhcp, identity, integrity, reliability, ste]
status: identity-overrides-complete
related: ["[[2026-11-17-topology-exclusions]]", "[[2026-11-17-topology-identity]]", "[[HANDOFF]]"]
---

# DHCP overrides through independent identity

Time: 2026-11-17T16:16:18+08:00 (Asia/Shanghai).
Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`.

The owner authorized continuation after the exclusion checkpoint.
The exclusion checkpoint is committed and pushed as `7d3c95f`.
These DHCP changes remain local and uncommitted.

## Goal

Connect configured DHCP overrides to enrolled independent subscriber references without endpoint-based identity inference.

## Changes


- Added `DhcpOverrideBinding` in source policy. Each entry contains a legacy list position, an account reference, and exact rates.

- Added `src/topology_manager/dhcp_overrides.rs` for configured source checks.

- Extended model validation, source policy, authorization, settings order, preflight, and publication diagnostics.

- Extended coordinator, preflight, source policy, and store tests. Updated two explicit source policy fixtures and the runner assertion.

- Updated the design, discussion, task plan, Home, and handoff.

The list position selects a configured rate entry. The enrolled reference identifies the subscriber within its selected DHCP scope.
The policy does not contain a MAC, IP, hostname, or lease locator.
The legacy configuration format and collector remain unchanged.

Source checks require complete coverage across the selected scopes of each router.
Duplicate positions, duplicate references, unknown identities, conflicting alias rates, and changed configured rates block the batch.
Effective profiles must use `SourceRead` base authority. Local bases cannot hide the override.

A final review found an incorrect block from an unselected catalog source.
A reproduced regression now limits that check to selected sources.
These checks include offline subscribers and occur before connection.

Independent authorization precedes rate selection.
An override uses a reference index and precedes fallback source text.
Subscriber min/max controls, placement, group factors, and WAN ceilings keep their rules.
Disconnects remove exported rows and automatic groups. Fresh reconnects keep the authorized identity and override.
Another subscriber that receives the old MAC does not inherit that override.

The transient source-policy digest now covers configured DHCP override rates.
It reads each router's rate list once across its scopes.
Changed or removed rates invalidate prepared output before configured publication or adoption.
Legacy MACs and labels do not affect manager subscriber selection.
Identity and profile indexes avoid repeated catalog scans for each override.

The field defaults to an empty list and omits empty lists from serialized output.
Schema 4 remains current. Older readers reject the nonempty unknown field.
No dependency changed.

## Verification


- Full suite: 1,118 passed, zero failed, and 12 ignored. Ten new active tests.

- All 383 topology tests pass in the full suite.

- The installed-library public sync test passes in temporary directories. It checks the configured pipeline without live writes.

- Store reopen preserves the bindings. Stale edits leave the current document intact.

- Format, Clippy with warnings denied, and whitespace checks pass.

- Initial behavior tests failed before the implementation. An initial full run also found an outdated diagnostic assertion.

- The author corrected that assertion and repeated the full checks. Author review only. No production benchmark occurred.

Logs: `.superpowers/sdd/2026-11-17-topology-dhcp-overrides/`.
Proof: `red-behavior.log`, `coordinator-red.log`, `coordinator-green.log`, `full-complete.log`, `clippy-complete.log`, and `native-sync.log`.

## Limits and next action

Bindings require completed revision-checked edits through the source settings API. The TUI workflow does not yet exist.
Source-read profiles still produce a dynamic-bound blocker during configured legacy adoption.
Native group minimums, static-input authority, shared branches, enrollment UI, native planner bounds, and live acceptance remain gates.
Schedules, service activation, and the Topology Manager TUI remain unfinished.

Next: migrate dynamic subscriber min/max bounds with explicit factors and exact limit checks.
Preserve group minimums as a separate native-admission requirement.

No live source read, router write, native apply, service action, deployment, or automatic mode switch occurred.
Unrelated operational edits remain intact. The graph remains the earlier baseline.

Plan: `docs/superpowers/plans/2026-11-17-topology-dhcp-overrides.md`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.
