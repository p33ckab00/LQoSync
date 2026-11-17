# Topology Manager implementation design

Status: architecture after review of the current product.
The pure core, registry, catalog, placement/aggregate pipeline, and explicit scoped rebalance exist in isolation.
Catalog ID allocation and a bounded-age RouterOS reader also exist in isolation.
Current PPP, DHCP, and Hotspot normalization also exists in isolation.
Opt-in operator-reference authorization also connects current evidence to registry reconciliation in isolation.
Explicit source profile bindings and exact sustained rate evidence now connect that path to placement/capacity evaluation.
Schema-4 source settings and export ownership, verified pre-edit backups, and forward policy recovery now exist in isolation.
An isolated publication adapter now connects fresh manager output to the current pair writer and receipt tracker.
A diagnostic parity report exposes unresolved legacy identities and unmanaged rows.
Explicit snapshot mappings and compiled-candidate admission now extend that boundary.
Durable export aliases, independent static ownership, and explicit legacy source priorities now exist in isolation.
Controlled compilation and a runner API now share one runtime writer owner.
Configured source orchestration now connects to an explicit runner sync API.
A configuration preflight now blocks unsupported source rules and checks configured legacy adoption.
Fresh source normalization applies global/router exclusions and binds prepared output to that policy.
Explicit DHCP override bindings now connect legacy rate entries to enrolled independent subscriber references.
Imported dynamic bounds now preserve legacy factors, floors, and rounded rate behavior for source-read profiles.
Explicit per-direction node minimums now have policy, hierarchy budget checks, compatibility export, and dormant migration checks.
Configured collection now binds independent static inputs to a private source document, revision, and byte digest.
Explicit static attachments and shared budget checks now support protected roots below managed WAN/intermediate nodes.
Exact shared legacy extraction and adoption proposals now preserve explicit ownership mappings in isolation.
Controlled first-adoption staging and forward recovery now connect the proven static source and registry in isolation.
Shaping-planner admission, schedules, service activation, and TUI remain unfinished.
See the [settings/recovery plan](../superpowers/plans/2026-11-17-topology-settings-recovery.md) for this storage boundary.
See the [source policy plan](../superpowers/plans/2026-11-17-topology-source-policy.md) for mapping and exact rate scope.
See the [identity plan](../superpowers/plans/2026-11-17-topology-identity.md) for the reference contract and acceptance proof.
See the [normalization plan](../superpowers/plans/2026-11-16-topology-normalization.md) for current account evidence and its limits.
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
| `model.rs` | Registry identities, subscriber intent, and the schema-4 document |
| `catalog.rs` | Router roles, source definitions, local profiles, hierarchy inputs, and reference checks |
| `ids.rs` | Typed catalog ID batches from the shared durable counter |
| `reader.rs` | Role-based complete current RouterOS evidence with time and configuration checks |
| `coordinator.rs` | Configured parallel source batches, session ownership, authorization, and fresh preparation |
| `preflight.rs` | Typed source compatibility issues and conservative checks of dormant legacy policies |
| `normalize.rs` | Typed current accounts/profiles/endpoints with local joins and presence checks |
| `publication.rs` | Fresh pair preparation, exact decimal rates, deadline fences, and the existing publication owner |
| `migration.rs` | Explicit snapshot mappings and protected ownership classification for parity |
| `admission.rs` | Exact compiled policy checks and the public admitted publisher |
| `parity.rs` | Diagnostic legacy comparison without identity inference |
| `compiler.rs` | Private compilation, library/configuration identity, and sealed runtime admission |
| `compiler_sandbox.rs` | Kernel controls for the compiler child |
| `runtime.rs` | Durable runtime ownership and legacy fallback prevention |
| `settings.rs` | Durable service/profile selectors, reference authority, and static scope validation |
| `identity.rs` | Explicit reference authority, selected scope membership, conflicts, and fresh registry reconciliation |
| `source_policy.rs` | Current service/profile selectors, stable source references, and explicit rate read modes |
| `source_rate.rs` | Exact sustained rate text to integer bps without floating point or default speeds |
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
The current private document uses schema 4 at `topology-manager/registry.json`.
It contains the registry, an optional catalog, and optional source settings.
None means unconfigured. It does not activate a default topology or reference authority.
Strict schema-1/2/3 adapters preserve IDs, verified references, counters, catalog policy, and subscriber intent.
Open does not rewrite an old primary file. A revision-checked commit writes schema 4 and advances the revision.
Unknown versions, duplicate fields, and false downgrades are errors.
Before a changed commit, the store saves and verifies the exact old bytes in a private backup.
Backup names include the original schema and revision. Existing conflicting backup bytes block replacement of the primary file.
The store keeps 16 recent schema-4 backups and the backups before schema migration.
Normal forward upgrades create at most one backup for each old schema.
Optional history cleanup errors set `history_cleanup_pending`. They do not reverse a confirmed write result.
Open checks history again, so an unresolved cleanup failure remains visible after restart.
A configured catalog has no lossless schema-1 representation. Configured source settings have no lossless schema-2 representation.
Never drop policy for a downgrade. Use the exact old backup only before new policy or identities require a newer schema.
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
Current normalization and opt-in independent reference authorization now exist below.
Deployment authority configuration and source mapping still require integration before activation.

