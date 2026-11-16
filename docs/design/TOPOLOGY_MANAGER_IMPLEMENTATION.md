# Topology Manager implementation design

Status: proposed implementation architecture after review of the current product.
Approved behavior: [discussion record](TOPOLOGY_MANAGER_DISCUSSION.md).
Evidence and risks: [product review](../reports/2026-11-16-topology-product-review.md).
Interface: [initial TUI layout](tui/TOPOLOGY_MANAGER_LAYOUT.md).

## Scope and ownership

Build a local topology and policy engine inside LQoSync3.
LibreQoS remains the shaper. RouterOS remains the source of current network observations.
Routing, traffic classification, and CRM features remain outside this implementation.
WAN placement expresses configured topology, not verified packet routing.

Keep one daemon as the policy and publication owner.
The TUI submits completed revision-checked edits and reads immutable view data.
It does not write shaping files or call the native dataplane directly.
Preserve the current legacy path until manager-mode migration passes parity checks.

## Separate records

| Record | Lifetime | Contents |
|---|---|---|
| Source catalog | Durable identity and current availability | Stable source/router references and operator mappings |
| Subscriber registry | Durable policy | Internal IDs, verified account references, assignment, pin, and profile reference |
| Policy document | Durable revision | Instance IDs, parents, capacities, factors, eligibility, speed controls, and schedule definitions |
| Observation snapshot | Current collection only | Current active sessions, endpoints, completeness, and time |
| Resolved world | One coherent evaluation | Assignments, rates, conflicts, and canonical outputs |
| Publication receipt | Durable attempt | Policy revision, content digests, phase, and native evidence |

Do not store old subscriber rows in the registry as inputs for later exports.
A diagnostic snapshot may exist, but recovery must not treat it as a current source observation.
An imported CSV row is migration evidence, not proof of present identity or activity.

## Minimal module boundary

Use `src/topology_manager/` for the new pure model and resolver.
Keep `src/topology.rs` as the legacy builder during migration.

| Module | Responsibility |
|---|---|
| `model.rs` | Typed IDs/references, policy schema, source roles, instances, and speed controls |
| `store.rs` | Versioned private policy document, expected-revision edits, and durable replacement |
| `rates.rs` | Exact rate/factor resolution, inheritance, and ancestor ceilings |
| `resolve.rs` | Graph checks, eligibility, placement, pins, demand, and export names |
| `observation.rs` | Fresh complete source evidence and verified identity reconciliation |
| `schedule.rs` | Active rule resolution and next transition after the schedule contract is settled |

Add modules only when their task starts. Do not create empty scaffolding.
Reuse `writer/pair.rs`, `write_receipt.rs`, `import_evidence.rs`, and `apply_evidence/`.
Do not create another publication service, telemetry cache, or parallel verification loop.

## Persistent policy contract

Use one private versioned JSON document for the first implementation.
Use the current standard-library and serde stack. No database is required for this scope.
Include `schema_version`, `revision`, and stable reference tables.
Keep current state plus a bounded set of previous policy revisions.
Do not include current session addresses as persistent policy fields.

For an edit, check the expected revision before a change to the document.
Apply the edit to a candidate and check its references and graph.
Persist the candidate through the current synced private-file mechanism.
Advance the in-memory revision only after durable success.
Persist publication intent before output side effects.
Treat a policy-store failure as blocking. Treat optional history exhaustion separately.

Restart recovery loads policy and receipt metadata, then recollects sources.
It does not regenerate active rows from the previous CSV or diagnostic snapshot.
Never claim atomicity across the two source files and the native world.

## Pure evaluation pipeline

1. Accept a completed policy revision and a fresh complete snapshot.
2. Resolve verified subscriber identity and current endpoints.
3. Validate unique references, graph structure, and allowed parent/child kinds.
4. Resolve eligibility and preserve valid assignments and pins.
5. Allocate new eligible members using the approved balance rule.
6. Resolve subscriber min/max policies and the active schedule overlay.
7. Compute immediate group demand once per subscriber circuit.
8. Resolve WAN and intermediate limits and constrain aggregate ceilings.
9. Validate export names, address ownership, minimums, and unresolved placements.
10. Render canonical outputs and compare their content with current outputs.

Schedules target subscriber policies and WAN/group capacities.
Define their effect on allocation demand before enabling evaluation.
A schedule transition does not implicitly authorize a placement rebalance.
Do not silently move subscribers at schedule boundaries under the stable-assignment rule.
Rate changes can update current assignments without an automatic full rebalance.

Sort stable IDs when order affects output or balancing ties.
Keep observation times out of the shaping-content digest.
Use explicit source-scope keys when discovered labels repeat across routers.
Give repeated tree instances unique export names while preserving internal instance IDs.
Reject ambiguous address ownership before rendering.

