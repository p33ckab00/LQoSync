# Shared Static Branch Plan

**Goal:** Attach explicitly owned static subtrees below managed WAN or intermediate nodes without exceeding shared guarantees or capacities.

**Architecture:** Extend `ExportOwnership` with optional root-to-parent attachments. Keep the current static source as a forest.
Map attachment parents through durable export aliases. Preserve every static node, row, rate, and ID.
Use one bounded tree traversal for insertion, then check the merged hierarchy before serialization or compiler admission.

Recalculate shared guarantees from circuit minimums and explicit node minimums. Count a circuit once across its device rows.

An attached static node without an explicit minimum reserves its full ceiling. Keep the current managed implicit-default planner gate.
Propagate each child obligation once. An explicit parent minimum must cover descendants, and all obligations must fit exported ceilings.
Reject an attached static ceiling above its parent. Do not reduce subscriber rates or static policy to fit.

**Skills:** writing-plans, test-driven-development, verification-before-completion, ponytail, brain, and ASD-STE100.
Execute inline. Do not deploy or activate services.

## Steps

1. Add failing ownership/publication tests for explicit attachments, parent aliases, and shared budget conflicts.

2. Add the optional attachment policy. Validate unique protected roots and managed WAN/intermediate parents. Canonicalize completed edits.

3. Add `shared_static.rs` for attachment insertion and merged budget checks. Retain independent-root behavior when no attachment exists.

4. Check all affected managed trees after whole-Mbps export quantization. Reject mixed direct rows and child groups in shared trees.

5. Test multiple WANs, nested guarantees, dormant static branches, multiple device rows, rate consistency, collisions, and invalid parents.

6. Verify configured collection, store restart, and installed-library scratch compilation. Run the full suite, format, Clippy, and whitespace checks.

7. Update design and project memory. Record proof and remaining activation gates.

## Scope limits

This slice attaches protected roots already classified by explicit ownership. It does not infer legacy shared-branch ownership.
The existing legacy adoption helper still accepts independent roots only. A separate shared snapshot migration needs exact preservation proof.

Static nodes remain explicit source policy. Automatic managed groups remain subscriber leaves and cannot receive attached children.
Native implicit commitments, planner transforms, live acceptance, enrollment UI, schedules, TUI, and service activation remain separate work.


## Result

The implementation is complete locally. Commit `f104356` contains the prior static-input slice. The push succeeded.
The new shared branch slice remains uncommitted. No deployment or activation occurred.

The full suite passed: 1,157 tests, zero failures, and 12 ignored tests. All 421 active topology tests passed.
Format, Clippy with warnings denied, and whitespace checks passed.
The installed-library scratch compiler preserved the attached static root and managed minimum.

Tests cover explicit parent aliases, multiple WANs, nested guarantees, store restart, duplicate circuit rows, invalid parents, and capacity conflicts.
A circuit with several devices counts once. Subscriber maximums remain 50 Mbps below a 20/10 Mbps static node.
The configured collection test combines source revision checks with the shared hierarchy and current dynamic subscriber state.
Compiler admission rejects moved or removed attachment evidence.

Logs: `.superpowers/sdd/2026-11-17-topology-shared-static/`. Proof: `full-verified.log`, `clippy-verified.log`, `format.log`, and `native-shared.log`.
Author review only. No production benchmark occurred. Shared legacy snapshot migration remains the next slice.
