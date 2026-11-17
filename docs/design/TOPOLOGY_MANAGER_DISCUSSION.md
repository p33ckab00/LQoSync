# Topology Manager discussion

Status: eleven behavior decisions and four supplemental detail groups approved, with the recorded clarifications.
Concrete implementation contracts and verification remain outstanding.
Recorded: 2026-11-16, Asia/Shanghai.

The [product review](../reports/2026-11-16-topology-product-review.md) compares this record with current code and tests.
The [implementation design](TOPOLOGY_MANAGER_IMPLEMENTATION.md) defines proposed module and migration boundaries.
The [foundation plan](../superpowers/plans/2026-11-16-topology-foundation.md) covers the first isolated implementation slice.
This file remains the behavior approval record. Review recommendations do not silently amend owner decisions.

## Purpose

LQoSync3 will collect candidates and let the operator build an approved topology.
The operator selects discovered nodes, creates custom nodes, and arranges their hierarchy.
The manager supports promotion, demotion, and moves between branches.
Auto-population and auto-balancing assign subscribers according to operator rules.

Research and source evidence: [adoption report](../reports/2026-11-16-topology-manager-adoption.md).
That report proposes implementation paths. It does not approve a native ingress migration.

## Confirmed direction from the discussion

### Discovery and selection

- Show discovered candidates explicitly, including candidates that the operator has not selected.
- Candidates include WANs, DHCP servers, PPP services, Hotspot services, and PPP profiles.
- The core router supplies WAN discovery. Distro routers supply subscriber discovery.
- A router can have multiple collection roles. A role selects which information the collector reads.
- The operator decides which candidates become topology nodes.
- PPP, DHCP, and Hotspot are optional groups. They do not dictate the hierarchy.
- Custom nodes can represent operator-defined groups or sites.

### Hierarchy and placements

- Permit WAN-only topology with subscribers directly below each WAN.
- Permit profile-only topology without WAN nodes.
- Permit custom and discovered child nodes below a WAN or another selected parent.
- Permit multiple top-level WAN nodes under a synthetic root.
- A WAN can be the only actual top-level node.
- Position in the hierarchy does not change a node's capacity policy.
- Permit a custom display label for the synthetic root in LQoSync.
- Do not assume that this label renames the native LibreQoS synthetic root.
- One discovered source can supply multiple topology instances under different parents.
- A shared child selection can create one instance below each selected WAN.
- An explicit child placement applies only to its specified parent.
- Show the generated instances in the output preview.

### Capacity and factors

- WAN base download and upload capacities are operator-defined. WAN factors are optional.
- Profile and group nodes directly above subscribers use the current automatic capacity calculation and group factors.
- This rule also applies when a profile or group is directly below the synthetic root.
- A child instance uses its assigned subscribers for its computed capacity.
- Assignment changes require a new calculation for affected computed nodes.
- Subscriber plan limits remain separate from aggregate node capacities.
- Ancestors without direct subscribers must not repeat subscriber-based calculation. Supplemental detail 1 defines their capacity modes.

### Profiles and subscriber placement

- One profile can serve subscribers assigned to different WANs.
- WAN assignment must not require a separate profile for each WAN.
- Subscriber source type does not require a separate branch.
- Auto-population and balancing operate on the operator's selected topology.
- Decisions 3 through 6 define eligibility, balancing metrics, pins, and rebalance triggers.

### Settings ownership

- Move node-specific capacities, factors, and related policy controls into the Topology Manager.
- Keep router connections and collection settings in the Settings TUI.
- Retain current values and computation results during the settings migration.
- Avoid duplicate editable controls for the same node policy.
- Global defaults and a separate profile editor were recommendations. Their exact scope remains open.

### Scope and terminology

- Implement the Topology Manager before the routing task.
- Current WAN assignments describe intended topology placement.
- Actual WAN routing does not need to be identified on the distro router for this design discussion.
- Routing implementation and verification are separate future work.
- Future routing includes application routing and subscriber routing.
- Traffic intelligence obtains packet and IP information to identify applications, CDNs, ASNs, and organizations.
- Traffic intelligence is a separate function. Do not call it topology intelligence.
- Do not claim that intended WAN placement proves the route of actual packets.

## Decision tracker

The owner approved the eleven behavior decisions through individual discussion rounds.
The latest recorded clarification takes precedence over an earlier proposal.
Do not treat unresolved implementation details as selected defaults.

| Number | Decision | State |
|---|---|---|
| 1 | Source identity, instance identity, and instance-specific settings | Approved |
| 2 | Shared selection versus explicit placement behavior | Approved |
| 3 | Subscriber eligibility rules | Approved |
| 4 | Balancing metric and capacity constraints | Approved |
| 5 | Rebalance triggers and movement limits | Approved |
| 6 | Manual pins and exceptions | Approved, with subscriber lifecycle requirements |
| 7 | Custom-node capacity policies and defaults | Approved |
| 8 | Nested calculations and factor application | Approved, with WAN ceilings and visible subscriber speed controls |
| 9 | Discovery rename, disappearance, and reconciliation | Approved |
| 10 | Unassigned subscribers and insufficient capacity | Approved |
| 11 | Automatic publication, optional preview, native verification, and recovery | Approved lifecycle; detailed recovery design remains open |

Deletion, promotion, and demotion must specify the destination of affected children and subscribers.
Discuss those consequences with placement behavior and the edit lifecycle.

## Decision 1 — Source identity and topology instances

State: approved by the owner on 2026-11-16, Asia/Shanghai.
The owner explicitly requires reliability and integrity.

Keep a source ID for each discovered object and an instance ID for each topology placement.
Instances of one source share its source definition, such as profile speed.
Each instance has its own parent and subscriber membership.
Each instance computes its own aggregate capacity from its assigned subscribers.
Decision 8 and supplemental detail 1 restrict subscriber-based calculation to an immediate subscriber group.
WAN and intermediate instances use their separately approved capacity modes.
Keep IDs stable across display-name edits and moves.
Keep shared default factors on the source definition.
Each instance can explicitly override those factors.
An instance without an override inherits the source defaults.
An instance with an override retains its values when the source defaults change.
Show inherited and overridden values distinctly.

Example: one 100 Mbps profile appears below WAN 1 and WAN 2.
The profile remains one source. The two placements are separate instances.
Subscribers must not duplicate merely because the profile has two placements.

### Reliability and integrity requirements

Enforce these requirements when the implementation is designed and tested:

- Keep source identities distinct across routers and source types. Display names alone cannot identify an object.
- Persist instance IDs. A refresh, restart, rename, or move must not regenerate an instance ID.
- Validate source and parent references before an edit becomes effective.
- Reject duplicate IDs, self-parent relationships, and cycles.
- Reject invalid factor values before capacity calculation or publication.
- Resolve factor inheritance deterministically. Keep explicit overrides separate from inherited values.
- Preserve one effective placement per subscriber in the generated tree.
- A failed or incomplete update must not replace the last valid approved revision.
- Verify identity stability, inheritance, and subscriber uniqueness with focused implementation tests.

