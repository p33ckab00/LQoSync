# Topology Manager implementation design

Status: architecture after review of the current product.
The pure core, registry, catalog, placement/aggregate pipeline, and explicit scoped rebalance exist in isolation.
Catalog ID allocation and a bounded-age RouterOS reader also exist in isolation.
Verified source adapters, schedules, migration, export/native integration, and TUI remain unfinished.
See the [source boundary plan](../superpowers/plans/2026-11-16-topology-source-boundary.md) for the read contract and identity limits.
See the [rebalance plan](../superpowers/plans/2026-11-16-topology-rebalance.md) for explicit movement scope and proof.
See the [aggregate plan](../superpowers/plans/2026-11-16-topology-aggregates.md) for capacity and commitment checks.
See the [resolver plan](../superpowers/plans/2026-11-16-topology-resolver.md) for placement scope and proof.
See the [catalog plan](../superpowers/plans/2026-11-16-topology-catalog.md) for schema and validation proof.
See the [registry plan](../superpowers/plans/2026-11-16-topology-registry.md) for its scope and proof.
See the [foundation plan](../superpowers/plans/2026-11-16-topology-foundation.md) for checks and implementation rulings.
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
| `model.rs` | Registry identities, subscriber intent, and the schema-2 document |
| `catalog.rs` | Router roles, source definitions, local profiles, hierarchy inputs, and reference checks |
| `ids.rs` | Typed catalog ID batches from the shared durable counter |
| `reader.rs` | Role-based complete current RouterOS evidence with time and configuration checks |
| `store.rs` | Versioned private policy document, expected-revision edits, and durable replacement |
| `rates.rs` | Exact rate/factor resolution, inheritance, and ancestor ceilings |
| `resolver.rs` | Current membership, eligibility, subscriber rates, stable placements, and pins |
| `aggregate.rs` | Direct group basis, aggregate limits, descendant demand, and minimum conflicts |
| `rebalance.rs` | Explicit source-pool/branch moves with fixed pins and aggregate checks |
| `observation.rs` | Fresh complete source evidence and verified identity reconciliation |
| `schedule.rs` | Active rule resolution and next transition after the schedule contract is settled |

Add modules only when their task starts. Do not create empty scaffolding.
Reuse `writer/pair.rs`, `write_receipt.rs`, `import_evidence.rs`, and `apply_evidence/`.
Do not create another publication service, telemetry cache, or parallel verification loop.

## Persistent policy contract

Use one private versioned JSON document for the first implementation.
Use the current standard-library and serde stack. No database is required for this scope.
Include `schema_version`, `revision`, and stable reference tables.
The current private document uses schema 2 at `topology-manager/registry.json`.
It contains the registry plus an optional catalog. None means unconfigured, not an active default topology.
The loader reads schema 1 through a strict legacy struct and retains all IDs, references, counters, and intent.
Open does not rewrite schema 1. A revision-checked commit persists schema 2 and advances the revision.
Unknown versions, duplicate fields, and false downgrades are errors.
Before production migration, retain a verified private backup and prove the rollback path.
A configured catalog has no lossless schema-1 representation. Never drop it for a downgrade.
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

## Implemented registry boundary

`model.rs` stores verified scoped identity references and subscriber policy intent.
It does not contain endpoint fields. A configured catalog checks profile, assignment, pin, and source references in the same candidate revision.
It checks destination structure, configured potential source targets, and explicit local profile restrictions.
Actual source-profile membership and eligibility across current aliases still require fresh resolver evidence.
Ordinary edits cannot delete registry records or transfer existing identity ownership.

`observation.rs` accepts complete required-source reads under an advancing in-process generation.
A `CompleteSnapshot` has no serde implementation or Clone. Reconciliation consumes it.
Several current sessions or explicit aliases can form one subscriber circuit.
Complete empty reads produce no active endpoints but keep durable intent.
The source adapters must still prove activity and reference authority from current reader evidence.
The later adapter must check evidence age again after normalization and before it creates a complete source read.

## Implemented ID and router read boundary

`ids.rs` reserves typed router/source/profile/node IDs in a draft under an expected revision.
It shares `next_id` with subscriber allocation. Labels, IPs, and MACs do not contribute to generated IDs.
Only a successful store commit makes the reservation durable. Restart and deletion do not roll the counter back.
Existing custom IDs remain valid. Reserved generated prefixes require the correct kind, format, and counter range.

`reader.rs` derives required tables from catalog router roles and reuses `RouterSession`.
It requests no passwords. DHCP endpoint fields are `active-address` and `active-mac-address`, without configured endpoint fallbacks.
The reader requires current row locators, rejects duplicate locators and unrequested fields, and accepts empty complete replies.
It reads inventory before and after active tables. Inventory comparison ignores reply order and rejects changed requested values.
Each command has a timeout. The whole read has an age limit that starts before its first command.
Read generations advance even after command failure or cancellation. Successful evidence expires and has no Clone, serde, or Debug implementation.
The public read method returns the API session only after success. Failure or cancellation drops the session before retry.
Checks and indexed inventory comparison use linear row storage. Synthetic scale checks do not establish production throughput.

