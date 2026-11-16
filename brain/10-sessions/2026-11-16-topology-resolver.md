---
type: session
created: 2026-11-16
tags: [topology, placement, identity, reliability, performance, ste]
status: placement-boundary-complete
related: ["[[2026-11-16-topology-catalog]]", "[[HANDOFF]]"]
---

# Fresh membership and stable placement

The owner asks for status and directs the next implementation slice.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `b94ed7c` plus prior uncommitted slices.
Agent: Codex `/root`. Timestamp: 2026-11-16T20:41:06+08:00, Asia/Shanghai.
Unrelated changes remain intact. No commit, push, deployment, service, or live configuration changed.

## Changes

Added `src/topology_manager/resolver.rs` and `resolver_tests.rs`.
Extended observations with current profile evidence, optional plan rates, and one account record per active identity.
Updated the registry integration fixture for the new observation field.
The durable registry contains none of these current observations.

The resolver checks source/profile membership, explicit targets, source-backed ancestors, and effective profile restrictions.
Active aliases intersect eligible destinations and must agree on SourceRead plans.
One circuit contributes its plan demand once, regardless of endpoint count.
Local authority keeps local rates. SourceRead authority requires current rates.
A profile must declare rate authority. The resolver does not select SourceRead implicitly when no policy supplies authority.

Valid pins and saved placements contribute demand before new circuits receive assignments.
Routine collection keeps valid destinations. Invalid saved intent becomes a typed conflict.
New allocations compare projected directional demand against independent capacities and use stable ID ties.
Automatic groups use independent ancestor capacity for allocation, or equal plan demand without one.
WAN percentage rules use true configured capacity and respect effective ancestor ceilings.

The result exposes fresh observations and preview placements.
Any unresolved required placement withholds the candidate registry.
A placement candidate is not a publication-ready or native-verified world.

## Review corrections

Regression tests exposed inconsistent equality for equivalent rational scores.
Exact cross-product equality now agrees with ordering.
Explicit source instances now work without a shared target list.
An existing catalog test proved that nonempty target sets must still restrict those instances.
Another regression proved that a broad pool must not bypass an ancestor target restriction.
A final regression removed implicit SourceRead authority when no profile supplies authority.

An asymmetric test initially expected equal allocation with a download bottleneck on one WAN.
The corrected fixture makes upload the bottleneck on both WANs.
Clippy requested one collapsed condition. The code now passes denied warnings.

## Proof and limits

Added 31 checks: 2 observation checks and 29 resolver checks.
The full suite passes: 824 passed, zero failed, 9 ignored.
`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass.
Logs: `.superpowers/sdd/2026-11-16-topology-resolver/`.

Tests cover two WANs with one profile, direct WAN destinations, unbounded groups, asymmetric capacities, factors, and true-base percentages.
They also cover alias conflicts, min/max replacement, fresh return data, restart persistence, overflow, and target restrictions.
The synthetic 10,000-circuit case checks deterministic output under reversed source order.
It is not a production benchmark.

Graph context uses iterative traversal. Selector indexes avoid unrelated profile scans per circuit.
Candidate cost still depends on eligible target count and ancestor depth.
The graph query used the older project graph for orientation. Direct code and tests supply current evidence.
No graph rebuild or dependency change was necessary for this slice.

## Next action

Implement aggregate formulas and minimum-feasibility checks around these placements.
Then add explicit scoped rebalance, verified identity/source adapters, ID creation, and complete migration/recovery.
Publication must reuse the existing native evidence verifier and serialized writer.
The TUI and scheduler remain unfinished. Scheduler overlap, overnight, and DST rules still need decisions.
No live path calls this module.

Plan: `docs/superpowers/plans/2026-11-16-topology-resolver.md`.
Design: `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`.
