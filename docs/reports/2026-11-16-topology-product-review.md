# Topology requirements and product review

Reviewed: 2026-11-16, Asia/Shanghai.
Scope: compare the approved topology requirements with the current development checkout.
Result: keep the approved behavior and stage the implementation behind an explicit migration boundary.

## Evidence boundary

Development branch: `lqosync3`, commit `0ad8b8d71cfc97c0aea22bde5d786c34f1ff2b06`.
The checkout contains prior unrelated changes. This review preserves them.
The separate `/opt/LQoSync3.0` checkout uses branch `lqosyncv3`, commit `0e125611077f4b362814044b61025c427c7b28b2`.

The active service uses `/usr/local/bin/lqosync3` with `/etc/lqosync3/lqosync3.toml`.
Neither checkout identity proves the exact contents of that deployed executable.
Installed LibreQoS control and dataplane packages report `3.0.7.202611141944`.

Sources: current files, focused tests, project notes, and the approved discussion record.
No production benchmark, native mutation, service restart, or package change occurred.

## Material differences

These are migration requirements. They are not all defects in the current product contract.

| Priority | Evidence | Difference and required action |
|---|---|---|
| High | `src/cycle.rs:132`, `src/model.rs:127` | The cycle begins from stored CSV rows. Build new active output from fresh observations instead. Keep identity and pins in a separate registry. |
| High | `src/cycle.rs:278` | Grace, removal settings, and failed-source retention can preserve active rows. Manager mode requires confirmed-absence removal and a whole-publication hold on failed required reads. |
| High | `src/cycle.rs:188`, `src/state.rs:32` | Circuit names identify stored rows. Same names across routers can overwrite records. Use verified scoped identity and explicit migration reconciliation. |
| High | `src/sources.rs:176` | PPP active rows enter a map keyed by username. Several sessions can collapse into one row. Keep one subscriber identity with several fresh endpoints. |
| High | `src/sources.rs:270`, `src/sources.rs:386` | Hotspot and DHCP paths use MAC-related identity or current configured fields. They do not implement permanent identity across address changes. |
| High | `src/routeros.rs:59` | The collector reads subscriber tables but no WAN routes or interface inventory. Add an optional WAN role with source completeness and minimal fields. |
| High | `src/topology.rs:21`, `src/topology.rs:83` | IDs and parents derive from names and source groups. Keep persistent instance IDs and resolve approved placements independently of display names. |
| High | `src/topology.rs:113`, `src/speed.rs:40` | Group factors and subscriber minima use global legacy formulas. Add explicit node modes and independent subscriber min/max policies. Preserve legacy parity during migration. |
| High | `src/write_receipt.rs:450`, `src/files.rs:36` | Pair files have separate durable renames. There is no atomic pair transaction. Reuse receipts, prove interruption handling, and avoid a second writer. |
| High | `src/write_receipt.rs:499`, `tests/write_receipt.rs:121` | Current tracking errors can coexist with successful file replacement. A new policy commit requires durable intent before side effects. Optional audit retention must not become a permanent outage. |
| High | `src/apply_evidence/verify.rs:193` | Strong native correlation already exists. Reuse it. A write, import, counter, or `ready: true` must not become a substitute. |
| Medium | `src/main.rs:195`, `Cargo.toml` | The loop uses collection sleeps and signals. It has no recurring IANA timezone schedule engine. Wake for due transitions without a per-subscriber timer. |
| Medium | `src/tui/settings/apply.rs:54`, `src/tui/app.rs:35` | General Settings has a manual configuration workflow. Isolate automatic topology edits without removing credential and configuration safeguards from unrelated settings. |
| Medium | `src/tui/devices.rs:102`, `src/tui/data.rs:160` | The TUI already caches shared data and row indexes. Extend these patterns instead of recalculating topology during draw. |

## Reuse before replacement

- Reuse the daemon lock and current collection concurrency controls.
- Reuse `files::write_secret` for private durable policy data after its failure checks pass.
- Reuse pair rendering, write receipts, import evidence, native verification, and health reporting.
- Reuse TUI themes, focus guards, cached row indexes, and compact-layout conventions.
- Keep general Settings behavior intact except for controls explicitly moved into Topology Manager.

The current node renderer also promotes a node when its parent is missing.
See `src/topology.rs:434`.
The manager must reject a missing parent before rendering. It must not silently repair operator intent by promotion.

## Integrity clarifications

One effective placement applies to a subscriber circuit, not to each row or endpoint independently.
Several current devices can share that circuit and its plan.
Count the circuit once for demand and minimum commitments.
Export each fresh endpoint once with its stable device identity where verified.

Discovered sources with equal names are distinct unless the operator explicitly maps them to one shared definition.
Shared schedule profiles are also distinct from subscriber profiles and topology instances.
Use stable IDs for all three relationships.

The project note `brain/30-knowledge/lqosync-owned-profiles-and-wan-policy.md` directs local ownership of service plans.
The current collector still reads RouterOS speed sources. Later discussion also refers to read base speed.
Keep the authority of each base rate explicit. Do not silently replace a local plan with a discovered RouterOS value.
Preserve current output during migration and resolve any conflicting ownership direction before activation.
Percentage calculations must name their authoritative base reference.

IP-only shaping requires an unambiguous address namespace.
Different router identities do not make identical packet-visible addresses distinguishable in the dataplane.
Reject conflicting current mappings. Do not guess ownership or silently discard one subscriber.
Any VLAN or tunnel classifier solution requires its own proven ingress contract.

