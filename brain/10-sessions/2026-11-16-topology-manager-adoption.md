---
type: session
created: 2026-11-16
tags: [topology, libreqos-v3, research]
status: requirements-approved
related: ["[[HANDOFF]]", "[[ADR-0005-topology-follows-v3]]", "[[libreqos-v3-ingress-facts]]", "[[lqosync-owned-profiles-and-wan-policy]]"]
---

# Topology Manager adoption research

Reviewed: 2026-11-16, Asia/Shanghai. Agent: Codex `/root`.
The owner wants to adopt the LibreQoS Topology Manager approach in LQoSync3.
Development checkout: `/home/pi/LQoSync3.0`, branch `lqosync3`, HEAD `0ad8b8d`.
The separate `/opt/LQoSync3.0` checkout has branch `lqosyncv3` and HEAD `0e12561`.

Read the graph, handoff, decisions, current code, installed UI, live topology metadata, and public source.
Public source reference: `9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d`.
Installed control and dataplane package version: `3.0.7.202611141944`.
The public source does not prove exact installed binary behavior.

Report: `docs/reports/2026-11-16-topology-manager-adoption.md`.
It records data contracts, command names, ownership, integration gaps, proposed scope, and acceptance checks.

The manual importer fixes nodes in place. The strict native JSON format also lacks editor metadata.
A source switch alone does not enable native branch moves.
A separate editor-state file does not override a populated canonical model in the inspected loader.
Generated state files remain LibreQoS-owned.

Proposed first design: LQoSync owns approved topology and edits, then exports through the current source pair.
Alternative: extend the ingress contract and reuse the native editor and override resolver.
The owner has not selected or approved an implementation design.
The earlier native-source rate-loss result needs a current-build retest before migration.
Applied-state proof remains separate from topology readiness and file acceptance.

No product code, live configuration, router object, package, or service changed.
No mutation request, restart, commit, push, or external message occurred.
Prior unrelated changes remain intact. The handoff records documentation checks.

## Requirements discussion

