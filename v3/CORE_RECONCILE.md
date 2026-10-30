# Health-aware core reconciliation planner

The `core_reconcile` library produces decisions from fresh source observations, actual core rows, and WAN health evidence.
The [Core control preview](CORE_CONTROL.md) calls this library from the scheduler and after configuration saves.
The library has no RouterOS mutation method.
It does not save assignments or publish shaping output.
A separate [guarded writer library](CORE_WRITER.md) implements bounded execution and recovery.
Live worker integration and production authorization remain separate implementation phases.

## Inputs and validation

`plan(Input)` requires dynamic topology, configured core bindings, a current session observation, source/core inventories, and optional previous observation and cleanup evidence.
The caller must get the session observation from the same complete source reads through the existing session planner.
The library validates observation structure and collection timestamps, then recomputes source normalization.
It does not independently prove collection tickets or physical client connectivity.
A bound DHCP lease proves lease presence, consistent with the existing session contract.

Incomplete or stale evidence returns an error. The caller must discard cleanup evidence after any error.
Do not bridge a failed read with old absence confirmations.
Source and core evidence use a configurable maximum age, with a 30-second default.

Health must match the canonical settings revision and WAN identity.
Its last completed sample and last evaluation must both remain fresh under the smaller configured/caller lifetime.
A recent read of an unchanged counter cannot extend sample freshness.

## Assignment behavior

The planner reuses the existing deterministic target scoring and session continuity rules.
Retained and pinned active clients contribute to load before pending clients receive destinations.
Online-count policies use WAN count, destination count, and stable target identity for ties.
Planned-load policies keep their existing capacity/rate scoring.
Manual arrivals follow the configured default pair. Manual pins take precedence over normal balancing.

New or reconnected automatic clients choose eligible destinations with fresh Healthy evidence.
Compatible continuous sessions keep their pair, including during Unknown health.
A confirmed Unhealthy WAN triggers evacuation when `evacuate_active` is enabled, which is the default.
Pinned clients block on a failed WAN unless the operator explicitly enables `pin_failover`.
Recovery does not force active clients back to their previous WAN.
No eligible healthy target produces a Blocked decision instead of an invented fallback.

The proposed assignments are desired state. They do not prove observed membership or applied shaping.
Blocked decisions must remain visible. A writer must not persist their proposed assignments as successful operations.
Fresh actual validation remains mandatory before publication.

## Decisions

- Keep: the selected owned host already has compatible actual membership.
- Add: a host is absent and its proposed destination is healthy.
- Move: change an existing owned host's list and ownership comment on the same core, with the same row ID and address.
- Remove: a prior source session has confirmed absence after the configured grace interval.
- Blocked: the planner cannot justify a supported transition.

IP changes require a separate guarded multi-operation transition and remain Blocked in this phase.
Cross-core moves, reused IPs with another owner, protected coverage overlaps, and unavailable destination evidence remain Blocked.
The planner never narrows manual/residual ranges or adopts another application's entries.
Malformed ownership, duplicate IDs/hosts, ambiguous addresses, and unsupported routing structures reject the plan.

## Offline confirmation

Cleanup requires a known prior source and at least two distinct complete source observations.
The default grace interval is 30 seconds. Repeated timestamps do not count as a new observation.
A source gap longer than the evidence lifetime restarts the confirmation interval.

A wholly empty selected snapshot does not authorize mass removal or accumulate cleanup authority.
A source/settings/row change invalidates the matching absence receipt.
Only the exact application-owned static host can receive a Remove decision.

## Route availability

The existing structural route analyzer still validates every configured consumer, table, and resolved interface.
A freshly confirmed Unhealthy source may have an inactive default if that configured default still passes structural checks.
The default must remain unique and enabled. Ambiguous or unresolved routes remain unsupported.
Healthy destinations still need an active valid default route and a unique enabled/running interface in the current inventory.

The original strict analyzer remains unchanged for existing publication and diagnostic callers.
The planner does not change or simulate router tables to bypass validation.

This exception does not resolve the health provider's physical-link-down limitation.
If probe attribution cannot be proved, health stays Unknown and cannot authorize evacuation.
Changing a list does not prove that existing connection marks or production shaping moved.

## Verification boundary

Tests use synthetic source/core inventories and health samples.
They cover arrivals, reconnects, stable sessions, pin override, manual mode, unhealthy sources, stale health, offline timing, residuals, IP reuse, and cross-core blocking.
They also test strict rejection of malformed healthy destination routes and settings changes after health evaluation.
No live router reads, writes, settings changes, or restart form part of this implementation.
See [WAN health preview](WAN_HEALTH.md) and [the implementation plan](../docs/superpowers/plans/2026-10-30-continuous-core-balancing.md).