The approved publication hold can leave previous mappings active in the native last-known-good world.
Fresh-only processing does not itself remove that previous world.
Show collection age, last publication, and native acceptance independently.
Do not promise immediate disconnect removal while publication is held.
A change to that failure policy requires a separate decision, not a hidden optimization.

## Rate computation corrections

Only the immediate automatic group calculates capacity from subscriber inputs.
WAN nodes keep configured usable capacities. This rule includes a WAN with direct subscriber children.
Intermediate nodes inherit a ceiling or use explicit Mbps or a WAN-capacity percentage.
Reject mixed subscriber/group children in the first version.

Keep the raw calculation, local target, and effective ancestor ceiling separate.
Never lower a subscriber maximum merely to make aggregate plan totals fit a WAN.
Do not count the same circuit once per active device.
Resolve subscriber mode/value overrides before arithmetic.
Apply a scheduled percentage to its approved base, never to a previously scheduled result.

Use checked integer rates in the new calculation core.
Keep exact decimal factors until the single export conversion.
Use ceiling-safe rounding and verify the serialized result against the ancestor limit.
Do not silently apply a floor that raises an aggregate above its parent.
Keep current source-specific formula inputs and prove migration parity before activation.

The term minimum means the configured CIR/commit value in this design.
It does not prove a guaranteed end-to-end service rate.
Validate the installed backend contract before you define minimum-feasibility rejection rules.

## Performance design

| Cost | Required design | Proof |
|---|---|---|
| Identity lookup | Build source/account and subscriber indexes once per accepted snapshot. | Count lookup work as subscriber count grows. |
| Hierarchy | Validate and index parents once per policy revision. | Detect cycles without a full tree walk per subscriber. |
| Demand | Assign once, then update affected immediate groups and ancestor bounds. | A changed endpoint must not trigger full rebalance. |
| Balancing | Keep valid assignments and pins. Rebalance only through approved triggers. | Collection order changes must not move subscribers. |
| Policy resolution | Cache by policy revision, base-rate revision, and active schedule set. | Unchanged inputs must not recompute every TUI frame. |
| Publication | Compare canonical rendered content, excluding observation-only timestamps. | Unchanged output performs no file replacement or reload request. |
| Scheduler | Compute the next boundary per rule and arm one daemon wakeup. | Idle ticks perform no router scan or full-world apply solely to poll time. |
| UI | Cache flattened tree and filtered indexes. Render the viewport only. | Synthetic large trees retain focus and bounded row construction. |
| History | Bound events, diffs, and revision snapshots. Separate durable intent from optional history. | Full history does not cause an endless write/retry storm. |

These are proposed design constraints, not measured performance improvements.
Benchmark with synthetic observations and temporary files.
Do not generate stress traffic or run scale tests on the production dataplane.

## Scheduler decisions that remain open

Approved: several named profiles, weekday selection, times, Mbps/Percentage modes, and timezone default `Asia/Manila`.
The owner also approves subscriber policies and WAN/group capacities as schedule targets.
Scheduled WAN limits cannot exceed configured true capacity. Group limits retain ancestor ceilings.
Schedule transitions do not implicitly authorize placement rebalancing.
Application timezone is independent of the Asia/Shanghai timestamps in these project records.

| Open item | Recommendation for review |
|---|---|
| Overnight interval | Selected weekday means the start day. End occurs on the next day when end precedes start. |
| Overlap | Reject overlapping rules that write the same effective target/control. Permit independent controls. |
| Bounds | Use inclusive start and exclusive end. Reject equal start/end until all-day intent has an explicit representation. |
| Restart | Resolve the policy active now. Do not replay every missed transition. |
| Time jumps | Re-evaluate wall time and timezone boundaries. Use monotonic time only for timeouts. |
| DST | Define ambiguous and nonexistent local-time behavior before supporting such schedules. |

These recommendations are not recorded as owner approvals.
Store IANA timezone names, not only UTC offsets.
The current dependency set has no dedicated IANA timezone engine.
Select and verify one maintained dependency when the schedule contract is settled.

## Verification from this review

Focused baseline checks use temporary or synthetic fixtures.
They prove current behavior. Some tests cover behavior that the manager migration must replace.
The following checks passed during this review:

| Command | Passed |
|---|---:|
| `cargo test --lib cycle::tests -- --test-threads=2` | 37 |
| `cargo test --lib sources::tests --quiet` | 25 |
| `cargo test --lib topology::tests --quiet` | 15 |
| `cargo test --lib validate::tests --quiet` | 11 |
| `cargo test --lib apply_evidence::verify::tests --quiet` | 5 |
| `cargo test --lib tui::data::tests --quiet` | 13 |
| `cargo test --test write_receipt --quiet` | 11 |

Total: 117 tests passed, zero failures.
An earlier library filter for receipt tests matched zero tests. The integration command above supplies the receipt evidence.
Documentation whitespace and relative-link checks also passed.
The STE checker ran on the three new documents. Technical-name and list heuristics need manual review.
No certified STE compliance claim follows from that checker.
No result here proves the future topology manager or scheduler works.

## Deliverables and next boundary

The [implementation design](../design/TOPOLOGY_MANAGER_IMPLEMENTATION.md) defines the proposed core boundaries.
The [foundation plan](../superpowers/plans/2026-11-16-topology-foundation.md) starts with isolated policy, rate, and persistence logic.
The [discussion record](../design/TOPOLOGY_MANAGER_DISCUSSION.md) remains the approval history.
The [TUI layout](../design/tui/TOPOLOGY_MANAGER_LAYOUT.md) remains the initial approved interface baseline.

Resolve the schedule contract before activation of that feature.
Do not migrate the live topology source or deploy this design as part of a documentation review.