These are requirements. No runtime proof exists for the proposed model yet.
Revision transactions and full apply recovery remain decision 11.

## Decision 2 — Shared selection and explicit placement

State: approved by the owner on 2026-11-16, Asia/Shanghai.

Use explicit placement rules to distinguish shared child selections from individual placements.
A shared rule targets an explicit set of selected parents and creates one instance per target.
A new WAN does not join that rule until the operator adds it to the target set.
An explicit placement targets one parent only.
Do not create duplicate instances when an explicit placement overlaps a shared rule for the same source and parent.
Show the resolved instances and affected subscribers in the preview.
Removal behavior for an instance with children or subscribers requires an explicit reassignment decision.

## Decision 3 — Subscriber eligibility

State: approved by the owner on 2026-11-16, Asia/Shanghai, after clarification of optional restrictions.

Define eligibility before the balancer selects an instance.
A source-backed node starts with subscribers that belong to that source.
For example, a profile node starts with subscribers assigned to that profile.
A DHCP server node starts with subscribers discovered from that server.
Use stable source references across routers. A shared display name must not merge unrelated sources.

WAN and custom nodes can use an explicit all-subscribers pool or operator filters.
Candidate filters include distro router, subscriber source type, and profile or service identity.
The operator must explicitly select the pool. Do not silently select all subscribers.
Once the operator selects a shared pool and its WAN destinations, permit assignments across all those WANs by default.
One profile can supply the entire pool. It does not restrict that pool to one WAN.
Profile nodes are optional. A WAN-only topology receives subscribers directly.
Only explicit operator restrictions narrow the allowed destinations.
Examples include a distro restricted to one WAN, a profile restricted to selected WANs, or a source-type filter.
Explicit exclusions remove selected subscribers from the auto-assignment pool.
Each child pool is restricted to the pool allowed by its ancestors.
Multiple placements of one source share the eligible pool. They do not duplicate subscribers.
The balancer assigns each subscriber to one eligible effective placement.
Expose matches and exclusions in the preview without requiring preview approval before normal automatic publication.

Manual pins and precedence remain decision 6.
The treatment of subscribers without an eligible placement remains decision 10.

## Decision 4 — Balancing metric and capacity constraints

State: approved by the owner on 2026-11-16, Asia/Shanghai.
Approval covers capacity-weighted plan allocation when destinations have configured capacities.
The basis for groups without independent capacities remains decision 8.

Use capacity-weighted plan allocation for the initial topology balancer.
Subscriber count alone must not determine the distribution when plans differ.
Compare the sum of assigned plan rates against each destination's effective directional capacity.
Effective WAN capacities include the configured WAN factors.
Evaluate download and upload separately. Do not ignore an asymmetric upload bottleneck.
This metric represents assigned plan demand. It does not measure actual packet load.

Respect approved eligibility restrictions and future pin rules before assignment.
Use deterministic choices when destinations have equal scores.
Do not use a child capacity derived from current members as the sole input for assigning those same members.
Nested computation details remain decision 8.
Rebalance triggers and limits remain decision 5.
Oversubscription must remain visible. Exact admission and fallback behavior remain decision 10.

Example: WAN capacities have a 2:1 ratio in both directions, and all subscribers have the same plan.
For 60 eligible subscribers, approximately 40 and 20 assignments fit the target ratio.
This example does not prove that all subscribers can reach their plan rates simultaneously.

## Decision 5 — Rebalance triggers and movement limits

State: approved by the owner on 2026-11-16, Asia/Shanghai.

Automatically assign a new eligible subscriber under an approved allocation policy.
Retain valid assignments for current subscribers during routine collection.
Do not move subscribers merely because collection order changes.
Do not infer a WAN failure from a failed collection or temporary absence.

For a capacity, factor, eligibility, or hierarchy edit, calculate a proposed allocation.
Expose affected subscribers and computed capacities in the preview.
The owner's decision 11 clarification requires automatic publication without a separate operator Apply action.
Require a valid reassignment plan before an edit invalidates current placements.
Offer an explicit rebalance action for the selected pool or branch.
Limit each action to that scope, retain future pins, and prefer fewer moves when allocations have equal quality.

Continuous background moves remain disabled for the initial version.
They require a separate policy for thresholds, hold periods, and maximum moves per interval.
Do not implement timer-based redistribution as an unstated default.
This proposal does not implement routing failover or observe packet-level load.

## Decision 6 — Manual assignments, pins, and exceptions

State: approved by the owner on 2026-11-16, Asia/Shanghai.
Approval includes the subscriber lifecycle and fresh-data requirements below.
The owner also explicitly approved the fresh-data clarification before discussion of decision 7.

Allow automatic assignment or an explicit manual assignment with a pin.
A pin binds a stable subscriber identity to a specific topology instance.
One placement and pin belong to a subscriber circuit, not separately to each device row.
All verified current endpoints of that circuit share its policy and placement.
Do not use an IP address alone as the subscriber identity.
Show the assigned instance, full ancestor path, and pin state.
An explicit rebalance must retain valid pins and include their plan demand in destination totals.
Balance automatic subscribers around those fixed assignments.

A pin does not override source membership, ancestor eligibility, or structural validation.
Reject an invalid manual destination and explain the failed rule.
Require an explicit eligibility edit before an exception can permit that destination.
Preview the effect of that exception. Do not silently expand eligibility for other subscribers.

If an edit would delete a pinned instance or invalidate its eligibility, block that edit until the operator resolves the pin.
If an ancestor move changes the WAN path of a pinned instance, disclose that change in the preview.
A pin locks the instance assignment. It does not independently lock the entire ancestor path.
If discovery fails or an external change creates a pin conflict, retain the saved intent and show the conflict.
Do not silently remove the pin, select a different destination, or publish an invalid new revision.

Unpin returns the subscriber to automatic eligibility.
Retain its current placement if that placement remains valid, as decision 5 requires.
Move it only through an approved reassignment or rebalance action.
Capacity warnings and insufficient-capacity behavior remain decision 10.

### Subscriber identity, offline removal, and fresh return data

The owner explicitly requires identity integrity across disconnect and reconnect events.
Keep durable subscriber identity and saved policy separate from current session observations.
An IP address or MAC address can change. Neither alone proves durable identity.
Never reuse stored IP addresses or MAC addresses as current subscriber observations.
Build active subscriber output from a fresh, complete, successful collection for the relevant source.
Do not fill absent current fields from a previous session or cached subscriber row.
Do not infer identity or transfer policy from a match against a stored IP address or MAC address.
Retain approved operator policy separately. It does not supply current session data.
The source-specific identity and reconciliation algorithm still require a concrete implementation design.
If identity is ambiguous, expose the conflict instead of silently merging subscriber records.

When a complete, successful source collection confirms absence, remove that subscriber from the active exports.
Remove its rows from `ShapedDevices.csv` and its subscriber presence from the generated network.
Remove subscriber-only generated nodes that no longer represent an active subscriber.
Retain durable identity, saved assignment, and pins in the internal store, separate from active exports.
Offline identity records must not keep stale IP or MAC mappings active.
The presence policy for empty shared groups and operator-created nodes remains decision 9.

