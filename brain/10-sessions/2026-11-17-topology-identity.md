---
type: session
created: 2026-11-17
tags: [topology, identity, authority, reliability, performance, ste]
status: identity-authorization-complete
related: ["[[2026-11-17-topology-normalization]]", "[[HANDOFF]]"]
---

# Independent subscriber reference authorization

The owner approves the next identity authorization task.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `489acc7` plus local normalization.

Agent: Codex `/root`. Timestamp: 2026-11-17T02:38:16+08:00, Asia/Shanghai.
The normalization and identity slices remain local and uncommitted. No push, deployment, router write, service, or configuration changed.
Unrelated operational work remains intact.

## Changes

Added `src/topology_manager/identity.rs` and `identity_tests.rs`.
Extended normalized protocol provenance/deadline metadata and module exports.
Added a public integration check in `tests/topology_catalog.rs`.
Updated the design, discussion, source/identity plans, Home, and handoff notes.

The API consumes normalized current routers and explicit selected scopes under an expected registry revision.
It requires matching generations, complete selected protocol reads, and fresh evidence.
A WAN-only read cannot establish an empty PPP/DHCP/Hotspot source.
Selected scopes match the required catalog scopes where a catalog exists.

Source authority is explicit: Unconfigured or OperatorManagedComment.
The optional operator contract uses `[lqosync-id:REFERENCE]` in current administrator-managed account/lease comments.
The operator assigns an independent reference and never reuses it for another subscriber within that scope.
The namespace `operator-comment/v1/` separates these references from other authority types.
No code enables this contract or writes a marker to a router automatically.

Missing, invalid, duplicate, or ambiguous selected references withhold the complete collection.
No account name, hostname, MAC, IP, client ID, or row locator acts as a fallback identity reference.
Equal references on different routers/protocols do not merge circuits. Existing explicit registry aliases remain effective.
Changed references create distinct identities without the prior pin. Verified returns keep saved intent and fresh endpoints.
Offline reads keep intent without active endpoints or durable address fields.

Indexed service selection avoids a full source scan per account.
Overlapping sources and partial multi-service membership remain typed conflicts.
Authorization reuses the observation gate and registry reconciliation, with freshness checks before/after both stages.
The returned world carries the original minimum read deadline for later placement/publication.
Current numeric source rates remain absent. Explicit local profile rates can reach the existing placement/capacity pipeline.

## Integrity correction

A trial Hotspot session could inherit local user metadata when its generated name matched a configured user.
The new authorization regression reproduced the unintended acceptance before its fix.
Normalization now keeps trial sessions separate and never borrows that user's reference or profile.
Named and MAC local logins still require the explicit independent operator-reference contract.

## Checks

Initial red run: 15 failed, 2 passed. All 22 focused identity checks now pass.
The trial regression failed before the fix and passes after it.
The public integration check covers offline removal, restart, verified rename/return, restored pin, and new endpoints.
It reaches placement and capacity evaluation with an unchanged 50 Mbps subscriber maximum.

Full suite: 950 passed, zero failed, 9 ignored. Format and Clippy with denied warnings passed.
Whitespace and stub scans passed. No dependency, schema, or legacy path changed.
A reversed 10,000-account/10,000-service check yields the same IDs and current output.
This synthetic evidence does not establish production throughput.
The STE checker is heuristic. Manual review does not establish certified compliance.

Plan: `docs/superpowers/plans/2026-11-17-topology-identity.md`.
Logs: `.superpowers/sdd/2026-11-17-topology-identity/`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.

## Limits and next action

This contract trusts the operator's uniqueness and non-reassignment assertion.
Software cannot detect a false assertion when a reference moves during an unobserved replacement.
The marker does not make RouterOS names/locators immutable and does not prove physical routing.
Trial guests without a separate independent reference remain conflicts.

Authority selection and enrollment still need durable configuration and UI integration.
Stable scope to current service mapping remains the caller's explicit responsibility.
Exact source-rate parsing remains unfinished. SourceRead rate authority has no invented numeric fallback.
The returned deadline must remain attached through later placement and publication checks.
The API does not commit candidates or publish outputs. Native admission and DPDK proof remain separate gates.

Next: source-scope mapping and exact source-rate integration.
Then complete migration/recovery, export/native integration, TUI, and schedules.
Do not enable manager mode before its remaining gates pass.
