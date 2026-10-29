# Flexible topology and reconnect assignment

The user authorized implementation after selecting the NOC design and describing the session policy.
The user also authorized performance review, correction, and commit/push after all checks pass.

## Operator behavior

- Select whole PPP profiles, DHCP servers, or Hotspot profiles from fresh inventory.
- Select individual PPP accounts, DHCP MAC identities, or Hotspot accounts.
- An individual selection overrides a group selection. An exclusion omits a selected record without RouterOS deletion.
- Bind sources to configured nodes or create a branch from a discovered source.
- Arrange branches through their parent controls. Named group nodes can contain different subscriber services.
- Keep manual assignments, manual pins, and automatic pools available together.
- Keep current legacy source bindings readable. Reject duplicate ownership rather than silently claiming a source.

## Assignment policy

New automatic pools use online client count. Keep the explicit planned-load policy as an option.
Saved pools without a metric use online client count after this update, as requested by the user.
Pinned clients use their explicit destination. Continuous sessions keep a compatible destination.
New sessions and reconnects choose the eligible WAN with the smallest active client count.
Use destination counts and stable IDs to resolve equal WAN counts deterministically.
Do not move continuous clients merely because another client disconnects.

PPP and Hotspot active records provide session handles and uptime evidence where available.
DHCP bound leases provide lease presence, not proof of physical device connectivity.
A healthy observed absence permits reassignment when the identity returns.
A changed session handle or reset uptime permits reassignment even when polling missed the absent interval.
Polling cannot establish a reconnect that leaves no observable evidence.
Failed, stale, partial, conflicting, and out-of-order reads must not create absence evidence.

## State and gates

Keep session evidence and proposed assignments in `state/online-assignments.toml`.
Keep committed `assignments.toml` unchanged in format. Session timestamps must not cause generation churn.
Save candidate evidence only after settings, read epochs, inventories, and generation checks still match.
A healthy empty scan records absence but does not publish an empty output pair.
A later core or capacity failure does not erase valid session evidence or replace the committed output.
Publication and live validation use the same assignment planner and candidate baseline.
Core plans and shaping output consume the same subscriber parents. Router writes remain disabled.
Never claim protected manual core members or bypass their publication gate.

## Approach and scope

Extend the current Rust model, normalizer, assignment planner, and Maud controls.
An independent selector list gives sources their own ownership without treating each discovered record as a queue.
Rebuilding the topology schema or adding a frontend framework would add unrelated work.
Keep one process per data directory. Preserve the old committed generation and validate new schema defaults.
Do not change credentials, production settings, packages, RouterOS configuration, or live services.

## Proof

Test selector precedence, generic groups, source health, token checks, and safe exclusions.
Test count balancing, continuous sessions, reconnects, full absence, pins, restart, and failed observations.
Test that core conflicts and empty scans preserve committed output while candidate evidence advances safely.
Run the full Rust gates, focused browser checks, large-input workloads, and independent performance review.
Fix material findings before the final commit and push.