On a confirmed return, reconcile the durable identity and collect current subscriber data.
The reconciliation must use current source identity evidence, not a stored IP or MAC match.
Write the subscriber to the active exports with fresh IP addresses, MAC, and current session attributes.
Restore its saved placement and pin if they remain valid.
Do not restore obsolete addresses from its previous session.
If the saved placement is no longer valid, expose the conflict and apply the separately approved fallback policy.

A failed or incomplete router read does not confirm subscriber absence.
Mark observations as stale or unknown. Do not interpret a read failure as an empty successful result.
Do not process the previous snapshot as fresh data when a collection fails.
This distinction preserves decision 5 and prevents accidental bulk removal during collector failures.
The exact stale-data policy and source-specific presence rules remain decision 9.

Recompute affected aggregate capacities from the current active membership.
Count only active exported members in the active plan demand and computed capacities.
Preserve a valid subscriber's placement across reconnects rather than treating every return as a new identity.

Required implementation checks include:

- Confirmed disconnect removes all active rows, subscriber nodes, and obsolete address mappings.
- Return with a new IP or MAC restores one identity with fresh data and a valid saved pin.
- Stored IP and MAC values never supply missing current values or reconstruct an active session.
- An address match against history cannot merge identities or transfer policy.
- A failed collection cannot republish stored subscriber observations as a fresh result.
- Repeated collections and repeated reconnect events do not create duplicate rows or placements.
- Failed or incomplete collection does not trigger the confirmed-absence removal path.
- Export and apply status distinguish file publication from verified LibreQoS acceptance.

## Decision 7 — Custom-node capacity policy and defaults

State: approved by the owner on 2026-11-16, Asia/Shanghai.

A custom group starts in automatic capacity mode.
When it directly contains subscribers, compute its capacity from those active subscribers with the approved group factors.
The owner's later decision 8 clarification restricts subscriber-based calculation to the immediate parent of subscribers.
An intermediate node without direct subscribers does not automatically recalculate all descendant subscriber plans.
Allow the operator to select manual mode for a node that represents a known aggregate limit.
Manual mode requires explicit download and upload base capacities.
Apply that node's factors to those base capacities, not to subscriber plan limits.

Keep WAN nodes in manual mode and source-backed profile/group nodes in their agreed automatic mode.
Automatic mode for a custom group does not make it a WAN or a service profile.
A move, promotion, or demotion must preserve the node's capacity mode and factor overrides.
Show a custom node mode change in the preview and automatically process the completed configuration edit.
The owner's decision 11 clarification removes the separate explicit Apply gate.

Use configured shared default factors where a source definition exists.
For new standalone custom nodes without configured defaults, use factors of 1.0 in both directions.
An explicit per-instance override takes precedence, as decision 1 requires.
Preserve current capacities, factors, and computed results when existing settings migrate.
Do not replace migrated values with new-node defaults.

Display the mode, base or computed input, factor origin, and resulting directional capacities separately.
Validate finite positive inputs and supported export bounds before application.
Do not silently reinterpret a manual capacity as measured traffic or interface speed.
Factor application across nested groups, parent constraints, and empty-group export behavior remain decisions 8 through 10.

## Decision 8 — Nested capacity calculations

State: approved by the owner on 2026-11-16, Asia/Shanghai, with the ceiling and subscriber-speed requirements below.
The owner also approved the later immediate-parent clarification before requesting decision 9.

Keep raw subscriber demand, local node capacity, and the ancestor-limited ceiling as separate values.
The owner clarified that subscriber-based automatic calculation belongs to the immediate parent of subscribers.
For a custom automatic group with direct subscribers, sum current plan rates for its unique active direct subscribers.
Apply that group's effective factors to its own calculation basis once.
Do not repeat that subscriber-based calculation at every ancestor.
Do not calculate another subscriber-based ancestor rate from already factored child capacities.
This clarification supersedes the earlier example that recalculated all descendant plans at each ancestor.
Supplemental detail 1 defines intermediate capacity policy and excludes mixed subscriber/group children in the first version.
Reject mixed children rather than silently ignoring subscriber or branch demand.

For source-backed groups, retain the current source-specific calculation basis.
The inspected implementation uses plan sums for PPP and per-plan groups.
Hotspot and DHCP site groups use active count multiplied by the configured unit rate.
Retain configured backstop caps and supported rate floors where applicable.
Use fresh membership and current approved policy inputs, without prior-session address fallback.
Tests must prove parity for unchanged current groups before migration.

For a manual node, use its configured directional base capacities and apply its factors once.
An instance override replaces the inherited factor. It does not multiply the inherited factor again.
Compute download and upload independently.
Preserve full precision during calculation and perform the required output conversion at the export boundary.
Do not repeatedly round a value as it passes through ancestor levels.

For aggregate nodes, the maximum export ceiling is the lesser of the local capacity and the applicable ancestor ceilings.
Retain and display the configured or computed local value separately from that constrained ceiling.
For example, show a local 800 Mbps capacity and a 500 Mbps ancestor-limited ceiling distinctly.
Do not divide parent capacity equally among children or require the sum of child ceilings to fit the parent.
Show aggregate oversubscription separately. A ceiling is not a guaranteed simultaneous rate.
If the ceiling cannot satisfy supported minimums, reject the candidate rather than silently increasing it above its parent.
Minimum-rate allocation and invalid-capacity handling need explicit checks before export.

Example: a configured 310 Mbps WAN contains one automatic group and ten direct subscriber children of that group.
Each subscriber has a 50 Mbps plan, and the group factor is 0.8.
The immediate group calculates 500 times 0.8, or 400 Mbps, before the ancestor ceiling.
Its exported ceiling is at most 310 Mbps. Each subscriber keeps its own 50 Mbps maximum.
The WAN remains explicitly configured. It does not auto-compute from the subscriber sum.
If subscribers are directly below a WAN, retain the WAN's manual capacity policy rather than auto-computing it.

For balancing, prefer independently configured destination capacities, as decision 4 requires.
If no independent capacities distinguish equivalent eligible destinations, use equal target shares of assigned plan demand.
This is equal plan demand, not necessarily equal subscriber count.
Do not derive allocation weights from the computed group output itself.
Child factors control their resulting limits and do not silently become allocation weights.
Automatic capacity calculation follows placement rather than defining its own input membership.

Empty-node presence and output defaults remain decisions 9 and 10.
These calculations require isolated source and export tests before production use.

### WAN ceiling and independent subscriber speeds

The owner explicitly requires aggregate nodes to stay within their applicable WAN ceilings.
WAN capacity is explicitly configured from usable upstream capacity. Subscriber demand and Ethernet link speed do not determine it.
For a WAN with an effective 310 Mbps capacity, an aggregate node below it must not export a ceiling above 310 Mbps.
The WAN aggregate limit also constrains the combined traffic of its descendants.
Retain the unconstrained calculated demand for display and oversubscription analysis only.
Do not export that demand as a node ceiling above its WAN limit.