## Rates and numeric integrity

Represent new core rates as checked integer bits per second.
Represent decimal factors as exact numerator/denominator values.
Use checked wider intermediates for multiplication.
Convert current `speed::Rate` values at an explicit compatibility boundary.
Reject lossy or unsupported migration conversions when parity fails.

Resolve each subscriber control's inherited or explicit mode/value first.
New policies default to max 100 percent and fixed minimum 1 Mbps, independently by direction.
Current migrated values stay unchanged unless the operator deliberately changes policy.
Do not clamp invalid minimums silently as the legacy rate function does.
Keep an explicit base-rate reference and authority for each policy.
The earlier project direction assigns local plan ownership to LQoSync. The current collector still reads RouterOS speed sources.
Reconcile these contracts before activation. Do not silently replace local plans through discovery.

An automatic group uses only its direct active subscribers and its approved source formula.
A WAN uses configured capacity. An intermediate node uses inherit, fixed Mbps, or WAN percentage.
Constrain aggregates after local computation. Never scale subscriber maxima down to fit a WAN.
Account for export precision before accepting a ceiling as valid.

## Collection and identity

Add optional WAN collection through explicit router roles.
Read only the tables needed by those roles and keep current session/concurrency limits.
Distinguish a required source failure from optional observability failure.
Capture source completeness only after the required read sequence completes.

Use PPP and authenticated Hotspot accounts as evidence for scoped registry mappings.
Keep simultaneous sessions as endpoints under one subscriber circuit.
DHCP uses current bound leases and a verified independent reference for durable continuity.
If continuity is unprovable after address changes, show a critical identity conflict.
Do not infer identity from a hostname or past address match.

Use current active endpoint fields when supplied by the source.
Any use of configured fields from the same successful read requires source-specific evidence that they describe the current bound session.
Never copy absent fields from a prior snapshot.

## Publication and native verification

The current receipt tracker remains the publication path for the managed pair.
Attach the policy revision and coherent observation identity to attempt metadata.
Unchanged shaping content keeps the current receipt identity unless native reconciliation is independently required.

Use one serialized writer and preserve network-first ordering until importer tests establish a safer supported contract.
Stage and check both outputs before replacement.
Record partial outcomes. Do not claim a staged pair is atomically visible.
Reconcile interruption before another mutation and collect fresh sources before a new candidate.

Reuse `Prepared` and `Verified` evidence from `apply_evidence::verify`.
The current verifier already checks file hashes, import content, source/shape generation, native stability, and the applied transaction.
Extend its metadata rather than weakening those checks.
Use read-only verification retries before another mutation.
Display configured, published, imported, applied, and stale states separately.

## Scheduler contract gate

The feature and timezone default `Asia/Manila` are approved.
The owner approves subscriber policies and WAN/group capacities as targets.
Scheduled WAN limits remain within configured true capacity.
Scheduled group limits remain within ancestor ceilings.
Overlap rules, overnight-day semantics, and DST edges remain open.
The review supplies recommendations, not owner decisions.
Do not ship enabled schedule evaluation with an invented precedence rule.

After those decisions, use one timezone-aware resolver and one next-boundary wakeup.
On restart, calculate the active policy now instead of replaying missed events.
Clock changes require re-evaluation. Timeouts use monotonic time.
No transition may use old endpoint data to satisfy a fresh collection requirement.

## Interface and migration

Keep the approved Topology entry with Builder, Assignments, Policies, and Changes views.
Keep Inventory, Tree, and Details distinct.
Build immutable view models only when their source revisions change.
Render visible rows and keep stable selection during refresh.
Completed valid edits trigger the daemon pipeline without a separate Apply gate.
General Settings retains its current protection for credentials and unrelated host configuration.

Import current node IDs, capacities, source formulas, and subscriber CIR values without changing live output.
Show unresolved name/identity mappings rather than silently deciding their ownership.
Compare shadow output against the current pair, including static/unmanaged row ownership.
Do not reclassify or remove protected static rows without an explicit migration rule.
Activate only through the current supported target. A source migration is separate.

## Required proof

Use fixture tests for identity changes, disconnects, multi-session accounts, collisions, inheritance, factors, schedules, and restart recovery.
Use synthetic scale fixtures for lookup counts, viewport work, and no-change publication.
Use current current focused tests as regression baselines.
Run live validation only after isolated pair/import/native-contract checks pass.
No live deployment is part of the review deliverable.

Write documentation, comments, errors, and UI text with the ASD-STE100 skill.
Preserve exact commands, Rust identifiers, file paths, and source quotations.
The heuristic checker does not certify STE compliance.
