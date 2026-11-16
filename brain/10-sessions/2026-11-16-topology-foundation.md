---
type: session
created: 2026-11-16
tags: [topology, implementation, performance, integrity, reliability, ste]
status: foundation-complete
related: ["[[2026-11-16-topology-product-review]]", "[[HANDOFF]]", "[[ADR-0005-topology-follows-v3]]"]
---

# Topology foundation

The owner authorizes implementation after documentation commit `b94ed7c` and requests another code review.
Worktree: `/home/pi/LQoSync3.0`. Branch: `lqosync3`. Base: `b94ed7c`.
The separate `/opt/LQoSync3.0` checkout and installed service remain unchanged.

## Changes

- Added `src/topology_manager/rates.rs` for integer bps, exact fractions, independent subscriber controls, and aggregate ceilings.
- Added `src/topology_manager/layout.rs` for ID-based forest validation.
- Exported the module through `src/lib.rs`. No production path calls it.
- Added `tests/topology_foundation.rs` for current formula, fixture, and migration parity.
- Updated the plan, implementation design, and discussion state.
- Preserved unrelated local notes and operations changes. No new commit or push occurred.

New defaults keep 100 percent maxima and fixed 1 Mbps minima, independently by direction.
Overrides replace inherited controls. They do not multiply inherited percentages.
Aggregate ceilings do not change subscriber maxima.

WAN percentages use configured WAN capacity and keep lower ancestor ceilings.
Invalid minima and zero rates return errors.
Division by zero and overflow also return errors.
Legacy conversion rejects nonfinite values and inexact integer-bps round trips.
It does not replace native export validation.

## Code review

The legacy speed helper silently clamps some invalid values. The new resolver rejects them.
Wider integer multiplication avoids overflow before checked conversion to u64.
Fraction calculations round down once. Ancestor constraints do not multiply or round the result again.
Successful arithmetic needs no heap allocation.


Hierarchy validation builds one borrowed ID index and flat child links.
An iterative traversal checks each node and edge once and carries WAN ancestry forward.
Expected work is O(N + E), plus ID bytes. Space is O(N).
A reversed 50,000-level tree passes without recursion or repeated ancestor scans.
This check does not measure production performance or maximum throughput.

`Fixed` means the effective local target after manual factors.
Later policy records must preserve the configured base, factor, and resolved value separately.
WAN nodes are roots. The synthetic UI root is not a capacity node.

`SubscriberGroup` represents a leaf group. Put direct members in a subscriber group below an intermediate node.
Duplicate labels remain legal model values. Export-name uniqueness remains an adapter responsibility.

## Checks

TDD evidence: 11 rate, 13 hierarchy, and 7 compatibility tests failed at their stubs, then passed.
Full `cargo test`: 735 passed, zero failed, 9 ignored. The change does not enable or alter ignored tests.
The full suite includes the previous cycle, source, topology, validation, native verification, TUI, and receipt checks.

`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` passed.
Whitespace checks passed. No Cargo dependency or existing fixture changed.

Final review found no defect that prevents use of this isolated slice.
STE guidance applies to comments, errors, and documentation. The checker does not certify compliance.
The project graph supplied recall leads. Current code and tests supplied implementation evidence.

## Next work

The Topology Manager is not active or complete.
Next: a concrete durable policy/registry and fresh-observation migration plan.
Then implement assignment, balancing, publication recovery, native proof, and TUI integration.
Keep one count per circuit across multiple endpoints in that later resolver.
Before activation, prove serialized rate precision, backend bounds, full parity, and source identity continuity.

Schedule targets include subscriber policies and WAN/group capacities.
Overlap, overnight-day, and DST behavior remain decisions before schedule evaluation.
Do not use archived IP/MAC/session observations as new active input.