Individual subscriber speeds remain independent of aggregate node factors and ancestor ceiling calculations.
A subscriber with a 50 Mbps maximum retains that maximum unless an explicit subscriber speed policy changes it.
Do not divide the WAN capacity among subscribers and rewrite their plan maxima.
Their aggregate traffic still shares the WAN capacity. Plan maxima do not guarantee simultaneous full throughput.
Implementation tests must prove both aggregate bounds and unchanged individual plan maxima.

### Visible subscriber maximum and minimum policies

The owner requires explicit settings for subscriber speed limits and their factors.
Keep these controls distinct from WAN and group capacity factors.
Expose the current base download/upload speeds, maximum-speed factors, and resulting maximum speeds.
A factor of 1.0 preserves the corresponding base maximum.
Only the subscriber maximum-speed policy changes the subscriber maximum.

Provide a selectable minimum-speed mode:

- Fixed Mbps, such as 1 Mbps.
- Percentage of the current read base speed, such as 1 percent.

For a 50 Mbps read base, the two examples produce minima of 1 Mbps and 0.5 Mbps respectively.
The percentage basis is the current read base speed, not a group capacity or an ancestor ceiling.
Do not silently apply it to the factored maximum instead.
Evaluate download and upload from their respective current base speeds.
Preserve approved minimum policy settings while session observations remain fresh-only.

Show the selected mode, configured value, calculation basis, and resulting minimum and maximum.
Validate supported rates and `minimum <= maximum` before export.
If a requested value is unsupported, show a validation error instead of silently replacing the requested policy.
Exact UI location, policy inheritance, and initial minimum defaults remain open details.
Do not assume 1 Mbps or 1 percent is the selected default. The owner supplied both as selectable examples.

## Decision 9 — Discovery changes and reconciliation

State: approved by the owner on 2026-11-16, Asia/Shanghai, after the DHCP status and lease-expiry clarifications.