The owner clarified discovery, node selection, multiple placements, capacities, factors, settings ownership, and the separate routing scope.
The owner wants one open decision per discussion round and a durable record of the full discussion.
The record is `docs/design/TOPOLOGY_MANAGER_DISCUSSION.md`.
It distinguishes confirmed direction, recommendations, and pending decisions.
The owner approved decision 1: shared source definitions and default factors, separate instances, and optional per-instance factor overrides.
The owner explicitly requires reliability and integrity.
The discussion record adds identity, reference, inheritance, subscriber uniqueness, and failure-preservation requirements.
These requirements have no implementation proof yet.
The owner approved decision 2: explicit shared target sets, individual placements, and no duplicate instance for an overlapping source/parent pair.
A new WAN requires explicit inclusion in a shared target set.
The owner approved decision 3 after clarification: one shared profile can populate all selected WANs.
Restrictions are optional operator filters. They narrow destinations or exclude subscribers from the auto-assignment pool.
An operator still selects the pool and WAN target set explicitly.
The owner approved decision 4: capacity-weighted plan allocation with download and upload capacity checks.
The approval covers destinations with configured capacities. Computed-only group behavior remains decision 8.
The owner approved decision 5: automatic initial placement, stable routine collection, previewed edits, and explicit scoped rebalance.
Continuous background redistribution remains disabled for the first version.
The owner approved decision 6 and explicitly requires identity integrity across subscriber disconnects and returns.
Confirmed absent subscribers leave the active CSV and generated network. Internal identity and pins persist separately.
Returning subscribers require fresh IP, MAC, and session observations. Old addresses must not remain active.
The owner clarified that stored IP and MAC data must never serve as current data or a fallback.
Active exports require fresh source observations. Missing current fields must not inherit values from prior sessions.
Stored operator policy remains separate from current observations. Identity cannot rely on a historical address match.
Failed or partial collection does not prove absence. Exact source presence and stale-data rules remain decision 9.
The discussion records required lifecycle checks. These behaviors are not implemented or verified yet.
The owner explicitly approved the fresh-data clarification and requested the next decision.
The owner approved decision 7: automatic custom groups by default, optional manual mode, and factors of 1.0 without configured defaults.
Preserve current values on migration and capacity mode on hierarchy moves.
Read `src/topology.rs` to confirm plan-sum and count-times-unit bases, factors, backstop caps, and floors.
The owner approved decision 8, with explicit WAN bounds and independent subscriber speed requirements.
An aggregate node below a 310 Mbps WAN must not export above 310 Mbps. Subscriber 50 Mbps maxima remain unchanged.
Expose subscriber max factors and selectable minima: fixed Mbps or percentage of current read base speed.
For a 50 Mbps base, 1 Mbps fixed and 1 percent produce 1 Mbps and 0.5 Mbps minima respectively.
Keep subscriber factors distinct from node factors. Exact UI location, inheritance, and minimum defaults remain open.
The owner clarified that subscriber-based automatic calculation applies at the immediate parent of subscribers, not every ancestor.
Corrected decision 8 and its example. A WAN remains explicitly configured, including when subscribers attach directly to it.
Intermediate aggregate nodes and mixed subscriber/group children still require an explicit capacity policy.
The owner approved the immediate-parent clarification and requested the next decision.
The owner asked whether DHCP activity follows bound status, including static leases.
Official RouterOS documentation confirms static and dynamic leases can be bound.
Clarified DHCPv4 presence as current `status=bound`, with fresh current address/MAC attributes.
A bound lease does not prove immediate reachability. Device disconnect can precede lease-state change or expiration.
The owner then asked about lease expiry. Dynamic clients can receive the same or a different IP on return.
Static fixed-address leases normally retain the configured address. Always read the current binding.
The owner approved decision 9 and requested the next item.
It holds combined export on failed required collection, retains manual nodes, and omits empty automatic branches while preserving internal IDs.
The owner approved decision 10: allow visible plan-max oversubscription, reject invalid minimum commitments, and hold unresolved required placements.
Fallback requires explicit operator configuration. No automatic synthetic-root or arbitrary-WAN placement.
The owner corrects decision 11: writing and application are automatic, followed by DPDK apply verification.
Preview is optional visibility, not an approval gate. No separate operator Validate or Apply is required.
Keep internal automatic integrity checks to prevent invalid output. Intervene only for unresolved critical conflicts.
Revised earlier explicit-Apply wording in decisions 3, 5, 7, and 9 for consistency with this direction.
The owner explicitly approved the corrected decision 11 lifecycle.
All eleven behavior decisions now have recorded approval, subject to the later clarifications.
Automatic checks, write/apply, DPDK verification, bounded retries, and fresh-data recovery require no routine manual gate.
Detailed edit-completion, recovery parameters, and rollback triggers still need design or a decision.
The owner requests individual approval rounds for supplemental implementation details.
Supplemental detail 1 is approved: inherit the parent ceiling by default, permit explicit lower capacity, and reject mixed children in version 1.
The owner adds explicit capacity as a percentage of WAN capacity. Record fixed-Mbps and WAN-percentage modes separately.
Constrain either mode by effective ancestors without changing subscriber maxima.
Next: supplemental detail 2, subscriber speed controls, inheritance, and defaults. Its proposed values remain unapproved.
The owner requests a switch between percentage-based and Mbps-based speed settings.
Recorded the selectable units without treating the 1 Mbps default as approved.
The owner explicitly approves Mbps/Percentage selectors for both minimum and maximum speed and requests the next detail.
Minimum and maximum modes are independent. Initial minimum defaults remain unselected.
DHCP cannot guarantee continuity across changed IP/MAC without a separate reliable source reference.
The owner asks whether persistent subscriber identity could support a CRM.
Recorded the distinction between a durable identity/policy registry and fresh-only session observations.
A CRM is a possible future product direction, not approved scope.
The owner approved the subscriber registry and identity model, then requested the next detail.
Added supplemental detail 4 proposal: completed-edit triggers, serialized recoverable publication, and correlated DPDK evidence.
Recovery recollects fresh observations rather than restoring old session rows. Verification delay does not trigger repeated full-world writes.
The owner approved supplemental detail 4 and requested continuation.
Concrete identity adapters, pair-publication recovery, and acknowledgement correlation still need isolated proof.
The owner approved supplemental detail 2a: new-policy maximum 100 percent and minimum fixed 1 Mbps for download and upload.
Existing effective settings must remain unchanged during migration.
The owner approved supplemental detail 2b: controls in Topology Manager with global defaults and shared-profile/service overrides.
Each directional min/max control inherits or overrides its mode/value independently. No multiplication of inherited and overriding factors.
All eleven behavior decisions and four supplemental groups now have recorded approval.
Next: prepare the implementation data model, migration boundaries, task sequence, and focused proof plan from this record.
Exact source adapters, native acknowledgement correlation, and interrupted-publication recovery still require implementation evidence.
No implementation tests or runtime changes occurred.

## TUI layout question

The owner asks whether the manager is one tab/interface and where its features belong.
Read current primary navigation in `src/tui/app.rs` and sidebar behavior in `src/tui/shell.rs`.
Used UI/UX guidance for keyboard focus and panel navigation, adapted to the terminal.
Proposed one Topology route after Routers, with Builder, Assignments, Policies, and Changes views.
Builder uses inventory, tree, and details panels on wide terminals and a focused-panel switcher on compact terminals.
The publication strip remains visible. Preview never becomes a manual Apply gate.
Layout record: `docs/design/tui/TOPOLOGY_MANAGER_LAYOUT.md`.
The owner approves this initial layout and explicitly expects further improvements in later reviews.
Keep it as a baseline, not a frozen interface. No renderer or runtime implementation has occurred.

## Speed schedule request

The owner asks for recurring speed schedules with Sunday-to-Saturday checkboxes, time settings, and Percentage/Mbps modes.
The owner approves the recurring schedule concept and Policies > Speed Schedules location.
The owner also requests multiple schedule profiles and a configurable timezone dropdown, default `Asia/Manila`.
Schedule profiles are distinct from subscriber speed/service profiles.
The owner subsequently approves both subscriber policies and WAN/group capacities as targets.
Scheduled limits retain configured true WAN capacity and ancestor ceilings.
Timezone selection and its default are approved. Overnight day semantics, overlaps, inheritance, and restart behavior still require decisions.
Schedule transitions must use current policy and fresh observations, without restoring historical IP/MAC or compounding percentages.
No schedule implementation or production activation occurred.

## Product review follow-up

The owner requests review against the current product, design optimization, and STE for later implementation text.
The review is in [[2026-11-16-topology-product-review]].
It records migration differences, reuse boundaries, performance constraints, and an isolated foundation plan.
No source or production configuration changed during that review.