These reads do not form an atomic RouterOS snapshot. A session can disconnect after its active table reply.
The reader does not prove activity rules, stable account continuity, selected WAN membership, true WAN capacity, or routing.
WAN interfaces are discovery candidates. The operator still selects WAN definitions and configures their true capacities.
The reader has no live caller and does not create `SourceRead::Complete` by itself.

The owner identifies PPP secrets, current DHCP hostname/MAC, and Hotspot active rows as available sources.
There is no RADIUS service. Local account reads do not require one.
PPP secrets identify local account configuration. PPP active rows establish current sessions.
Hotspot active rows require authentication checks and correlation with the applicable account authority.
DHCP normalization must require `bound` and fresh active endpoint fields.
Hostname, current MAC, session locators, and RouterOS `.id` do not prove continuity after ambiguous reuse or replacement.
Do not transfer old pins or policies through those fields alone. Keep the unresolved binding visible as a critical conflict.
Protocol normalization and the durable account lifecycle contract remain the next implementation task.

Reference: [MikroTik PPP AAA](https://help.mikrotik.com/docs/spaces/ROS/pages/132350049/PPP%2BAAA),
[MikroTik DHCP](https://help.mikrotik.com/docs/spaces/ROS/pages/24805500/DHCP),
and [MikroTik API](https://manual.mikrotik.com/docs/developer-guides/api/).

`store.rs` reuses the instance lock and synced private-file writer.
A new private directory receives a durable empty registry at revision 0.
A missing file in an existing directory requires recovery; corrupt files never become empty registries.
Edits check the expected revision and disk document, then advance memory only after durable success.
A possible replacement failure or detected external change blocks store use until reopen.
The later daemon integration must transfer this lock ownership into one publication owner.
No live collector, TUI, or writer calls these modules yet.

## Implemented catalog boundary

The catalog stores the synthetic root label, router read roles, WAN/subscriber source origins, local speed profiles, and hierarchy instances.
All discovered definitions can remain in the catalog without selection. Selection comes from explicit instance and target references.
New WAN definitions do not expand an existing target list.
Equal labels do not merge source, instance, profile, or subscriber IDs.
Ordinary edits cannot change a source's authoritative origin under the same ID.

Manual base rates, factors, automatic group formulas, backstop inputs, and percentage modes remain distinct stored values.
Instance factors replace shared factors. WAN factors cannot exceed configured true capacity.
A compiled WAN rate is its effective local shaping target. WAN percentages still use the stored configured base.
Local and SourceRead base authorities are explicit profile choices. Discovery does not choose or overwrite them.

The store canonicalizes ID tables and target lists. An order-only edit does not replace the saved file.
A configured catalog cannot become None through an ordinary edit.
A complete snapshot must match the selected subscriber scopes of the current catalog.
The validated registry index carries those scopes without a separate catalog scan for each subscriber.
Policy history, verified source adapters, export precision, and native proof remain required before activation.

## Implemented placement boundary

Complete snapshots carry current source-profile evidence and optional source plan rates.
Unread profile evidence differs from a verified absence of a profile.
Reconciliation keeps these observations outside the durable registry and retains one account record per active identity.

The resolver indexes selected sources by scope and profile reference.
It checks explicit source targets, source-backed ancestors, effective profile restrictions, and all current aliases.
Active aliases intersect destinations. Conflicting default profiles or SourceRead plans block the candidate.
A selected rate profile must declare Local or SourceRead authority. Missing authority does not select SourceRead implicitly.
Local rates remain authoritative despite current source rate changes.

An explicit source-backed leaf can supply a destination without a shared target rule.
When a source has a nonempty target set, that set restricts its instances.
A broader pool cannot bypass a source-backed ancestor's membership or target restriction.
One circuit receives one placement and contributes its base plan demand once, regardless of endpoint count.

Reserve valid pins and saved assignments before new allocations. Routine collection retains those destinations.
Invalid saved intent produces a typed conflict. No conflict path silently unpins or assigns a fallback.
The result exposes fresh active observations, valid preview placements, and conflicts.
It supplies a candidate registry only when all required active placements pass.
That candidate still requires aggregate, minimum, export, and native checks before publication.

Allocation compares projected plan demand against independent capacities in each direction.
It uses the worst directional ratio along the independent ancestor path, then destination demand, then stable target ID.
Automatic groups use the nearest independent ancestor for allocation. Unbounded groups receive equal plan-demand weights.
The resolver never uses a capacity computed from the members under assignment as an independent weight.
WAN percentage rules use configured true capacity, then respect effective ancestor ceilings.
Checked integer sums and u128 cross products avoid floating-point score changes and arithmetic wrap.

Graph context uses an iterative traversal. Selector indexes avoid scans of unrelated source profiles for each circuit.
Candidate work still depends on eligible target count and ancestor depth. This is not a maximum-throughput claim.
The synthetic 10,000-circuit test checks determinism. Production benchmarks remain separate.
No live collector, writer, or interface calls the resolver.

## Implemented aggregate boundary

`aggregate::evaluate` consumes a complete placement candidate with current account and endpoint evidence.
It checks one placement per active circuit and matches each target against the candidate assignment.
It checks current layout membership again to reject mixed subscriber/group children.

Automatic groups use only direct active circuits for their calculation basis.
PPP and DHCP per-plan groups use authoritative plan sums. DHCP site and Hotspot groups use count multiplied by unit rate.
Custom automatic groups use plan sums with their compiled factor choices.
Each circuit counts once, even when it has several current identities or endpoints.
The calculation applies the compiled directional factors once.

Source-backed automatic groups then apply their backstop and optional explicit floor.
The catalog stores `GroupDefaults.floor` as an optional directional rate.
An absent field loads as None and does not add a serialized field or invent a default.
A later migration must copy the current source floor explicitly and prove parity.
Existing schema-2 documents without the field retain their policy meaning.
A previous reader cannot load a configured floor it does not support. Keep the required backup and rollback gate.

The result separates the pre-backstop calculation, local target, and ancestor-limited ceiling.
Manual nodes retain configured capacities. Inherited intermediates use the effective parent ceiling.
WAN percentages use true configured WAN capacity before ancestor constraints.
A floor never increases an ancestor or bypasses a backstop. An impossible floor becomes a typed conflict.
Manual modes do not inherit automatic source backstops or floors.

The evaluator sums descendant plan, maximum, and minimum demand for checks and display.
These sums do not recalculate an intermediate node's capacity from descendant subscribers.
It compares summed minima against every applicable directional ceiling, including common ancestors of sibling groups.
It does not require the sum of child ceilings to fit their parent. Subscriber maxima remain unchanged.
Plan and maximum oversubscription produce separate warning flags. Impossible minima block the candidate.

An empty automatic branch has no result capacity. Its definition and stored placement remain intact.
Manual/WAN nodes remain in the result, including when empty.
The graph pass uses flat child links and iterative traversal. Arithmetic uses checked integer sums and products.
Graph and demand work is linear in nodes plus placements. Canonical result order adds an ID sort.
The 20,000-level test checks recursion safety. It is not a production benchmark.

The output supplies a candidate only if aggregate and commitment checks pass.
These are necessary mathematical checks for the proposed placement, not a proof of global allocation feasibility or packet scheduling guarantees.
Existing export rate bounds, precision, unique names, native admission, and DPDK evidence checks still apply before publication.
No live path calls this stage.

## Implemented scoped rebalance

The public API is `topology_manager::rebalance::rebalance`.
It accepts a fresh reconciliation and a request with an expected policy revision and a source-pool or branch ID.
The implementation shares the resolver's private index through a child module. Eligibility and capacity rules have one owner.
Normal resolution assigns new circuits first and preserves saved placements.
The explicit action reports its proposed moves relative to that normal result.

A branch action uses descendant destinations only. It does not move a subscriber to another WAN outside that branch.
A source-pool action uses current verified source membership and the pool's selected destinations.
Without an explicit target set, it uses selected instances of that source and their descendants.
An unselected discovery or WAN inventory source cannot become a subscriber pool.
The action does not pull a placement from outside its selected destination set.
Pins, offline intent, and placements outside scope remain fixed.
All current placements contribute to demand, including those fixed by pins or scope.

The action visits movable circuits once, with larger plans first and stable subscriber IDs for ties.
It compares the worst affected directional demand/capacity ratios before and after each possible move.
A move must improve that bottleneck, or improve destination balance without worse independent constraint scores.
Equal bottleneck quality keeps the current placement, including ties caused by indivisible plan sizes.
Sparse changes to descending score multisets rank improving moves with exact rational comparisons.
Independent capacities take precedence over destination shares. Automatic capacities do not become allocation weights.
A scope with mixed bounded and unbounded destinations requires compatible explicit capacity policy before movement.
All-unbounded scopes use equal plan demand.

Trial scoring uses only affected independent paths and destination loads. It avoids a full graph scan for each possible move.
One action performs one bounded sweep. It does not prove a global optimum or globally minimum move count.
It does not schedule another sweep or enable background redistribution.
The final proposal passes through the aggregate stage once.
Minimum/floor conflicts withhold the candidate. The action does not reduce rates, bypass pins, or broaden scope.
The result contains proposed moves and the aggregate result, including warnings and conflicts.
The API performs no durable write, publication, packet routing, or native verification.

Tests include 10,000 circuits and repeat-action stability. This is not a production benchmark.
A separate oracle compares sparse scores against full recomputation for shared ancestors and different destination capacities.
Public integration covers the revision-checked store, pins, restart, and absence of stored session endpoints.

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