Apply the approved fresh-data and confirmed-absence rules from decision 6.
Distinguish a complete successful collection from a failed, partial, or inconsistent collection.
Use source-specific active-presence evidence. A retained configuration or lease record alone does not universally prove an active session.
For RouterOS DHCPv4, use the current lease status `bound` as the active-lease criterion for static and dynamic leases.
Other statuses, including `waiting`, are not active bound leases. RouterOS does not require a literal `unbound` status.
Read current lease observations and active address/MAC fields. Do not substitute historical session fields.
Validate current required attributes before export.
Static lease configuration alone does not qualify a subscriber for active output.
The bound criterion describes lease state, not immediate device reachability.
A disconnected device can retain a bound lease until the router updates or expires it.
Immediate physical-offline detection would require separate evidence and is not promised by this criterion.
Reference: [RouterOS DHCP Server](https://manual.mikrotik.com/docs/network-management/dhcp/server/).
PPP and Hotspot presence rules still require source-specific design and tests.
Dynamic lease expiry can return an address to the pool. A subsequent binding can use the same or a different address.
A fixed-address static lease normally retains its configured address on return.
Do not infer the current address from expiry, renewal, or historical assignment. Read the current binding.

After a complete successful collection confirms absence, remove the subscriber from the next active export.
Remove obsolete session mappings and subscriber-only generated nodes.
Retain operator-owned assignments and pins separately from active observations.
On a confirmed return, use fresh source identity and session data only.
Restore a saved assignment if valid. An ambiguous identity or invalid saved placement becomes a visible conflict.

On failed or partial collection, mark the affected source as stale or unknown.
Do not replace absent observations with stored IP, MAC, or subscriber rows.
Do not treat the incomplete result as proof of an empty source.
For the first version, hold the new combined export until all required subscriber sources have valid fresh input.
Leave the previous accepted output unchanged and report the collection/publication hold explicitly.
An unchanged previous output is not a new successful collection or fresh publication.
The previous native world can retain old mappings during this hold. Do not claim immediate disconnect removal while publication is held.
Recovery must collect fresh data before publication resumes.

Keep a discovered object's stable identity through a rename when the source proves continuity.
Do not infer continuity from a matching display name, old IP, or old MAC.
If the source cannot prove continuity, require explicit reconciliation instead of an automatic merge.
Keep explicitly created topology instance IDs stable independently of display names.

If a selected source node disappears from a complete snapshot, mark its definition as missing in the inventory.
Do not silently delete the approved hierarchy or substitute another source.
Do not use the missing source definition to fabricate current subscriber observations.
Require explicit resolution for affected assignments that no longer have a valid destination.

Keep operator-selected WAN/manual nodes in the configured topology, including when they have no active subscribers.
For an empty automatic group, retain its definition and placements in the UI but omit its empty branch from active export.
Do not invent a zero or floor-based capacity to represent the absent subscribers.
This empty-branch rule applies only if the branch has no active descendants.
Do not prune an intermediate node that still carries an active child branch.
When eligible subscribers return, recreate the exported automatic group with its existing instance ID and fresh computed capacity.

Reject stale or conflicting revisions before an edit overwrites newer approved topology.
Expose renames, source reconciliation, and structural changes in the preview without a separate manual Apply gate.
Unresolved critical identity or reference conflicts still require operator resolution.
Normal fresh subscriber appearance and confirmed disappearance follow the approved automatic lifecycle.
Fallback for unresolved subscriber placement remains decision 10.

## Decision 10 — Unassigned subscribers and insufficient capacity

State: approved by the owner on 2026-11-16, Asia/Shanghai.

Distinguish assigned plan oversubscription from invalid topology or impossible minimum commitments.
Subscriber maximum-rate totals can exceed WAN capacity, as decision 8 permits.
Retain individual subscriber maxima. Constrain aggregate node ceilings to the effective WAN and ancestor ceilings.
Show directional plan-demand ratios and oversubscription warnings without silently increasing WAN capacity.
Do not treat maximum-rate totals above WAN capacity as a reason to remove otherwise valid subscribers.
Use the approved allocation policy among eligible destinations, without bypassing pins or restrictions.

Validate explicit minimum-rate commitments against the applicable aggregate constraints.
If a candidate cannot represent those commitments, reject it and identify the conflicting policy.
Do not silently reduce subscriber minima, change maximums, or increase parent ceilings to make the candidate pass.
Concrete minimum-feasibility checks require verification against the supported LibreQoS contract.

Show a fresh subscriber without any valid destination in an Unassigned list with a reason.
Examples include no eligible node, conflicting restrictions, or an unresolved invalid saved pin.
Use strict publication for the first version: hold a candidate with unresolved required placements.
Keep the previous accepted output unchanged and clearly show that publication is held.
Do not republish historical subscriber observations as fresh data during the hold.
This publication hold does not block packet forwarding or guarantee shaping of a new subscriber.

Do not silently assign an unplaced subscriber to the synthetic root, an arbitrary WAN, or a generated fallback node.
The operator resolves the assignment or explicitly configures a valid fallback destination before application.
A fallback must have an approved capacity policy and must not silently bypass eligibility or pins.
Its subscriber observations must still be fresh.
Do not add a fallback destination automatically in the initial design.

Preview the conflicting subscribers and distinguish oversubscription warnings from blocking validation errors.
When publication can resume, collect and validate fresh source data again.
Apply recovery and verification details remain decision 11.

## Decision 11 — Automatic publication and verification

State: approved by the owner on 2026-11-16, Asia/Shanghai, after the automatic-lifecycle clarification.
The previous proposal for a mandatory operator Validate, preview approval, and Apply sequence is superseded.
Detailed recovery behavior remains proposed where the owner has not yet decided it.

Keep configured policy revisions, published inputs, and verified applied state distinct.
Automatically process completed policy changes and successful fresh subscriber collections.
Automatically compute placements and capacities, check integrity, write inputs, request the supported apply path, and verify the DPDK dataplane.
Do not require separate operator Validate or Apply actions, or approval of each normal publication attempt.
Preview is an optional view of pending and completed changes. It is not an approval gate.
Do not automatically activate incomplete keystrokes or an invalid partially edited configuration.
The concrete edit-completion and batching mechanism still needs UI design.

In the preview, show created, removed, renamed, and moved instances.
Show affected subscribers, pins, assignments, directional capacities, factors, and min/max policies.
Show validation errors separately from oversubscription warnings.
Run checks automatically before publication and stop an invalid candidate from reaching the dataplane.
Operator intervention is for unresolved critical conflicts, not routine publication or nonblocking warnings.
Failed required collection still holds publication automatically under decision 9. Resume after successful fresh collection without manual approval.

Assign persistent revision identities to policy edits and correlate each export attempt with its input observations.
Check the edit's base revision before persistence. Reject stale edits instead of overwriting a newer revision.
Serialize publication attempts, including automatic collection and operator edits.
Refresh subscriber observations before publication. Do not revive historical IP/MAC values.
If observations change during computation, recalculate from a coherent fresh snapshot and refresh the preview automatically.

Check the topology, identities, source references, memberships, factors, rates, and pin constraints automatically before export.
These internal checks satisfy the approved integrity requirements. They are not a manual validation workflow.
Persist policy revisions through a recoverable transaction.
Stage and validate output files before replacing their production counterparts through the existing writer path.
The CSV/JSON pair is not one atomic filesystem transaction. Do not describe independent renames as atomic pair publication.
The implementation must define recoverable write ordering and interrupted-publication behavior and test the importer interaction in isolation.
Persist enough attempt state to reconcile an interrupted write without inventing a successful application.

Expose at least these states:

- Pending changes or automatic computation.
- Automatic integrity checks and publication in progress.
- Published inputs, awaiting import.
- Imported, awaiting native apply verification.
- Verified applied.
- Held, rejected, failed, or verification unavailable, with an explicit reason.

Do not mark a revision applied merely because files exist, a request succeeded, or topology reports `ready: true`.
Use correlated import and native acknowledgement evidence for the intended world.
If that evidence is unavailable, report verification unavailable instead of success.
Retry supported transient failures automatically with bounded attempts. Escalate unresolved critical failures with their reason.
If publication or application fails, preserve the previous known-good state where the contract permits it and expose recovery status.
Do not promise full dataplane rollback from a file restore alone.
Recovery must use a bounded attempt policy, not an endless retry loop or repeated full reloads.
Retain the latest attempt and known-good revision so a restart can reconcile their states before another write.

Rollback selects a previous approved topology/policy revision and creates a new application attempt.
Combine that policy with fresh current subscriber observations and current eligibility checks.
Do not replay the previous revision's historical subscriber IP/MAC rows.
If the previous policy cannot represent current subscribers, show the conflict and hold publication until it has a valid resolution.
Verify rollback through the same import and native-apply checks as a forward change.

Record the revision, operator or automatic action, timestamp, validation result, and publication/apply outcome.
Keep credentials and unnecessary subscriber session data out of the audit record.
Tests must cover stale edits, concurrent collection/edit attempts, partial file writes, restart recovery, failed import, missing acknowledgement, and rollback with changed addresses.
The approved lifecycle includes bounded automatic retries and automatic resume after valid fresh collection.
Intervention is for unresolved critical conflicts. Preview does not gate publication.
Detailed recovery parameters, rollback triggers, and partial-write mechanisms still need implementation design or a decision.
None of these statements claims implemented reliability or verified dataplane behavior.

## Supplemental implementation decisions

Discuss one detail per round and record each approval.

| Detail | Topic | State |
|---|---|---|
| 1 | Intermediate capacity and mixed children | Approved, including WAN-percentage capacity |
| 2 | Subscriber speed controls, inheritance, and defaults | Approved, including selectors, defaults, UI location, and inheritance |
| 3 | Source-specific subscriber identity and presence | Approved registry and identity model; concrete adapters require proof |
| 4 | Edit completion, publication recovery, and DPDK correlation | Approved behavior; implementation proof required |

### Detail 1 — Intermediate capacity

State: approved by the owner on 2026-11-16, Asia/Shanghai.
The owner adds explicit percentage-based capacity with WAN capacity as the basis.

An intermediate node contains group nodes rather than direct subscribers.
Its default capacity follows its effective parent ceiling.
It does not sum subscriber plans again or apply the immediate subscriber group's factors.
Allow an explicit fixed directional capacity when the node represents a lower aggregate limit.
Also allow a directional percentage of the applicable WAN's configured capacity.
Percentage-based capacity is a distinct explicit mode, not a multiplier on the fixed-Mbps mode.
Do not base this percentage on subscriber totals or silently substitute the immediate parent's capacity.
Compute download and upload from their respective configured WAN capacities.
Constrain the result to all applicable effective ancestor ceilings, including the WAN shaping ceiling.
Show the mode, WAN reference, configured input, calculated target, and constrained result.

Example: WAN capacity is 310 Mbps. An intermediate node at 50 percent targets 155 Mbps.
If an intervening parent has a 100 Mbps ceiling, the resulting node ceiling is at most 100 Mbps.
Fixed capacity remains fixed when WAN capacity changes, subject to ancestor constraints.
Inherited and WAN-percentage capacities automatically follow changes to their respective reference values.
For WAN-percentage mode, require an unambiguous WAN reference before publication.
Do not invent a WAN capacity for topology without a configured WAN.

For the first version, reject nodes with both direct subscribers and group children.
Place those subscribers in an explicit automatic group under the intermediate node.
The immediate group performs subscriber-based calculation under decision 8.
The WAN retains its explicitly configured capacity when subscribers attach directly to it.

### Detail 2 — Subscriber speed policy controls

State: the owner approves Mbps/Percentage selectors, detail 2a defaults, and detail 2b control location and inheritance.
The owner requires a selectable percentage-based or Mbps-based speed control.
Fixed Mbps and percentage of current read base speed are selectable minimum modes, as decision 8 specifies.
Maximum speed also supports fixed Mbps or a percentage of the current read base speed.
Minimum and maximum have independent modes and directional values.
In percentage mode, 100 percent preserves the base maximum. In fixed mode, use the configured Mbps value.
Show the unit, current base speed, and calculated Mbps result.
A mode change must not silently reinterpret the old numeric value in the new unit.
Require a valid value in the selected mode before the automatic publication path processes a completed edit.
The approved initial minimum for new policies is fixed 1 Mbps in each direction.

Expose a Subscriber Speed Policy section in the Topology Manager.
Provide global defaults and optional overrides on the shared source profile or service definition.
Profile overrides apply across its WAN instances. Node placement alone does not alter subscriber policy.
Show inherited versus overridden values and resolved download/upload results.
Resolve an explicit source-profile override before the global default.
Do not multiply an inherited factor by its overriding factor.

For new policy defaults, use maximum Percentage mode at 100 percent in both directions.
Use fixed 1 Mbps as the initial minimum mode/value, with the approved base-speed percentage mode available.
Retain current effective settings on migration instead of replacing them with new-policy defaults.
Validate unsupported rates and minimum-above-maximum as conflicts rather than silently changing the values.
Do not add individual-subscriber speed overrides unless separately requested.

#### Detail 2a — New-policy speed defaults

State: approved by the owner on 2026-11-16, Asia/Shanghai.

For a new policy, use maximum mode Percentage with a value of 100 percent for download and upload.
This preserves each current read base maximum unless the operator selects another mode or value.
Use minimum mode Fixed Mbps with a value of 1 Mbps for download and upload.
The operator can switch either minimum or maximum independently to Percentage or Fixed Mbps.
For percentage minimums, use the current read base speed, as decision 8 requires.
Keep mode and value visible. Do not silently switch units or alter an unsupported value.
Reject a minimum greater than its corresponding maximum and expose the conflicting policy.
Apply these defaults only to new policies. Preserve effective current settings during migration.
This round selects defaults only. The owner subsequently approved control location and inheritance under detail 2b.

#### Detail 2b — Control location and inheritance

State: approved by the owner on 2026-11-16, Asia/Shanghai.

Place global subscriber speed defaults in Topology Manager > Subscriber Speed Policy.
Place optional subscriber policy overrides on the shared profile or service definition in the same manager.
Do not duplicate these editable controls in the general Settings TUI or on each WAN instance.
General Settings retains router access, collection, paths, and notifications.

Resolve each of the four controls independently: download minimum, upload minimum, download maximum, and upload maximum.
Treat each control's mode and value as one override unit.
A control without an override inherits the current global mode and value.
A control with an override uses that explicit mode and value instead.
Do not multiply a global percentage by its overriding percentage.
An edit to global defaults affects inheriting controls. Explicit overrides retain their values.
Reset to inherited removes the override and uses the current global control automatically.

All WAN placements of the same shared source profile/service use that same subscriber policy.
Their group capacity factors remain separately configurable under decisions 1 and 8.
Do not add subscriber-speed overrides per WAN placement or individual subscriber in the initial version.
Show the inheritance origin, mode, value, current read base speed, and resolved Mbps values.
Automatically check and publish valid completed edits under decision 11.
Preserve current effective values during migration, including explicit overrides where necessary to retain parity.
Verify min/max consistency after inheritance resolution, not only when each field changes.

### Detail 3 — Subscriber identity and source presence

State: approved by the owner on 2026-11-16, Asia/Shanghai, after the subscriber-registry and CRM clarification.
The approval covers the identity/policy registry and fresh-only observations, not implementation of CRM features.

Assign each subscriber a persistent internal ID. Keep current session observations separate.
Bind that ID to a verified source account or authoritative subscriber reference, scoped to stable router/service identity.
Do not assume that session IDs, display names, IP addresses, or MAC addresses establish permanent identity.
Do not merge accounts from different routers merely because their usernames match.
Renames, router migrations, and reused account names require explicit continuity evidence or reconciliation.

For PPP, obtain current presence from `/ppp/active` and map the authenticated account to its internal subscriber ID.
For authenticated Hotspot users, use `/ip/hotspot/active` and the verified account/service reference.
Keep account-level subscriber identity distinct from individual sessions and devices.
If one account has several simultaneous sessions, retain each fresh endpoint without duplicating the subscriber's shared plan.
MAC-authentication and trial users do not automatically gain a stable account identity across MAC changes.

For DHCP, retain the approved current `status=bound` presence rule.
Use an operator-assigned subscriber reference or a verified unique source identifier to reconnect the internal identity.
Client identifiers or relay circuit identifiers are candidate evidence only when their stability and uniqueness are established for the deployment.
Never assume that a client ID, host name, lease entry ID, or physical port universally identifies one permanent subscriber.
If both IP and MAC change and no independent reliable reference exists, identity continuity cannot be inferred safely.
Expose an unresolved identity instead of assigning another subscriber's historical pin or policy.
Operator reconciliation is a critical-conflict path, not required for ordinary verified reconnects.

For every collection, use fresh current addresses and session attributes only.
Historical identity/policy records cannot supply missing current IP or MAC values.
Do not publish a candidate with unresolved required identity conflicts, consistent with decision 10.
Tests must include account reuse, duplicate names across routers, reconnect with changed addresses, and multiple sessions per account.

#### Owner question — Subscriber registry and a possible CRM

The owner asks whether a persistent Subscriber ID means retained subscriber information and could support a CRM.
A subscriber registry would retain identity references and operator-owned policy, such as profile assignments and pins.
Current IP, MAC, and active session observations remain fresh-only inputs for active exports.
Persistent identity does not authorize reuse of historical session observations.

Such a registry could provide an identity foundation for future customer and service management.
A CRM would require separate customer records, account relationships, service lifecycle, and other explicitly selected features.
Billing, payments, contact data, and support workflows are not implied by the topology registry.
Do not assume one customer equals one subscriber, one service account, one device, or one session.
The owner subsequently approved the identity registry direction and requested the next detail.
The possible CRM remains a future extension. No CRM features or implementation are authorized by the registry requirement.

References: [PPP AAA](https://help.mikrotik.com/docs/spaces/ROS/pages/132350049/PPP+AAA),
[Hotspot active users](https://help.mikrotik.com/docs/spaces/ROS/pages/56459266/HotSpot+-+Captive+portal),
and [DHCP](https://help.mikrotik.com/docs/spaces/ROS/pages/24805500/DHCP).

### Detail 4 — Automatic publication triggers and recovery

State: approved by the owner on 2026-11-16, Asia/Shanghai.
Approval defines required behavior. It does not prove current writer atomicity, native protocol compatibility, or implemented reliability.

#### Completed edits and collection triggers

Trigger the automatic pipeline after a completed, internally valid field/form edit or a successful fresh collection changes effective output.
Do not publish each partial keystroke or an incomplete form.
Completed edit detection and short batching belong to the UI/controller, without a separate Validate/Apply confirmation.
Coalesce related changes into one coherent policy revision and one export attempt.
Do not rewrite or reapply unchanged output merely because another collection completed.
If session observations are unchanged but missing or conflicting native evidence requires reconciliation, perform verification separately.

#### Publication serialization and interruption

Use one publication owner. Queue newer desired revisions instead of overlapping writers.
Before publication, calculate from a coherent policy revision and fresh complete observations for all required sources.
Stage and check the pair before ordered replacement.
Record attempt metadata and completed phases so a restart can identify an interrupted operation.
Do not assume the two files become visible atomically or that the importer never observes an intermediate pair.
Prove safe importer interaction and recovery with isolated interruption tests before enabling this path.

On restart or partial publication, reconcile disk, import, and native state before another mutation.
Recollect subscribers and generate a coherent candidate from the intended valid policy.
Do not recover by blindly restoring old subscriber IP/MAC rows from an archived export.
If fresh required inputs are unavailable, hold the new attempt and report its exact phase.
Retain the native last-known-good world where LibreQoS supports it. Do not claim that this makes historical observations fresh.

#### DPDK application evidence and retries

Correlate the policy revision, source snapshot, exported content, LibreQoS import, and intended native world.
Match actual native acknowledgement/state to that intended world before showing Applied.
Where supported, verify affected mappings, hierarchy, and rates rather than relying on traffic counters alone.
Neither a successful file write, import readiness, nor an unrelated generation value proves the intended dataplane state.
Inspect the exact installed acknowledgement contract before selecting correlation fields.
If reliable evidence is unavailable, show Verification unavailable or Pending with a reason.

For delayed acknowledgement, retry read-only verification first.
Do not resubmit the full world on every poll or treat a timeout alone as proof that application failed.
Retry mutation only through the supported idempotent or reconciled contract.
Use bounded attempts and a cooldown. Exact values require current-contract tests and remain implementation details.
Automatically resume after transient conditions clear and fresh required data passes internal checks.
Escalate unresolved critical conflicts without fabricating success or continually rewriting files.

Keep policy revision and attempt history for diagnosis. Do not create another long-lived subscriber-session cache for recovery.
Recovery and rollback must preserve the fresh-only observation rule from decisions 6 and 11.
No source pair, native socket, service, or router is mutated by this design discussion.

## Implementation work and proof still required

The eleven decisions and supplemental details define approved behavior. They do not prove an implementation.
Intermediate aggregate modes and the first-version mixed-child restriction are approved in supplemental detail 1.
Subscriber min/max control location, inheritance, selectors, and new-policy defaults are approved in supplemental detail 2.
Source-specific identity reconciliation and PPP/Hotspot presence rules need concrete evidence and tests.
The installed LibreQoS export/apply contract and interrupted pair publication need isolated proof.
The data model, UI edit-completion mechanics, and bounded recovery parameters need a concrete implementation design.
Do not claim production reliability or compatibility until relevant checks pass.

## Approved extension — Recurring speed schedule profiles

State: the owner approves the recurring schedule feature on 2026-11-16, Asia/Shanghai.
Approval includes multiple schedule profiles and configurable timezone selection with an Asia/Manila default.
The owner also approves both subscriber policies and WAN/group capacities as schedule targets.
Conflicting-rule resolution still needs a decision.

The owner asks for selectable days from Sunday through Saturday, configured times, and Percentage or Mbps speed settings.
Provide enabled/disabled schedule rules with weekday checkboxes and explicit start/end times.
Allow multiple named schedule profiles, each with its own enabled state, weekday set, time range, and speed settings.
Use Schedule Profile for a time rule, distinct from the subscriber's service/speed profile.
Provide a searchable timezone dropdown for each schedule profile and default new profiles to `Asia/Manila`.
Store an explicit timezone identifier and interpret that profile's weekdays and times in the selected zone.
Show the schedule timezone explicitly. Do not rely on an invisible host timezone.
The Asia/Manila application default does not change the Asia/Shanghai timezone used for this project's discussion timestamps.
Provide Percentage or Fixed Mbps mode for each scheduled speed control within the selected scope.
Support subscriber min/max policies and WAN/group capacities as explicit schedule targets.
A scheduled WAN limit must not exceed its configured true capacity.
A scheduled group limit remains subject to every ancestor ceiling.

Place schedule controls under Topology Manager > Policies > Speed Schedules, as shown in the approved concept.
Do not add another primary navigation destination for this capability.
Show current scheduled/base policy and the next transition in the relevant details/status views.

Outside an active interval, resolve the current base policy rather than replaying an old subscriber snapshot.
For percentage schedules, calculate from the current approved base reference, not a previously scheduled result.
Prevent cumulative percentage application across repeated scheduler ticks.
Keep fresh-only subscriber observations, eligibility, pins, and aggregate ceiling rules intact.
Automatic schedule transitions use the same internally checked publication and DPDK verification pipeline.
Recompute affected automatic group capacities if scheduled subscriber limits affect their approved computation basis.

Timezone configurability and its default are approved. Midnight-crossing day semantics, overlaps, inheritance, restart catch-up, and DST edge behavior still need design.
An offline subscriber's return must use the policy active at the current time, not its last session's scheduled policy.
The owner has not selected schedule precedence yet.
This extension does not authorize router routing changes, live scheduler activation, or a production deployment.

## Implementation state

The pure rate and hierarchy foundation now implements part of these requirements in isolation.
The foundation has 31 new tests. The registry/observation slice adds 35 more.
The catalog/schema slice adds 23 checks and persists catalog plus subscriber intent in one checked revision.
The placement slice adds fresh profile/rate evidence, eligibility, stable assignments, and weighted allocation for new circuits.
It adds 31 checks. At that stage, the full suite passed with 824 passed, zero failed, and 9 ignored.
It keeps one placement per circuit, preserves valid pins, and withholds the candidate on unresolved placement conflicts.
The aggregate slice computes direct source formulas, backstops/floors, ancestor ceilings, and summed minimum conflicts.
It adds 35 checks. At that stage, the full suite passed with 859 passed, zero failed, and 9 ignored.
It keeps subscriber maxima and treats plan oversubscription as a warning. Mathematical candidate checks do not prove native admission.
Explicit scoped rebalance now uses one bounded sweep with fixed pins, current eligibility, and aggregate checks.
It adds 25 checks. The full suite passes with 884 passed, zero failed, and 9 ignored.
Catalog ID allocation and the role-based read boundary now exist in isolation.
They add 19 checks. The full suite passes with 903 passed, zero failed, and 9 ignored.
Current protocol normalization now separates local-account evidence from endpoint-only evidence.
It adds 24 checks. The full suite passes with 927 passed, zero failed, and 9 ignored.
Opt-in independent reference authorization now connects fresh current data to registry reconciliation.
Source service/profile mapping and exact sustained rate input now extend that isolated path to existing policy evaluation.
It adds 23 checks. The full suite passes with 973 passed, zero failed, and 9 ignored.
It adds 23 checks. The full suite passes with 950 passed, zero failed, and 9 ignored.
Live authority configuration/enrollment, migration, export/native integration, and TUI remain unfinished.
The registry persists identity/policy intent and consumes current complete observations separately.
No live collector, export writer, TUI, or scheduler calls these modules.
Requirements approval is complete for the recorded behavior scope. Full product integration remains unfinished.
The approved scheduler extension adds outstanding overlap and time-boundary decisions to that original behavior scope.

## Source clarification — Local RouterOS accounts

The owner identifies the available sources: PPP secrets, current DHCP hostname/MAC, and Hotspot active rows.
The deployment has no RADIUS service. Local reads do not require RADIUS credentials.
Use secrets for PPP account configuration and active rows for current presence.
Use bound DHCP leases with fresh active addresses. Hotspot presence requires current authenticated sessions.
The read boundary requests these inputs without passwords and without old configured DHCP endpoint fallbacks.

This clarification does not establish a permanent reference for ambiguous account replacement or DHCP endpoint changes.
Current hostname/MAC and RouterOS row locators cannot, by themselves, justify transfer of old subscriber intent.
Current protocol normalization now checks presence, local joins, endpoints, and freshness.
An explicit operator-managed reference contract now supports verified continuity through the current account/lease comment.
The marker is `[lqosync-id:REFERENCE]`. It is an opt-in implementation path, not a reinterpretation of ordinary comments.
The operator must assign independent references and never reuse one for another subscriber in that source scope.
Missing, malformed, duplicate, or ambiguous references remain critical conflicts.
This contract requires no RADIUS and performs no router writes. Live enrollment and authority settings still require integration.
Preserve the approved identity-conflict guard when that contract is unavailable or cannot be trusted.

## Implementation detail — Current source policy bindings

Source selections now carry explicit service-name selectors and current profile-name selectors with stable source profile references.
DHCP/Hotspot service selectors resolve from fresh server inventory. An unavailable service does not become an empty-source assertion.
These names are selection rules and do not authorize subscriber continuity.
Missing active mappings and malformed enabled rate input withhold source acceptance.
Rate reads have per-profile Disabled/RateLimit modes and an explicit unprofiled mode for direct lease inputs.
Missing rate text stays absent. SourceRead authority must have actual numeric evidence before placement succeeds.
The integer parser has no float rounding, speed clamp, or default rate.
Explicit local profiles remain effective when source rate reads are disabled.
Durable settings, legacy source-priority migration, UI, and live activation remain separate work.

## Implementation detail — Durable settings and policy recovery

Schema 3 stores explicit source settings with the catalog and subscriber registry.
Old schema-1/2 files load without a disk rewrite or invented authority. A checked commit upgrades the document.
Each changed commit requires an exact verified private backup before primary replacement.
History keeps 16 recent schema-3 backups and the backups before migration. Optional cleanup errors remain visible after restart.
Forward recovery restores catalog/source settings while it preserves current subscriber records, pins, and the ID counter.
Incompatible references block recovery. A corrupt primary does not trigger an automatic identity rewind from old backups.
The restart integration uses saved settings and fresh source data through the existing capacity pipeline.
Live legacy-policy parity migration, publication/native integration, enrollment, TUI, and schedules remain unfinished.

## Implementation detail — Isolated publication and parity

Fresh authorized manager output now reaches the current pair writer and receipt tracker through an isolated adapter.
The adapter preserves the source deadline, expected policy revision, node ceilings, and subscriber maxima.
Multi-WAN roots remain separate. Repeated labels use stable IDs in export keys.
CSV rates preserve integer-bps precision through the f32 import format or produce a conflict.
Optional receipt metadata binds the policy revision, observation generation, and registry digest to native evidence.
An older receipt has no invented topology context. Unchanged content keeps the matching receipt identity.
The shared tracker now requires durable intent before output writes. Optional notification history remains independent.
Legacy parity reports unresolved IDs, changed rows/network content, and unmanaged baseline rows without address-based identity inference.
Explicit legacy identity/label/ownership conversion, exact native admission, owner wiring, enrollment, TUI, and schedules remain unfinished.
No manager-mode activation or live DPDK apply test forms part of this isolated slice.

## Implementation detail — Explicit mappings and compiled admission

Snapshot-specific manifests now declare legacy/current row and node mappings with revision, generation, and content fingerprints.
Mappings change only declared identity/label fields for comparison. They do not infer continuity from endpoints or transfer registry ownership.
Unclassified and protected baseline rows remain explicit issues, including coincident export IDs.

The public manager publisher now requires checked canonical/compiled shaping artifacts.
It rejects lost rates, endpoints, nodes, ancestry, or unexpected movement/matching metadata.
Scratch compiler checks found 100.5 Mbps node input becomes 101 Mbps internally.
The adapter now applies the approved ceiling-safe rule: export node limits downward to whole Mbps and expose both calculated/exported values.
It rejects quantization below subscriber minimum commitments or configured automatic group floors.
Subscriber speeds remain unchanged. A 50.5 Mbps subscriber survives the safe integer-node scratch compile.

These checks remain isolated from the live runner. Durable legacy adoption, source-priority conversion, compiler/configuration binding, and runtime owner wiring remain unfinished.
No live DPDK apply test occurred.


## Durable legacy adoption implementation, 2026-11-17

The isolated implementation now stores explicit export aliases and protected static ownership in schema 4.
Schema-1/2/3 files load without a rewrite. A changed commit saves the exact previous bytes before the upgrade.
The history retains 16 recent schema-4 backups and all backups before migration.
Ownership reservations survive restart and offline subscribers. Managed endpoints always come from the current accepted source read.

Current authoritative static input can preserve independent static roots and their explicitly protected rows.
Missing input, conflicting IDs, ambiguous device aliases, and static rows without parents block publication.
Shared static branches below managed nodes remain gated until their capacity rules exist.

The new source converter preserves configured PPP, DHCP-server, and Hotspot base speed priorities.
Current source comments remain in memory. Source order and explicit defaults remain visible policy.
Address-based overrides and complete legacy configuration migration still require explicit policy and shadow parity.
The implementation does not activate manager mode, enroll live accounts, or change routing.


## Controlled runtime owner implementation, 2026-11-17

An explicit runner API now compiles and publishes a fresh manager pair with one instance lock and one receipt tracker.
The controlled child uses a private workspace, bounded inputs, kernel write/socket controls, and the original source deadline.
The compiler records its exact library/source-config identity and the effective private-config digest.
Native-loader migrations can change the private copy. They cannot change the requested pair, source, mode, or workspace paths.

The runtime saves writer ownership before its first manager write. The updated runner blocks legacy fallback after restart.
Older executables cannot enforce this marker. A downgrade requires an explicit migration procedure.
Compiler drift, stale observations, wrong directories, corrupt ownership, and unbound runtime publication block the write.

Scheduler import requests are automatic and correlate with the receipt ID.
The existing monitor still checks native acceptance. The service loop remains on its current path until the activation gates pass.
The installed-binding tests used temporary directories and did not apply a live native world.