Reference: [MikroTik PPP AAA](https://help.mikrotik.com/docs/spaces/ROS/pages/132350049/PPP%2BAAA),
[MikroTik DHCP](https://help.mikrotik.com/docs/spaces/ROS/pages/24805500/DHCP),
and [MikroTik API](https://manual.mikrotik.com/docs/developer-guides/api/).

## Implemented current protocol normalization

`normalize.rs` accepts bounded-age reader evidence and returns typed current account data.
It keeps the original router ID, generation, and expiry. Normalized account/profile access checks that same expiry.
Normalization checks age before and after its work. It cannot extend a read deadline.
Current output has no Clone or serde implementation. Neither the router result nor its account rows implement Debug.

PPP active sessions join current secrets and profiles by exact account/profile names within one router and protocol.
Missing or disabled local accounts and ambiguous names produce typed errors instead of silent removal.
Several sessions for one current account retain all current endpoints and one profile reference.
A non-MAC PPP caller ID does not invent a MAC address.
Current profile definitions retain raw rate text. Missing rate text remains absent.

DHCP includes only `bound` leases. Known non-bound states contribute no endpoints, even for static retained leases.
Unknown states produce a typed conflict. Bound leases require fresh active IP and MAC values and an unambiguous enabled server.
Hostname, client ID, comment, and the lease locator remain endpoint-only evidence.
Repeated hostname/MAC values do not merge leases. Direct lease rate text remains uninterpreted input without a default.
An unresolved wildcard server does not justify a guessed service mapping.

Hotspot checks the current login method and local account/profile where applicable.
It also requires a matching enabled server from current `/ip/hotspot` inventory.
Named-account methods retain local-account evidence. MAC and trial methods retain endpoint-only evidence.
Mixed named/MAC sessions for one local user remain endpoint-only, independent of reply order.
A trial session remains separate and cannot borrow a matching local user's reference or profile.
Unknown authentication methods and missing required local users produce typed conflicts.
See [MikroTik Hotspot](https://help.mikrotik.com/docs/spaces/ROS/pages/56459266/HotSpot%20-%20Captive%20portal) for method semantics.

Current addresses require valid unicast values. Required MAC fields reject invalid, zero, and multicast values.
The normalizer checks IP ownership across these protocol reads and reports conflicts at the active row.
It uses indexed joins and sorts output deterministically. Synthetic scale checks do not establish production throughput.

Local-account evidence does not prove an account incarnation survived a rename, replacement, or name reuse.
Endpoint-only evidence does not establish permanent subscriber continuity.
The normalizer does not authorize permanent identity or create complete source reads by itself.
The next stage below performs opt-in authorization before complete source acceptance.

## Implemented independent reference authorization

`identity.rs` consumes current normalized routers and explicit source selections under an expected registry revision.
It requires matching collection generations, complete reads of each selected protocol, and fresh router evidence.
A WAN-only read cannot become a complete empty subscriber read.
Selected scopes must match the catalog's required scopes when a catalog exists.

Reference authority has two explicit API states: Unconfigured and OperatorManagedComment.
Unconfigured authority produces a critical conflict for a selected active account.
The operator-managed contract reads `[lqosync-id:REFERENCE]` only from the current administrator-managed account or lease comment.
It requires an independent reference that the operator never reuses for another subscriber within that source scope.
This is an explicit operator assertion. The marker does not make row IDs, names, or endpoints immutable.
No code writes this marker to a router or enables the contract for existing accounts automatically.

References contain 1–128 ASCII letters/digits, with dot, underscore, or hyphen inside the value.
They start and end with a letter/digit. Address-shaped, malformed, missing, and repeated markers produce conflicts.
Registry identity references use `operator-comment/v1/REFERENCE`, scoped to the stable router/service/protocol identity.
Equal reference values across routers/protocols do not merge circuits. Existing explicit registry aliases still apply.
Two current account rows cannot claim one reference in the same scope.
The operator contract must preserve uniqueness and non-reassignment. Software cannot detect a false operator assertion during an unobserved replacement.

Selections can include the whole router/protocol or explicit service names.
DHCP/Hotspot names resolve to server locators from current inventory. PPP uses explicit documented transport names.
Service indexes limit each account check to matching selections. The code does not scan every source for every account.
Overlapping scopes and partial multi-service membership produce critical conflicts instead of duplicated or leaked endpoints.
The caller supplies stable source scopes and explicit service-name selectors. The current index resolves their server references.
Authority contracts and selection edits need durable policy/UI integration before activation.

Any critical identity conflict withholds the complete collection.
A ready authorization reuses `ObservationGate` and registry reconciliation, with deadline checks before/after acceptance and reconciliation.
The returned world carries the original minimum read deadline for later placement and publication checks.
Saved policy follows an existing verified reference. A changed independent reference creates a distinct identity without the old pin.
Offline reads keep saved intent and contribute no active endpoints. Current addresses never enter the durable registry.

Current profile membership now uses the explicit source policy mappings below.
Rate observations exist only when the corresponding rate read mode enables them.
Both explicit local rates and SourceRead rates can reach the existing placement/capacity pipeline.
The API does not commit candidates or publish outputs. Store revisions and later publication/native checks remain mandatory.
Durable source/authority settings now exist. UI enrollment, legacy parity migration, and publication integration remain unfinished.

## Implemented source policy mapping and exact rate evidence

Each source selection now includes an explicit SourcePolicy.
The separate `service_names` field selects current service names under that scope.
DHCP/Hotspot selectors resolve against fresh server inventory. An unavailable selector produces a configuration conflict instead of an empty source assertion.
A recreated server locator with the same explicit name selector uses the new current locator.
Hotspot reader roles now require `/ip/hotspot` inventory before and after active reads, with two additional read-only commands per collection.
Current server metadata includes name, locator, and disabled state under the original read deadline.

A profile binding maps one current profile name selector to one stable source profile reference.
The current inventory resolves that selector to its current row locator. The locator never supplies an implicit stable reference.
Duplicate names/references and blank bindings fail configuration checks.
An active profile without a matching current binding produces a typed critical conflict.

A recreated locator with the same explicit name selector keeps the configured logical reference and reads its current rate text.
A rename needs a revised selector. Retained row IDs and cached profile rows do not repair a missing mapping.
Names act as operator selection rules here, not proof of a permanent profile incarnation or subscriber identity.
Selected catalog profile filters require matching configured bindings, including during complete offline reads.
Service filters and stable scope membership retain the prior explicit authorization checks.
Service/profile names remain selection rules. They do not prove immutable physical services or subscriber continuity.

Current service/profile inventory has one index per router, scoped by protocol and name.
Each selected profile binding compiles once. Its accounts reuse the compiled membership/rate result.
Invalid rate input in an unused profile does not become an active account observation.
Original generations, read deadlines, and independent subscriber reference rules remain effective.

Rate read modes are Disabled or RateLimit with explicit direction and unitless format settings.
Each mapped profile has its own mode. Unprofiled sources such as DHCP have an explicit direct lease rate mode.
Disabled mode creates no numeric evidence and ignores unrelated raw rate text.
Enabled mode reads current profile or direct lease rate text. Missing text stays absent; invalid active input becomes a critical conflict.
The resolver still requires numeric evidence for SourceRead authority. It does not fall back to an invented speed.

The parser reads the first sustained rate token and supports a symmetric single value or an explicit pair.
RouterOS Rx/Tx order maps Tx to download and Rx to upload. A separate explicit format preserves legacy download/upload and unitless Mbps input.
MikroTik documents these direction and symmetry semantics in [PPP AAA](https://help.mikrotik.com/docs/spaces/ROS/pages/132350049/PPP%2BAAA).
The parser accepts decimal k/M/G scales with case compatibility. It does not write RouterOS syntax.
Burst, priority, and RouterOS minimum fields do not replace the Topology Manager's subscriber policy controls.

Decimal arithmetic uses integers with overflow checks. Fractional bps, zero, malformed values, and overflow produce fixed errors without raw input.
There is no float conversion, rounding, floor, clamp, or guessed default in this parser.
Exact integer observations can exceed floating-point precision. Native bounds and export precision still need their later admission checks.
The existing legacy speed parser remains unchanged.

Bindings and read modes now have strict durable fields. Explicit migration of legacy source priorities, UI, and live activation remain unfinished.
The API does not publish or commit shaping output. Publication must retain/recheck the returned deadline and prove native admission and DPDK application.

`store.rs` reuses the instance lock and synced private-file writer.
A new private directory receives a durable empty registry at revision 0.
A missing file in an existing directory requires recovery; corrupt files never become empty registries.
Edits check the expected revision and disk document, then advance memory only after durable success.
A possible replacement failure or detected external change blocks store use until reopen.
The later daemon integration must transfer this lock ownership into one publication owner.
No live collector, TUI, or writer calls these modules yet.

## Implemented settings and recovery boundary

`SourceSettings` stores explicit selections with service names, reference authority, profile bindings, and rate read modes.
The settings and catalog commit as one revision. Unknown fields and absent required mode fields fail decoding.
Static checks require exact catalog scope coverage, unique selectors/references, and bindings for selected catalog profile filters.
Overlapping service selections fail before collection. DHCP/Hotspot names still require fresh service inventory at collection time.
Unconfigured authority can exist in a saved draft. `authorize_configured` blocks it before acceptance, even when the source is empty.
The configured authorization path uses only the settings from its expected registry revision.
Ordinary edits cannot silently remove configured settings.

`restore_policy` accepts an expected revision and a backup schema/revision.
It restores catalog/source settings as a new revision and keeps current subscriber records, pins, and the ID counter.
Current reference validation blocks a restore that would orphan an identity, pin, profile, or source binding.
A stale revision, unknown backup schema, conflicting metadata, corrupt backup, or link cannot replace policy.
A missing or corrupt primary still blocks open. An older backup alone cannot prove the latest identity counter or account history.
Do not copy an old registry over a newer one to repair policy. Use forward policy recovery against a valid current registry.
Full reconstruction after primary corruption requires independently verified complete identity evidence. Automatic identity rewind remains prohibited.

Backups use mode 0600 inside the existing private directory. A synced temporary file becomes a backup through a non-replacing hard link.
The store verifies the resulting bytes and syncs the directory before primary replacement.
The original atomic writer still handles primary replacement. Uncertain results require reopen and a disk recheck.
No source endpoints, raw rate strings, credentials, or sessions enter the durable settings or backups.
The explicit conversion path below preserves legacy comment/name/default speed priorities. Manager-mode activation remains gated.

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

## Implemented publication boundary

`publication::prepare` consumes authorized reconciliation and keeps its original source deadline.
It reuses placement, aggregate checks, the current pair renderers, the existing validator, and import normalization.
A critical placement/aggregate conflict or expired source evidence blocks preparation.

Export includes the effective node ceiling and the unchanged subscriber min/max policy.
Node keys combine the configured label and stable ID. Duplicate display labels remain distinct.
Stable catalog node IDs remain in node metadata and CSV parent references.
The synthetic root stays in policy metadata. The exported forest has no artificial common capacity.
Empty automatic groups disappear. Configured WAN nodes remain.

Each current session exports one device row. Rows share the durable subscriber circuit ID.
Device IDs hash the current subscriber, source identity, and session locator. They do not establish durable subscriber identity.
Only current addresses and MACs enter these rows. Offline registry intent supplies no endpoints.
Subscriber/device display names use their IDs until explicit label migration exists.

CSV rates use integer decimal Mbps text with up to six fractional digits.
The adapter checks that the imported f32 rate recovers the same integer bps. It rejects precision loss instead of changing policy.
The [LibreQoS ShapedDevice fields](https://github.com/LibreQoE/LibreQoS/blob/develop/src/rust/lqos_config/src/shaped_devices/shaped_device.rs) also use f32 rates.
The existing file validator still applies its minimum-rate rules. Exact compiled-candidate admission now precedes publication.
The installed compiler rounds a fractional node ceiling upward to a whole Mbps value.
The adapter exports node ceilings downward to whole Mbps. The calculated policy value remains unchanged.
`node_export_adjustments` exposes calculated and exported limits for preview.
A 100.5 Mbps calculation exports 100 Mbps. Subscriber min/max rates remain unchanged.
Export rejects a result below 1 Mbps or below current subscriber minimum commitments or a configured automatic group floor.
The adapter then uses the checked bps/Mbps conversion. It never exports a ceiling above the calculated limit.
The JSON import limit permits at most 63 node levels. The adapter checks depth before recursive serialization or validation.

`AdmittedPair::publish` first reconciles an interrupted receipt when possible.
It commits the candidate through the expected registry revision, then checks the source deadline again.
It checks the deadline once more before the current tracker records intent and replaces files in network-first order.
A deadline failure after registry commit leaves durable identity/assignment intent. It does not publish stale endpoints.
A file write result remains pending. It does not establish import or DPDK acceptance.

Optional receipt metadata stores the policy revision, observation generation, and registry digest.
Old receipts have no invented topology context. Old callers set that optional field to None.
Prepared/Verified evidence now binds this context as well as the existing write ID, hashes, and native evidence.
A revision, generation, or digest change cannot consume an earlier proof.
Matching unchanged content keeps its receipt identity and original attempt context.
Matching files without a matching receipt create an attempt with unchanged file outcomes, without replacing file bytes.

The shared tracker now stops both file writes when durable intent storage fails.
Optional event-history failure still reports degraded notification state and does not reverse a confirmed shaping write.
A partial pair remains explicit in the receipt. Fresh collection and the existing repair path remain required after interruption.
The adapter has no live runner caller. These changes do not activate manager mode.

## Implemented legacy parity boundary

`compare_legacy` compares the prepared pair with an explicit baseline pair.
It reports added, missing, changed, or duplicate circuit/device rows and changed network content.
Rate-format differences compare as exact integer bps. JSON numeric spelling differences do not create changes.
Other label, parent, endpoint, rate, metadata, and ownership differences remain visible.
An unmanaged baseline row appears as missing. The report never silently adopts or deletes that row.
Circuit/device IDs must match explicitly. An IP/MAC match does not create an identity mapping.

This report does not convert comment/name/default speed priorities or enroll accounts.
Explicit legacy ID/label/static ownership mapping and source-policy conversion still need parity proof before activation.
The public restart test reaches temporary pair files and the existing Prepared evidence builder with saved settings and fresh source data.
The native evidence tests use fixture artifacts. They do not establish live native admission or a DPDK apply result.

## Implemented explicit mapping boundary

`MigrationManifest` declares the expected revision, observation generation, and exact baseline/candidate fingerprints.
The manifest contains explicit circuit/device row mappings, legacy labels, node mappings, and protected row IDs.
Strict decoding rejects endpoint/session fields. The manifest is evidence for one snapshot, not durable enrollment or an address archive.
A changed revision, generation, baseline, candidate, or expired source read invalidates the mapping check.

Row and node mappings must reference existing objects and preserve one-to-one ownership.
Several device rows of the same declared circuit can map to the same current circuit. Distinct legacy circuits cannot merge implicitly.
The comparison changes only declared ID/label fields. Rates, current endpoints, hierarchy, and extra metadata remain visible.
The comparison preserves the baseline and prepared pair. It does not transfer a registry identity or subscriber policy.

Unmapped rows produce an UnclassifiedRow issue even when their export IDs happen to match.
Protected rows produce a ProtectedRow issue and remain in the comparison.
A protected row cannot also have a managed mapping. Protection alone does not authorize adoption or removal.
The durable adoption path below extends these snapshot checks. Shared static branches remain an activation gate.

## Implemented compiled-candidate admission

Only `AdmittedPair` exposes the public manager publisher. A prepared preview alone cannot publish.
`check_compiled` consumes a fresh prepared pair and compares the compiler's canonical and compiled shaping artifacts.
It requires the supported artifact schema/source and the same ingress identity in both artifacts.
The existing import normalizer checks the complete compatibility tree and circuit/device policy/endpoints.
Canonical node IDs, names, kinds, ancestry, attachments, and intrinsic rates must also match.
Unrequested movement, allowed-parent extensions, anchors, or circuit-match extensions block admission.
A compiler success flag alone is insufficient.

The pair budget is 16 MiB. Canonical plus compiled shaping artifacts have a combined 64 MiB budget.
The offline reader checks combined bounds before parse. The checker counts serialized artifact bytes without another full copy.
Import comparison accepts borrowed network/device parts. It does not clone both compiled artifacts into a temporary document.
Mapping checks index identities and move a single cloned network tree through the remap pass.

`check_artifacts` and the development example provide offline diagnostics only. Serialized rows do not authorize fresh subscribers.
The offline harness now compares requested policy instead of requiring an old node-ID prefix.
Scratch checks use the installed binding, blocked socket connections, and the existing file-write audit.
Integer node limits preserve policy. A raw 100.5 Mbps node compiles to 101 Mbps and fails admission.
A safe 100 Mbps node plus a 50.5 Mbps subscriber compiles without changing the subscriber maximum.
These checks do not apply a native world or prove live DPDK acceptance.

Runtime wiring must obtain artifacts from its controlled compiler and bind them to the current compiler/configuration identity.
It must preserve one lock/writer owner, source deadlines, static ownership, and pending receipt reconciliation.
The existing post-write verifier remains necessary. Full shaping-planner bounds and a live DPDK acceptance check remain separate gates.

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

## Durable legacy ownership and source priorities

`ownership.rs` stores explicit circuit, device, and node aliases in schema 4.
Circuit aliases refer to permanent subscriber IDs. Device aliases refer to verified `IdentityRef` values.
Node aliases refer to stable catalog IDs. The registry stores export IDs and labels, never endpoints or session IDs.

`adopt_legacy` checks the revision, observation generation, deadline, and both content fingerprints.
It requires complete managed-policy parity before it constructs a candidate registry.
An explicit mapping must classify every managed baseline row. Protected rows require declared independent static roots.
The method does not infer identity or enroll a subscriber. The caller commits its candidate through the existing store.

Current device evidence supplies every managed address and MAC value.
An offline managed subscriber emits no rows. A return session can keep its explicit device alias with fresh endpoint values.
Several current sessions cannot claim one legacy device alias. Ambiguous ownership blocks the pair.
The store keeps ownership reservations after disconnect and restart. Ordinary edits cannot remove or transfer them.
Labels can change. Forward policy recovery retains current export ownership and its labels.

`prepare_with_static` requires current authoritative static input for all protected rows and roots.
The protected row IDs and root IDs must match the saved ownership exactly.
Every static row requires an explicit parent in its static forest. Static and managed node IDs cannot overlap.
The combined pair must pass validation, byte limits, and exact compiled admission.
Only independent static roots are supported. A static branch below a managed WAN requires a shared capacity design first.
The future runtime owner must establish the source and revision of static input. Previous managed output is not an authoritative source.

`legacy_rate.rs` converts the existing PPP, DHCP-server, and Hotspot speed priorities into explicit source settings.
The converter preserves source order, rate direction, unitless units, the profile-name expression, and configured default values.
It reuses `SpeedParser` and requires exact conversion to integer bps. A conversion cannot round a subscriber rate.
PPP retains its configured fallback. DHCP has no implicit fallback when the selected sources contain no usable rate.
Hotspot retains its configured per-client rate. Address-based DHCP overrides require a separate explicit identity policy.

The current normalizer retains account, active-session, profile, and service comments only in memory.
A priority rule reads these current values. Different effective speeds across simultaneous sessions block acceptance.
A scope cannot combine legacy priorities with another enabled source-rate authority.
Stable profile selection remains explicit. A legacy source rule does not infer an account identity or change a topology assignment.
Profile and service indexes are shared per router. Ownership preservation uses indexed membership checks.
These structural bounds and synthetic tests do not establish production throughput.

The source-priority converter covers the base speed policy. Complete configuration migration still needs shadow parity.
Address-based overrides, shared static branches, and legacy minimum/factor policies require explicit decisions or matching policies.
No runtime caller or automatic manager-mode switch is added by this slice.

## Controlled compiler and runtime publication

`Runner::publish_topology` accepts a fresh prepared pair and a `ControlledCompiler`.
The method requires apply mode and the v3 pair target. The service loop does not call it yet.
The compiler API owns its subprocess command. Runtime callers cannot supply compiled artifacts or a replacement child command.
The existing low-level diagnostic admission API remains available for isolated checks.
After runtime ownership exists, the low-level publisher also requires a controlled compiler binding and the matching tracker/output directories.

`InstanceLock` binds the canonical state directory, directory identity, and current lock-file identity.
The runner and registry share this guard. The registry does not acquire a second lock inside the runner.
A changed state directory requires a restart. A replaced lock path invalidates the existing guard.
Manager compilation runs before the receipt mutex is acquired. Publication reuses the same tracker as the existing native monitor.
The owner rejects older or repeated observation generations during one runner lifetime.
Fresh tickets cannot be deserialized after restart. The source coordinator advances past the generation in the recovered receipt.

The compiler snapshots the exact library bytes in a private cache. It never treats that binary as newline-terminated text.
Each compilation gets a new private workspace. Current CSV/network bytes enter that workspace with their original values.
The child has a cleared environment, a fixed Python command, null output streams, and a source deadline of at most 60 seconds.
Library input is at most 64 MiB. Native configuration is at most 1 MiB.
The existing pair limit is 16 MiB. Both compiled artifacts together remain at most 64 MiB.
Only regular artifact files are accepted. Symlink artifacts and oversized inputs fail before parse.

The child requires Landlock ABI 3 or later. Content writes stay inside its private workspace.
A seccomp filter denies socket creation, socket transmission, and `io_uring` setup.
An unsupported kernel control blocks compilation. There is no unrestricted retry.
These controls isolate trusted native compiler operations. They are not a general sandbox for hostile executables.
The [Landlock documentation](https://www.kernel.org/doc/html/latest/userspace-api/landlock.html) describes the ABI requirement for truncation control.
The [seccomp documentation](https://www.kernel.org/doc/html/next/userspace-api/seccomp_filter.html) describes the filter and `no_new_privs` requirement.

The installed native loader can migrate its private configuration copy.
The compiler records the effective configuration digest after it restores real source/state paths in that digest input.
Random workspace paths do not create a new compiler identity each cycle.
Source, mode, workspace paths, and requested pair bytes must remain unchanged.
The original library/configuration digest remains distinct from this effective configuration digest.
The original configuration and live pair remain untouched during compilation.

Exact compiled policy admission precedes publication. Current library/configuration bytes are checked again before the pair write.
Receipts store both compiler identities with the policy revision, observation generation, and registry digest.
A new or changed compiler binding creates a new receipt even when output bytes match.
The files keep their bytes in that case. Unchanged output with the same binding keeps its original receipt identity.
The existing native proof checks compiler drift during preparation, validation, and proof acceptance.

The runtime saves `topology-manager/runtime-owner.json` before its first manager write.
The marker blocks the updated runner's legacy cycle after restart. Corrupt or linked ownership data also blocks that cycle.
Do not remove the marker to change writer modes. A return to the legacy writer requires an explicit migration procedure.
Older executables do not understand this marker. Do not run them against a state directory after manager ownership is claimed.

Manager publication sends the existing scheduler reload request automatically.
The request nonce uses the receipt ID. A retry can request import without replacing unchanged pair bytes.
A reload-request failure remains explicit while ownership and the pending receipt remain intact.
The read-only monitor must still establish native acceptance. A compile success or scheduler request is not a DPDK acknowledgement.

The installed-binding scratch tests exercise compilation and the public manager API with active and empty batches.
All runtime pair, marker, and scheduler writes in those tests target temporary directories.
No live native apply occurs. Activation preflight, complete migration parity, native planner bounds, and live acceptance remain gates.


## Configured source orchestration

`Runner::sync_topology(compiler, max_age, now)` collects and publishes one complete current batch.
It uses the current registry source settings, the controlled compiler, and the same runtime writer owner.
The method requires apply mode, the v3 pair target, valid runtime ownership, and successful receipt recovery.
This explicit API does not activate manager mode in the service loop.

A selected catalog router must match exactly one configured router by name.
The catalog ID remains the source scope identity. The name only selects the configured connection.
A missing or ambiguous connection blocks the whole batch before any connection attempt.
Each selected subscriber protocol requires its catalog role and its enabled RouterOS source setting.
No router address, credential, protocol, or identity authority is inferred.

The coordinator reads only selected subscriber protocols and their required account/profile/service tables.
Other catalog roles do not add commands to this batch.
Configured WAN capacity does not require a current WAN interface read.
WAN discovery remains a separate read operation. Placement does not assert actual packet routing.

All selected routers collect in parallel. Connection, inventory checks, and active reads share one monotonic deadline.
The deadline starts before the connection phase. Authorization, pair preparation, compilation, and publication preserve that deadline.
Each router gets one attempt per batch. A later sync starts a new generation and reads every required source again.
Legacy retry and retained-row rules do not supply observations to this path.

Successful sessions can survive between batches. They contain no accepted subscriber snapshot.
Changes to connection names, addresses, ports, TLS settings, usernames, or secret paths discard the old session.
A config reload closes all coordinator sessions. Removed source routers also lose their sessions.
Failed or cancelled reads drop their session. Unfinished tasks stop when the batch exits.
A failed router or critical identity conflict blocks pair preparation for the whole batch.
Errors from the transport and normalizer use fixed categories instead of raw source text.

The coordinator advances its generation before the first asynchronous read.
Failed and cancelled attempts cannot reuse that generation within the process.
A restarted runner advances beyond the topology generation in its recovered receipt.
Only fresh authorized results reach the observation gate and resolver.
The registry retains identity and policy. Every exported endpoint still comes from the current accepted batch.

Tests cover selected PPP, DHCP, and Hotspot sources, parallel reads, deadlines, cancellation, and connection changes.
Complete DHCP unbinds and Hotspot disconnects remove rows from prepared output.
The optional installed-library test exercises the public sync API entirely in temporary directories.
It checks active publication, failed-read fencing, restart, disconnect, and reconnect with a changed IP/MAC.
The authorized Subscriber ID remains stable. No live DPDK apply occurs.

Source reads still use the current RouterOS transport allocation behavior.
The deadline and final pair/artifact byte limits do not establish a hard transport memory ceiling or production throughput.
Independent enrollment, legacy shadow parity, static-input authority, and native planner bounds remain activation requirements.


## Legacy configuration preflight

`preflight::source_issues` checks the selected scopes against the current configuration.
The coordinator requires an empty report before its first connection attempt.
Valid global/router exclusions now pass source preflight and apply to current observations.
Malformed IPv4/CIDR entries and empty relevant name patterns block their selected scopes.
Unrelated protocol settings do not block a scope.
Unmapped DHCP endpoint overrides and unsupported membership still block configured collection.

Configured DHCP client overrides require explicit `dhcp_overrides` bindings in durable source policy.
The preflight never converts an old MAC-address rule into a permanent subscriber identity.
A matching current lease cannot waive an offline override or prove future MAC continuity.

The fresh DHCP path requires `require_bound = true` and `shape_unassigned = false`.
It requires current enabled server evidence. The legacy default for `shape_unassigned` can therefore produce a compatibility issue.
If legacy DHCP discovery is disabled, explicit selected service names must equal the configured server set.
These rules prevent silent collection of servers or leases that the previous policy did not select.
A later implementation must migrate these membership rules explicitly.

`preflight::legacy_issues` also checks declared policies that have no active subscriber.
It compares configured source priorities, sustained base-rate authority, local subscriber minimums/maximums, group formulas, factors, caps, and node minimums.
It uses the current legacy rate function and exact integer-bps conversions for local profile bounds.
Different DHCP service policies require separate source scopes.
The checker preserves the legacy first-per-plan-server factor rule in its comparison.
Unproved dynamic bounds, fixed group capacities, and unmatched native node minimums remain explicit issues.
A capacity floor does not prove a native minimum commitment.

Factor checks compare integer products. They do not use floating-point equality or tolerance.
This first checker supports configured factors with exact conversion at six decimal places.
Factors outside that conversion range remain unproved, even if a rounded output happens to match.
`SourceRead` profiles require structurally matching imported bounds after inheritance.
All four controls must preserve the configured factors, floors, and legacy rounding behavior.
A matching current sample cannot substitute for that structural proof.

`ownership::adopt_configured_legacy` requires current filter proof and a clear configuration report before the existing exact snapshot adoption checks.
It retains revision, generation, content fingerprints, current identity mappings, static classification, and the original deadline.
A current matching row cannot waive a dormant policy conflict.
The lower-level `adopt_legacy` adapter remains available for isolated snapshot diagnostics.
Neither adapter commits a registry, activates a service, or writes a shaping pair.

The report contains scope IDs, setting paths, catalog IDs, and typed issue kinds.
It does not copy endpoint overrides, router credentials, or source comments into diagnostics.
Profile indexes and a single scope-to-profile index avoid repeated subscriber scans for each source instance.
These structural checks do not establish production throughput.

A clear configuration report proves only the checks described here.
Unmapped overrides, unmatched dynamic bounds, static-input authority, shared branches, enrollment, and native planner bounds remain gates.
The existing native monitor must still establish live DPDK acceptance.


## Current-source exclusions

Configured normalization reuses the legacy `Excluder` matcher.
Global and router lists apply together. PPP profiles and DHCP server names use case-insensitive `*` wildcards.
IP rules match IPv4 addresses and CIDR blocks. These rules do not exclude IPv6 sessions.
Only PPP uses profile exclusions. Only DHCP uses DHCP server exclusions. All subscriber protocols use IP exclusions.

Each current active row still requires valid account, profile, server, endpoint, and authentication evidence.
Duplicate current IPs still block normalization, also when an exclusion would remove those rows.
Exclusion occurs after these checks and before the normalizer groups sessions into accounts.
A removed session contributes no endpoint, active comment, or service membership to identity authorization or rate selection.
The reader keeps account/profile/server inventory unchanged. Normalized profile/server inventory also remains available.
Excluded accounts do not need subscriber enrollment.

A partial PPP exclusion preserves only the other current sessions of that account.
It cannot leave an excluded session comment as a competing speed authority.
Complete exclusions produce no active subscriber rows or automatic child groups. Configured WAN roots remain.
The registry keeps previously authorized identity and policy intent.
A later included session needs fresh evidence and its independent identity reference before the same Subscriber ID returns.

The coordinator shares one immutable global exclusion snapshot across its tasks.
It does not clone the whole configuration for each router.
Profile and server wildcard checks run once per inventory item, not once per active session.
Parsed current IPv4 addresses use the matcher directly, without text conversion and another parse.
The legacy collector keeps its previous matcher behavior.

A configured prepared pair carries an optional in-memory filter digest.
The digest covers effective selected scope IDs, configured router names, and their global/router filter lists.
Unrelated protocol filters do not change this digest. No session data enters the digest.
The digest exists only when a relevant filter list is nonempty.
It cannot be loaded as a current observation after restart.

`Runner::publish_topology` requires the current filter binding before compilation.
Configured legacy adoption requires that binding before configuration and exact snapshot checks.
A changed or removed filter list invalidates the prepared pair. A pair without proof cannot publish under active filters.
Normal automatic sync starts another fresh batch to use the changed policy.
The binding does not extend the original source deadline or prove independent static-input authority.

Tests compare current IPv4 membership with the legacy collector.
They cover merged lists, wildcard names, unrelated protocols, retained IPv6 sessions, partial PPP sessions, malformed rows, duplicate IPs, and expiry.
Runtime tests cover removal, fresh reconnect, policy changes, missing proof, and configured alias adoption.
The installed-library scratch test publishes a valid excluded batch with no subscriber rows, then active and reconnect batches.
All scratch pair, registry, and scheduler writes target temporary directories. No live native apply occurs.

Unmapped MAC-based speed overrides remain blocked. Explicit independent identity bindings now support their configured source rates.
Dynamic bounds, native group minimums, static-input authority, shared branches, enrollment, native planner bounds, and service activation remain gates.


## DHCP overrides through independent identity

`SourcePolicy::dhcp_overrides` stores explicit `DhcpOverrideBinding` values.
Each value contains a zero-based `legacy_client_index`, an enrolled `account_ref`, and an exact `RateBps` pair.
The position selects the configured rate entry within that router. It never identifies a subscriber.
The account reference must match an existing identity in the same DHCP scope.
For operator comment authority, the stored reference has the form `operator-comment/v1/REFERENCE`.
The current lease must provide `[lqosync-id:REFERENCE]` through the existing independent authority contract.

Operators complete these bindings through revision-checked source policy edits before activation.
The API does not infer a binding from MAC, IP, hostname, or a matching lease.
The durable policy contains no endpoint. The legacy configuration and collector remain unchanged.
An absent list defaults to empty. Empty lists do not change serialized source policy.
Schema 4 remains current. An older reader rejects a nonempty unknown field instead of silently discarding the override.

Source preflight requires complete coverage of configured override entries across the selected DHCP scopes of each router.
Disjoint service scopes share that coverage. Duplicate positions cannot assign one legacy entry to multiple scopes.
References and positions must be unique within each scope. Rates must be positive and valid.
Subscriber aliases cannot declare conflicting override rates.
The rates must equal the current configured entry after exact Mbps conversion.
The effective subscriber profile must use `SourceRead` base authority. A local base cannot hide the override.
Unselected catalog sources do not define that effective base.
Missing, changed, removed, or unknown bindings stop collection before connection.
These checks also cover offline subscribers.

After independent authorization, the compiled policy finds the rate through an indexed account reference.
The explicit override precedes fallback source text, as it does in the legacy collector.
Normal subscriber min/max controls still apply to the base rate.
Placement, WAN ceilings, and group factors retain their existing rules.
The manager never consults the legacy MAC to find an active subscriber.
A changed MAC alone does not change this explicit policy intent.

The transient filter digest now also includes configured DHCP override rates in list order.
Changed or removed rates invalidate prepared output before configured compilation, publication, or adoption.
The digest excludes legacy MACs and operator labels because those values do not select manager subscribers.
The original source deadline remains mandatory.

Tests cover offline removal, store reopen, stale edits, complete scope coverage, duplicate bindings, and fresh reconnects.
A reconnect test assigns the old MAC to another independent subscriber. The override follows the original verified reference.
The new subscriber receives its own fallback rate.

Unmatched dynamic subscriber bounds remain a configured legacy-adoption blocker for source-read profiles.
This task does not authorize service activation or replace exact snapshot and native planner admission.
See [the DHCP override plan](../superpowers/plans/2026-11-17-topology-dhcp-overrides.md).


## Imported dynamic subscriber bounds

`legacy_bounds::import` returns four explicit `SpeedOverrides` values from the configured bandwidth defaults.
The helper does not change a registry or configuration. Persist the completed profile edit through the existing revision-checked store.
`RateRule::LegacyRoundedBase { factor, floor_bps }` represents the imported behavior.
The rule uses the existing legacy rounding function: three decimal places in Mbps, then the configured floor.

The minimum controls use `min_rate_percentage` and `safe_min_mbps`.
The maximum controls use a factor of one and the larger of `safe_min_mbps` and `max_floor_mbps`.
The minimum factor must be positive and must not exceed one.
This condition and the ordered floors keep each imported minimum within its maximum.
The catalog rejects reversed factors or floors without a current subscriber sample.

Imported factors require exact representation at six decimal places.
The importer requires the existing configuration floors: minimum at least 0.1 Mbps and maximum at least 0.2 Mbps.
All rate conversions require an exact round trip between integer bps and the legacy Mbps representation.
Unsupported precision, invalid factors, and out-of-range rates fail instead of truncation or substitution.
The exact `FixedBps` and `BaseFraction` controls retain their previous behavior.
New policy defaults remain unchanged. Legacy rounding applies only through the explicit imported rule.

Configured legacy adoption compares all four effective controls after inheritance.
It calculates the expected controls once and compares rational factors through integer products.
The comparison covers dormant profiles. It does not infer a complete policy from currently active speeds.
A changed factor, floor, or effective rule retains the typed `DynamicSubscriberBounds` issue.
Source priority, base authority, group factors/formulas, native group minimums, ownership, and exact snapshot checks remain independent.

Each fresh authorized observation supplies the current source plan.
The resolver calculates imported minimums and maximums from that plan without durable session speed storage.
Disconnects remove current rows. Reconnects use the current plan and endpoints with the authorized subscriber identity.
Group calculations and ancestor ceilings keep their existing rules. They do not rewrite subscriber speed controls.

Tests compare the imported rules with the legacy function at floor transitions, rounding boundaries, asymmetric rates, and multiple factors.
Literal checks cover ordinary rates and rates below the legacy floors.
Other checks cover precision rejection, strict serialization, inherited controls, store reopen, and dormant policy changes.
A coordinator test changes the source plan from 50/10 Mbps to 23/7 Mbps after disconnect.
At a 10% factor, the returning subscriber receives minimums of 2.3/0.7 Mbps and maximums of 23/7 Mbps.
The subscriber ID remains stable and the endpoint comes from the fresh read.

Schema 4 remains current. Existing rule variants retain their serialized format.
An older reader rejects the new variant rather than discarding its meaning.
Native export precision and planner admission remain mandatory. This import does not establish live DPDK acceptance.
Native group minimums, authoritative static inputs, shared branches, enrollment UI, schedules, service activation, and TUI remain unfinished.
See [the dynamic bounds plan](../superpowers/plans/2026-11-17-topology-dynamic-bounds.md).


## Explicit node minimum commitments

`CatalogNode::minimum` stores a `NodeMinimum` with optional `down` and `up` rates in bps.
Each explicit rate must be positive whole Mbps and must pass the existing exact legacy conversion check.
`NodeMinimum::from_legacy` converts the configured node template with checked multiplication.
An absent direction remains absent. The policy does not substitute a capacity floor or multiply a minimum by a group factor.

An absent catalog field defaults to empty. Empty policies do not change serialized node data.
Schema 4 remains current. An older reader rejects a nonempty unknown field instead of discarding its meaning.
The existing revision-checked store and backups preserve this intent.

The aggregate evaluator checks guarantees from leaves to roots without recursion.
Each node starts with the minimum demand of its direct subscribers.
It adds each included child's required commitment once.
If the node declares a minimum, that direction must cover its descendant obligations.
The required commitment is the larger of that sum and the node's own explicit minimum.
That value must fit the effective node ceiling, including WAN and ancestor limits.
Sibling guarantees therefore share the same parent budget.
A parent guarantee replaces its smaller descendant sum for the next ancestor. The evaluator does not count both values again.
All additions use checked integer arithmetic.

`minimum_demand` still reports subscriber minimum demand.
`minimum_commitment` reports the propagated obligation, including explicit node guarantees.
A conflict removes the publication candidate. The evaluator does not shrink guarantees or increase WAN capacity.
Empty automatic groups remain absent from output. Their stored intent remains available for a fresh reconnect.
Fixed empty branches remain present and reserve their declared minimums.

Publication checks propagated commitments against the whole-Mbps exported ceilings.
This check also catches a parent that fits before quantization but fails after it.
Explicit directions export as integer `downloadBandwidthMbpsMin` or `uploadBandwidthMbpsMin` values.
The current v3 network writer maps these fields to native `commit_down_mbps` and `commit_up_mbps` values.
An absent direction retains the existing native default of commit equal to ceiling.
Subscriber rates, node capacity factors, and WAN budgets keep their separate roles.

Configured adoption checks every instance of a selected source against the legacy template, including dormant instances.
One missing or different direction retains the `GroupMinimum` issue.
A source without corresponding instances cannot claim a nonempty template minimum.
Capacity floors remain separate conflicts. An exact snapshot and the existing compiler checks are still mandatory.
The compiler admission check rejects changed or dropped minimum fields through the existing exact compatibility comparison.

Tests cover hierarchy budgets, sibling excess, explicit parent limits, direct subscriber minimums, quantized ceilings, storage, and numeric limits.
The installed-library scratch compiler preserves a pair with an explicit node minimum.
Its writes target temporary directories. This check does not apply a live dataplane policy.

These mathematical budgets cover declared node guarantees and subscriber minimums.
They do not prove the feasibility of every implicit native default commitment or shaping-planner transformation.
Native planner admission and live DPDK acceptance remain activation gates.
Authoritative static inputs, shared branches, enrollment UI, schedules, service activation, and TUI remain unfinished.
See [the node minimum plan](../superpowers/plans/2026-11-17-topology-node-minimums.md).


The schema-2 writer check in this slice covers nodes only.
Generated manager device IDs exceed that writer's 64-character ID limit.
Current manager publication uses the legacy/manual pair. A future direct schema-2 path requires a compatible identity migration before activation.


## Authoritative independent static input

Configured collection reads `<state_dir>/topology-manager/static-input.json` when export ownership declares protected rows or roots.
This file is separate from generated output and registry identity policy. Collection never creates it from the previous pair.
Only the explicitly owned independent static forest belongs in this source. Dynamic subscriber endpoints still require current RouterOS observations.

The source uses one JSON document with required `schema_version`, `revision`, `rows`, and `network` fields.
Schema version 1 requires a positive revision. Rows use the existing `CsvRow` column names. The network uses the compatibility forest format.
One document prevents a source update from mixing separate CSV and network generations.

For example, this document supplies one independent static root and no static endpoints:

```json
{
  "schema_version": 1,
  "revision": 1,
  "rows": [],
  "network": {
    "Management": {
      "id": "management-root",
      "type": "site",
      "downloadBandwidthMbps": 20,
      "uploadBandwidthMbps": 20,
      "children": {}
    }
  }
}
```

The registry must explicitly protect `management-root` before collection accepts this example.
Every supplied row and root must match protected ownership. Missing entries, extra entries, collisions, or invalid endpoints block publication.
The existing independent-tree validation remains mandatory. Static rows cannot borrow managed parents.

The loader requires a private directory and regular file owned by the current user. It rejects a symlink, unsafe permissions, or oversized source.
The source byte limit is 16 MiB. The row decoder stops above 100,000 entries before it builds the complete row vector.
Existing combined export limits remain separate.
Missing or invalid source files fail before router connections. Semantic ownership checks also run before pair admission.
The automatic collection path rereads the current source for every batch. It does not cache static endpoints between batches.

`StaticBinding` records the absolute source path, revision, and SHA-256 digest of the complete source bytes.
Prepared and compiled pairs retain this binding. Admission and publication checks reread the file and verify the digest.
Runtime publication requires the configured owner's source path. A raw `prepare_with_static` preview has no authoritative proof and cannot publish.

The runner checks revision continuity before the expensive compiler call. Publication repeats this check before registry commit.
Against the current receipt, a lower revision or different path fails. Changed bytes at the same revision also fail.
Every source edit, including whitespace, requires a higher revision. Replace the complete document atomically with mode 0600.

Write receipts store the optional binding. A new binding creates a new receipt even when generated output bytes are unchanged.
This retains the current revision across restart. Existing receipts without the field still load.
The receipt version remains 1. Older strict readers reject a nonempty new field; do not downgrade while such a receipt is active.
Apply evidence retains the complete context. Changed source path, revision, or digest cannot reuse previous evidence.

Checks run at admission and publication boundaries. They do not lock an external editor across output writes.
A detected change before registry commit leaves policy and output unchanged. A later detected change blocks the file intent after the policy commit.
The next attempt must collect current dynamic evidence and prepare a new pair. Existing recovery rules still apply to interrupted output writes.
The revision comparison uses the current receipt, not a separate permanent revision ledger.

Shared managed/static branches, enrollment UI, schedules, TUI, service activation, and native planner proof remain unfinished.
The automatic sync API exists; the service loop remains gated. This source binding does not prove live DPDK acceptance.
See [the static input plan](../superpowers/plans/2026-11-17-topology-static-input.md).


## Explicit shared static branches

`ExportOwnership::shared_attachments` stores a list of `StaticAttachment` values.
Each value maps a protected static `root_id` to an internal managed `parent_id`.
A root can have one attachment. Several roots can use the same parent. The parent must be a catalog WAN or intermediate node.
Subscriber groups remain leaves. Missing parents, unprotected roots, and duplicate attachment roots fail policy validation.

The authoritative static source remains a forest. An attached root stays at the top of that source document.
Publication inserts it below its managed parent. A protected source root without an attachment remains an independent output root.
This separates explicit static ownership from its current hierarchy position. The source binding and revision checks remain mandatory.

For example, add this policy field when `management-root` is already in `protected_roots`:

```json
"shared_attachments": [
  { "root_id": "management-root", "parent_id": "wan1" }
]
```

The parent ID identifies the managed catalog node. The exporter resolves durable ID and label aliases before insertion.
Static root IDs, labels, capacities, minimums, descendants, and rows remain unchanged.
An attachment never duplicates a static root across WANs. Distinct roots require distinct owned IDs and labels.
Global node ID and label collision checks still apply. The combined tree must satisfy the JSON import depth limit.

## Shared capacity checks

The merge checks shared trees after managed export has floored capacities to whole Mbps.
Attached static nodes also require positive whole-Mbps ceilings. Each child ceiling must fit its immediate parent.
Publication rejects an oversized static child; it does not clip the source policy or alter subscriber speeds.
Subscriber maximums can exceed the containing node ceiling. Their minimum guarantees must still fit the shared budget.

The checker sums subscriber minimums once per circuit, across all its device rows.
Device rows for one circuit must use the same parent and minimum/maximum rates.
A shared circuit cannot span a shared node and another parent. A shared node cannot mix direct circuits and child groups.

For each direction, an attached static node without an explicit minimum conservatively reserves its full ceiling.
This also reserves capacity for an empty static branch. An explicit static minimum must cover the branch's descendant obligations.
A managed node retains the current rule: absent explicit minimums contribute descendant obligations without a new implicit reservation.
Native defaults and planner transformations still require separate admission proof before activation.

The reverse traversal checks each explicit guarantee against its descendants and its exported ceiling.
A child obligation counts once. The parent's explicit guarantee replaces a smaller descendant sum before propagation to the next ancestor.
Static and managed siblings share the same parent and WAN budget. Download and upload checks are independent.
Every sum uses checked integer arithmetic. A critical conflict blocks the whole candidate before compiler admission and registry commit.

The checker does not change saved subscriber assignments or automatically run a rebalance.
Existing placement stability and explicit rebalance scope remain in force. A new static reservation can therefore expose an existing assignment conflict.
The operator must resolve the conflicting policy or use the separately scoped rebalance action.

## Shared branch persistence and limits

Schema 4 remains current. The new policy field defaults to empty and stays absent from serialized empty ownership policy.
Existing documents keep their previous shape. Older strict readers reject a nonempty attachment field.
The store canonicalizes attachments by root ID and persists them with the completed revision-checked settings edit.
Reparenting changes hierarchy intent; it does not transfer or remove protected root ownership.

Insertion visits the bounded hierarchy once. Budget checks use flat indexes and an iterative reverse traversal.
Work and temporary storage grow with nodes, rows, and attachments. Independent-only policy skips the new shared budget pass.
The existing source byte, source row, export byte, and depth limits remain in force. No production performance benchmark occurred.

Configured collection and scratch compilation now exercise attached static policy with fresh dynamic observations.
Compiler admission rejects changed or removed static attachment evidence. These checks do not establish live dataplane acceptance.
The existing legacy adoption helpers still classify independent top-level static roots only.
Shared legacy snapshot extraction and exact preservation need a separate migration slice.
Enrollment UI, schedules, TUI, service activation, native implicit commitments, and planner/live proof remain unfinished.
See [the shared branch plan](../superpowers/plans/2026-11-17-topology-shared-static.md).


## Exact shared legacy adoption proposals

`legacy_static::adopt` accepts a fresh prepared pair, the complete legacy snapshot, its explicit migration manifest, selected static root IDs, and attachments.
The manifest still binds the registry revision, observation generation, baseline digest, and fresh candidate digest.
Changed endpoints, rates, labels, or snapshot content require new evidence. Matching IP/MAC values never authorize identity continuity.

The helper extracts selected subtrees from their actual positions. It retains unselected nodes and fields in the managed forest.
Each nested selected root must have an attachment to its actual managed parent, after explicit legacy node mapping.
An independent root must remain independent during adoption. Ordinary later hierarchy edits remain a separate action.
Missing roots, overlapping selections, duplicate attachments, or a mapped managed node inside a selected static subtree fail.

The existing managed adoption helper proves parity after the explicit static partition.
It builds durable circuit, device, and node aliases through the existing authoritative identity mapping.
The new path then resolves those aliases in the fresh managed snapshot and inserts the static branches at their declared positions.
Source node IDs cannot collide with managed export IDs, including IDs of dormant managed nodes.

The complete reconstructed snapshot must match the original legacy snapshot under the existing parity rules.
Comparison includes every row field, all network fields, and the hierarchy. Numeric rate formats normalize to exact integer bps.
The snapshot digest still binds the supplied row order. Semantic parity allows a different output row order.
The checker applies existing import, size, collision, and shared guarantee checks before it returns a proposal.
It never clips rates or moves a mismatched branch to make the migration pass.

`legacy_static::adopt_configured` also checks current source-filter proof and the legacy configuration preflight.
A changed filter or unmatched legacy speed policy blocks configured adoption, including when current row samples happen to match.
The original independent-root adoption helpers retain their API and behavior.

## Static adoption source draft

`StaticAdoption` returns the proposed registry and a private static forest/row payload through `static_input()`.
`source_document(revision)` serializes that payload as source schema version 1. The revision must be positive.
The source draft follows the current 100,000-row and 16-MiB reader limits. Serialization borrows rows and network values.

Only explicitly protected static rows and selected static subtrees enter the draft.
Dynamic subscriber endpoints enter neither durable registry policy nor this source document.
After restart or disconnect, the normal collection path must obtain current dynamic endpoints again.
Tests preserve legacy subscriber aliases while returning subscribers receive a new IP and MAC.

The helper performs no registry commit, source installation, output write, writer claim, or service action.
Repeated calls with the same fresh evidence return the same proposal. Expired evidence cannot be reused.
Discarding a proposal requires no rollback writes. Existing verified backups and forward registry recovery remain in use after later committed edits.

The caller must establish the returned source as explicit operator-owned policy before normal configured publication.
The source must use the private file boundary and current revision checks.
Use the controlled first-adoption transaction below to install the returned source and registry together.
The proposal API alone does not prove installation.

Focused tests cover actual parent mapping, managed aliases, mixed independent/shared roots, classification gaps, dormant identity collisions, and budget conflicts.
A temporary-store test covers restart, disconnect, and fresh reconnect data.
The installed-library scratch compiler preserves the adopted legacy aliases, static branch, endpoints, and speeds without live apply.
No production benchmark or independent review occurred. Native implicit commitments and planner/live proof remain gates.
Schedules, enrollment UI, TUI, migration activation, and service activation remain unfinished.
See [the shared adoption plan](../superpowers/plans/2026-11-17-topology-shared-adoption.md).

## Controlled first-adoption transaction

`staging::stage` accepts a sealed `StaticAdoption` and the locked `RegistryStore`.
The proposal keeps its original evidence deadline. Changes to its public registry invalidate the seal.
The store applies its existing identity, ownership, catalog, and revision checks to calculate the exact target policy.

This first-adoption path requires an absent `static-input.json` and `runtime-owner.json`.
It rejects existing files, including symlinks. Source replacement and migration of an active topology owner need a separate contract.
The transaction does not select a runtime mode, claim topology ownership, or write shaping output.

Before installation, the store confirms an exact private backup of the current registry.
The transaction then saves `topology-manager/migration-pending.json` with schema version 1.
Its payload contains the canonical owner directory, base policy, target policy, and exact static-source bytes.
A SHA-256 digest covers the payload.

The journal has a 48-MiB byte limit. The source keeps its 16-MiB limit.
Dynamic session endpoints never enter this payload. Protected static endpoints remain explicit private policy.

The pending journal blocks ordinary registry commits, writer claims, and both runtime publication paths.
Malformed, future-version, unsafe, or corrupt journals also block these operations.
The store can reopen for inspection and recovery. It does not automatically resume the transaction during open.
The future enrollment controller must call recovery before normal runtime work. No new per-batch approval is required.

`staging::recover` first confirms that the current registry matches the recorded base or target.
It confirms the current disk document and owner lock before any installation.
A journal from another owner directory fails. Unknown registry content, source bytes, or runtime ownership fails without replacement.

Before source installation, recovery creates a temporary runtime-owner record with writer `migration-pending`.
Earlier strict runtime readers reject that writer. The marker never grants topology ownership.
Mixed-version registry editing remains unsupported. An older registry editor can cause a conflict that requires explicit recovery.

Installation uses the existing private, no-replacement file publisher for the journal, blocker, and source.
It creates a private temporary file, syncs its contents, and publishes it with a hard link.
An existing destination cannot be replaced, including a file created after the initial absence check.
The publisher then removes its temporary name and syncs the directory. It preserves exact bytes without an added newline.

Recovery installs source revision 1, checks its exact bytes through the existing source reader, and commits the recorded registry target.
The store keeps its ordinary backup, canonicalization, identity preservation, and uncertain-write rules.
An uncertain registry write requires a store reopen. A restart accepts the exact base or target and never increments the revision twice.

After registry and source confirmation, recovery removes the temporary owner marker and then the journal, with directory syncs.
A restart during cleanup can repeat these steps. Recovery returns false when no journal remains.
An existing source with different bytes never receives a guessed repair, even if its IDs match.
A missing source after registry commit requires explicit repair. Recovery does not reconstruct it from historical dynamic observations.

`staging::cancel` is available only while the registry still matches the base and no source exists.
After source installation, recovery must move forward. It never removes adopted identities or restores an older ID counter.
After completion, later static-source edits use the existing revision and receipt rules. The completed transaction does not freeze future edits.

The API has no TUI or service-start integration yet. Native implicit commitments and planner/live proof remain separate activation gates.
File checks cannot lock an unrelated external editor. Supported writers must hold the instance lock and respect pending migration state.
See [the staging plan](../superpowers/plans/2026-11-17-topology-migration-staging.md).
