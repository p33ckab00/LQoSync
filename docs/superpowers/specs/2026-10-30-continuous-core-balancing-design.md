# Continuous core balancing — design proposal

Status: operator accepted active-client failover on 2026-10-30. Implementation choices below are engineering proposals. Live migration remains on hold.
This document proposes behavior. It does not describe an implemented continuous writer.

## Goal and existing components

Use the selected sources, configurable topology, eligible node/WAN pairs, and manual pins.
Reuse the session planner for online counts and reconnect detection.
Reuse core ownership validation, exact row guards, private journals, and process fencing.
The current maintenance adapter preserves observed WAN membership. It cannot execute arbitrary WAN moves.
The current core planner describes additions and removals. Its output is not a safe execution sequence.

## Approaches

1. Reconcile new and reconnected sessions. Keep continuous sessions stable. This limits session disruption but does not equalize every scan.
2. Redistribute active sessions when counts differ. This converges faster but can interrupt connections and creates more router writes.

Proposal: use approach 1 for normal balancing. Make WAN-failure evacuation a separate configurable policy.
Operator decision: evacuate active clients from a confirmed unhealthy WAN, even if connections can interrupt.
Do not introduce periodic active-session redistribution in the first implementation.

## Operator controls

Use cog modals for the global writer controls and each WAN or source policy.
Keep status, counts, pending operations, and failures visible without expanded configuration panels.
Global modes: preview, paused, and explicitly enabled automatic reconciliation.
The initial mode is preview. Enabling the writer requires a separate operational authorization.

Retain manual and automatic source policies and configured eligible destinations.
Retain the online-client and planned-load metrics. Counts do not represent measured traffic or utilization.
Add configurable offline confirmation, probe thresholds, recovery delay, and bounded write batches.
Proposed defaults: 10-second health rounds, 30-second evidence lifetime, three failure rounds, three recovery rounds, and a 30-second recovery delay.
Use two distinct external probe targets per WAN. At least one must pass for the round to succeed.
Proposed cleanup requires two complete absence observations at least 30 seconds apart. Limit each router batch to 20 mutations.
Keep all values configurable and validated. Time thresholds use elapsed time, not merely collection counts.

## Session behavior

A fresh complete source observation confirms online identity, current IP, handle, and continuity evidence.
A confirmed offline session leaves the online count and releases its automatic assignment for reconsideration.
After the configured offline confirmation interval, remove only the exact application-owned host entry.
A failed, partial, stale, or ambiguous observation does not prove absence and cannot authorize removal.
A wholly empty scan cannot authorize mass deletion without independent confirmation under an explicit cleanup policy.
A reconnect can select the least populated eligible healthy WAN, then destination, with deterministic tie-breaking.
Continuous sessions keep compatible assignments. Manual pins take precedence over normal balancing.
An IP reused by another identity requires explicit ownership reconciliation before any addition.
An IP change removes the exact old owned row and creates the new owned host only after identity checks.

Residual ranges preserve unmanaged membership. They do not count as online subscribers.
Before an IP inside a residual range can move to another WAN, its old manual coverage must be resolved.
The writer cannot silently narrow a protected residual range. Such a conflict blocks that move for explicit review.
The rollout must define the managed address scope and approved residual handling before automatic mode can start.

## WAN health and failure policy

Track healthy, unhealthy, and unknown states with observation timestamps and reasons.
A successful inventory read or a running interface does not prove internet reachability.
Use fresh route/interface evidence and a WAN-specific internet probe mechanism with verified routing attribution.
First provider: read explicitly selected operator-configured ICMP Netwatch rows. Do not create probes or routes automatically.
Require dedicated distinct IPv4 probe targets and verified host routes through the mapped WAN, without fallback to another WAN.
Reject ambiguous output routing, route changes, disabled probes, unsupported probe forms, and stale or unverifiable results as unknown.
The provider must verify advancing completed probe evidence; repeated reads of the same result cannot satisfy consecutive rounds.
Confirm exact status fields and completion behavior against the deployed RouterOS version in an isolated fixture before implementation acceptance.
Do not use an unbound server probe as evidence for a particular WAN.
Require configurable failure and recovery thresholds to prevent repeated switching.
Exclude unhealthy or unknown WANs from new automatic assignments. If none qualify, show blocked state instead of guessing.

Provide two failure modes: stop new assignments to the failed WAN, or evacuate its active clients to eligible healthy WANs.
Accepted behavior: default to active-client evacuation when automatic mode is explicitly enabled and the WAN is confirmed unhealthy.
Unknown health blocks new placements but does not prove a failure or authorize evacuation.
For pinned clients, proposal: keep the pin and report blocked state unless the operator enables a pin failover override.
WAN recovery allows new assignments after the recovery delay. It does not force active clients back by default.
The pin override, recovery behavior, and numeric thresholds are engineering proposals. Active-client evacuation is the recorded operator decision.

## Write safety and recovery

Use one serialized writer per data directory with exclusive maintenance coordination.
Reject concurrent settings changes, maintenance execution, and superseded source evidence before each mutation.
Use exact existing row IDs and complete ownership fields. Never infer an added row ID after an uncertain response.
Preserve other owners, protected manual rows, unrelated lists, routes, and firewall rules.
Plan each change from fresh evidence and journal its intent and verified result.
Keep desired assignments, observed core membership, staged shaping output, and production applied output distinct.

A cross-list move is not atomic. Adding first can create dual membership; removing first can create a routing gap.
First supported move: keep an owned host row on the same core and change its list and ownership comment in one scoped set request.
Keep its row ID, address, static state, and enabled state. Verify the exact full row before and after the request.
This avoids an explicit add/remove pair for this move but does not prove packet-level atomicity or preserve existing connection marks.
Cross-core moves remain blocked until a separate transition procedure exists.
Test the same-core procedure on an isolated RouterOS instance before any live move.
Do not flush connection tracking or modify firewall rules automatically. Report that established flows may retain old marks.
Do not claim uninterrupted failover or atomic route/shaping updates.
An unknown request outcome freezes dependent work for reconciliation. A known-unsent failure can retry after fresh validation.
Verify actual membership after each bounded batch before publishing any claim of success.
Keep the last valid generation when source validation fails. Never label a candidate as production applied.

## Acceptance evidence

Test balanced arrivals, confirmed offline sessions, reconnects, stable sessions, manual pins, and reused IPs.
Test partial and empty scans, health flapping, all-WAN failure, recovery delay, and pinned-client failure behavior.
Test residual overlap, foreign edits, settings drift, concurrent writers, crashes, lost replies, and exact rollback.
Test count determinism and bounded collection/write work at the existing large-fixture sizes.
Use isolated RouterOS tests for the chosen cross-list transition and probe attribution before live authorization.

## Implementation boundary

Implement and test the health and reconciliation path in isolation before enabling any live writer.
Separate structural route validation from destination availability: a failed source WAN must not prevent safe evacuation to a strictly validated healthy destination.
Keep unsupported topology, probe attribution, connection-mark behavior, residual conflicts, and cross-core moves visibly blocked.
Use the companion implementation plan. Each phase must pass its tests before the next phase starts.
No production writes or restarts are part of this design task.

## Protocol references

The [Netwatch reference](https://help.mikrotik.com/docs/spaces/ROS/pages/8323208/Netwatch) describes ICMP probes and their configuration.
The [Ping manual](https://manual.mikrotik.com/docs/diagnostics-monitoring-and-troubleshooting/ping/) describes interface and VRF selection.
These documents do not prove the current server has WAN-specific probe routing. Verify actual attribution separately.
