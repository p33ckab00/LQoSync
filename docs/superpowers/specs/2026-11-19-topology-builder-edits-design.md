---
type: spec
created: 2026-11-19
status: draft
related: ["[[topology-manager-status]]", "[[2026-11-19-topology-wan-discovery]]", "[[2026-11-18-topology-subscriber-plans]]"]
---

# Builder edits design (Phase 2b-3b)

## Intent

The owner wants to build the production tree in LQoSync: two WAN nodes from the `pppoe-out` interfaces of the MikroTik core, optional intermediate nodes, and one instance of each subscriber group below each WAN.
LQoSync balances the subscribers of each group between its instances, so that the two WANs carry equal plan demand relative to their capacity.
This slice adds the Builder edits that make this tree, through the service API and the Topology TUI.

## Owner decisions (2026-11-19)

- Edits: WAN node from a discovered candidate, intermediate node, one more instance of a source below another parent, move (including promote and demote), remove, and node name and capacity.
- After each edit that changes the destinations of a source, LQoSync runs one scoped rebalance of that source pool. The edit and the rebalance commit together in one revision. Pins stay. Equal results prefer fewer moves. There are no background moves.
- Remove moves the subscribers of the removed instance to the other instances of the same source, through that rebalance.
- The TUI shows the affected subscribers and their destinations before the operator confirms.
- Balancing objective: the demand of a subscriber is its plan rate times the factor of its group, in each direction. The rebalance splits the demand of each group across its instances in the ratio of the capacity above each instance. With equal WANs, each group splits equally, so the WANs also carry equal demand. With a 2:1 capacity ratio, the split is 2:1.
- The minimum (guaranteed rate) of a group is split across its instances in the same capacity ratio. The total minimum of the group does not change.
- Approach A (below): propose, then confirm, with the proposal kept in the service.
- Edits do not change live shaping before Phase 4 activation. After activation, the automatic pipeline publishes them. Routing on the core does not change (later feature).

These decisions follow the approved discussion: Decision 5 (a hierarchy edit calculates a proposed allocation and requires a valid reassignment plan before it invalidates current placements) and Decision 6 (an edit that deletes a pinned instance is blocked until the operator resolves the pin).

## Goals

- The operator builds WAN, intermediate, and group instance nodes in the TUI and sees each result before it commits.
- Each committed revision is valid: no placement conflict that the edit caused, no lost subscriber.
- The web frontend can use the same two operations later.

## Non-goals

- Explicit rebalance and pin edits from the TUI (Phase 2b-3c).
- Group factor overrides, profiles, and speed defaults (Phase 2b-4).
- Custom groups without a source, and all-subscriber pools on WAN nodes.
- Router writes or address-list changes.

## Anti-goals

- A commit that differs from the proposal the operator saw.
- A revision that strands subscribers on a removed node or gives them `InvalidAssignment` or `InvalidPin` conflicts.
- A silent pin removal.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. Propose, then confirm, with a kept proposal (chosen) | `topology.edit.propose` applies the edit to a copy, reads sources, rebalances, and keeps the result. `topology.edit.confirm` commits that kept result | The commit equals the preview. One router read for each edit. The service holds one proposal with an expiry |
| B. One operation with `dry_run` | The same operation shows moves on `dry_run`, then reads again and commits | No service state. Two reads for each edit. The commit can differ from the preview when subscribers connect or disconnect between the calls |
| C. Structure first, rebalance after | The edit commits the structure, then the TUI sends a separate rebalance | Two revisions with an unbalanced state between them. No move preview before confirm. Against the one-revision decision |

The owner chose A because the operator sees exactly what commits, each edit needs one read, and blockers show before confirm.

## Operations (new)

| Operation | Args | Result |
|---|---|---|
| `topology.edit.propose` | `expected_revision`, `edit` | `{proposal_id, expires_at, edit, nodes, moves, cleared, sources}` |
| `topology.edit.confirm` | `proposal_id` | `{revision}` |
| `topology.edit.cancel` | `proposal_id` | `{}` |

- Each operation needs export ownership ("enroll first: the registry has no export ownership"), as the plan operations do.
- A stale `expected_revision` gives `conflict`. A rule failure gives `invalid` with the rule text. No proposal is kept after a failure.
- The service keeps one proposal. A new `propose` replaces it. The proposal expires 120 s after it is made.
- `confirm` commits the kept candidate when the registry revision still equals the proposal base revision and the proposal has not expired. Otherwise it answers `conflict` ("the proposal is stale, propose again") and drops the proposal. A confirmed proposal is dropped.
- A committed edit clears the cached preview, so the next preview read shows the new tree.

### Edit kinds

The `edit` argument is one object with a `kind` field.

| `kind` | Fields | Effect |
|---|---|---|
| `wan.create` | `router_id`, `endpoint_ref`, `name`, `down_mbps`, `up_mbps`, optional `factor_percent` (`[down, up]`) | Adds a WAN source and a root WAN node with `Fixed` capacity. Adds the catalog router (from `[[routers]]`, by name) when the catalog has none, and adds the WAN role to it |
| `node.create` | `name`, `parent` (node ID or `null`), `capacity` | Adds an intermediate node |
| `instance.add` | `source_id`, `parent` (node ID or `null`) | Adds a subscriber group node for an existing subscriber source, with `Automatic` capacity, inherited factors, and its share of the group minimum |
| `node.move` | `node_id`, `parent` (node ID or `null`) | Changes the parent. The node keeps its ID, capacity mode, and factors |
| `node.update` | `node_id`, optional `name`, optional `capacity` | Changes the name or the capacity |
| `node.remove` | `node_id` | Removes one node without children |

Capacity on the wire:

- `{"mode": "inherit"}`
- `{"mode": "fixed", "down_mbps", "up_mbps", "factor_percent": [down, up] or null}`
- `{"mode": "wan_percent", "down_percent", "up_percent"}`
- `{"mode": "automatic"}` (inherited factors)

Allowed modes: WAN node `fixed` only, with factors from 1 to 100 percent. Intermediate node `inherit`, `fixed`, or `wan_percent`. Subscriber group `automatic` or `fixed`.
Rates are positive whole Mbps from 1 to 1,000,000. Percentages are whole numbers from 1 to 100. Names have 1 to 64 characters without outer spaces.

### Rules

- `wan.create`: the candidate must be in the last `wan.discover` answer for that router, with state `ok`, and without `source_id`. The runner now keeps the last discovery answer (this resolves the 2b-3a ruling). A WAN source is unique for each router and interface.
- `node.move`: a WAN node stays a root. A node cannot move below itself or its descendants. Two instances of one source cannot have the same parent.
- `node.remove`:
  - A node with children is refused ("move or remove the children first").
  - A node that a subscriber pin names is refused ("unpin the subscribers first"), with the pinned subscriber IDs.
  - The last instance of a source is refused while the registry has a subscriber record assigned to it ("this is the last instance of the source").
  - Records whose saved assignment names the removed node lose that assignment. Online records get a new destination from the rebalance. Offline records get a destination from the balancer when they return.
  - When the removed node is the last node of a WAN source, the WAN source is removed too. The router keeps its WAN role.
- `instance.add` accepts only a source that has at least one instance already (a selected source). Source settings do not change in this slice (designer call: `SourceSettings` must match the catalog scopes, and a new selection needs an authority and a policy that the operator sets in Phase 2c).
- `node.remove` of the last instance of a source is always refused, for the same reason (designer call).
- Enrolled nodes with an export node alias keep their legacy export name (designer call):
  - `node.move` and capacity changes are allowed.
  - `node.update` of the name is refused ("the export name comes from the legacy node").
  - `node.remove` is refused ("an enrolled node keeps its legacy export name").
- `instance.add` takes an optional `name`. The default is "<source name> <parent name>", or the source name at the root (designer call).
- Every candidate passes `RegistryDocument::validate` and the catalog rules. Their texts are the `invalid` messages.

### Affected pools and the rebalance

- An affected source is a subscriber source with an instance in the changed part of the tree:
  - `instance.add` and `node.remove`: the source of that instance.
  - `node.move`, `node.update` with a capacity change, and `node.remove` of an intermediate or WAN node: each source with an instance at or below the node, and each source with an instance below the old or new parent's WAN.
- With no affected source, `propose` does not read routers. Examples: a new WAN node, a new intermediate node, a rename.
- With affected sources, `propose` reads the sources for the candidate registry (the same read as a preview refresh, 30 s), resolves placements, then runs `rebalance` with `RebalanceScope::SourcePool` for each affected source, in source ID order, on the result of the previous one.
### Share objective

- The capacity above an instance is the effective capacity of its nearest ancestor with an independent capacity (a WAN node, or an intermediate node with `fixed` or `wan_percent`), in each direction. An instance without such an ancestor counts as equal to its siblings (designer call).
- The share of an instance is the group demand times its capacity divided by the sum of the capacities of all instances of the group. Download and upload use the larger relative deviation (designer call).
- The rebalance for an affected source keeps pins and their demand, then moves automatic subscribers from instances above their share to instances below it. It stops when no single move reduces the largest deviation. Ties keep the current destination, and then use subscriber ID order, so the result is deterministic.
- This objective replaces the current `RebalanceScope::SourcePool` sweep, which balances only the WAN load and has no group factor. `RebalanceScope::Branch` does not change (designer call).
- New subscribers between edits keep the current resolver rule (least loaded path). The WANs stay balanced. A group split can drift until the next edit or the 2b-3c rebalance (designer call).
- Minimum split: the group total is the sum of the current instance minimums. Each instance gets its share of that total, rounded down to the unit of `NodeMinimum`. The remainder goes to the instance with the largest capacity, so the total is exact. The split runs again after each edit that changes the instances of the group or the capacity above them (designer call on rounding).

### Refusals

- A placement conflict anywhere before the rebalance refuses an edit with affected sources ("resolve the placement conflicts first"). Such conflicts already stop publication. Edits without affected sources stay possible (designer call).
- An aggregate conflict after the rebalance (`MinimumExceedsCeiling`, `FloorExceedsCeiling`, `MinimumBelowDescendants`) refuses the edit with the conflict text, because a committed revision with such a conflict stops publication after activation. The operator orders the build steps so that each step is valid, for example the WAN2 instance of a group before more groups move below WAN1 (designer call).
- The rebalance rule "rebalance scope mixes bounded and unbounded destinations" refuses the edit with that text.
- The coordinator gets one new method that returns the authorized reconciliation for a given registry, without publication. The preview refresh keeps its current path.

### Proposal content

- `edit`: the edit as received.
- `nodes`: the candidate tree, `[{id, name, parent, kind, capacity}]`, in tree order. Capacity uses the wire form above.
- `moves`: `[{subscriber_id, label, from, to}]` for online subscribers whose destination changes. `label` is the preview subscriber label when known.
- `cleared`: the IDs of offline records that lose their assignment.
- `sources`: the affected source IDs.
- `new_records`: the number of new subscriber records that the reconciliation adds, and `assigned`: the number of online subscribers whose saved assignment the candidate sets or changes outside `moves`.

`confirm` commits the full reconciled candidate: the edit, the new subscriber records, and the assignments of all online subscribers. The automatic pipeline commits the same data after activation. The proposal shows these counts, so the commit equals what the operator saw (designer call).
A move `from` is the saved assignment, or the destination that the balancer gives before the edit when no assignment is saved.

## Tree data (new operation)

| Operation | Args | Result |
|---|---|---|
| `topology.tree` | none | `{revision, nodes: [{id, name, parent, kind, source_id, capacity, alias}], sources: [{id, name, kind}]}` from the registry catalog, without a router read |

The Builder Tree and the parent picker use `topology.tree`, so empty instances and a preview in the conflicts state do not hide nodes. Preview counts and rates join the tree by node ID when the preview has them (designer call).

## TUI

- Builder Inventory gets a selection (Up and Down while the Inventory panel has focus). The selected line can be a subscriber source or a WAN candidate.
- Actions on the Inventory selection:
  - A WAN candidate opens the WAN form: name (the interface name by default), down Mbps, up Mbps, factor percent (100 by default).
  - A subscriber source opens the parent picker for `instance.add`.
- Actions on the Tree selection: new intermediate node (form: name, capacity), move (parent picker), promote (parent becomes the grandparent), demote (parent picker limited to siblings), edit (name and capacity form), remove (proposal first).
- The parent picker lists "root" and each valid parent in tree order.
- Every action sends `topology.edit.propose`. The proposal popup shows the tree change, the number of moves, the first moves (subscriber, from, to), the cleared count, and the keys: `Enter` confirms, `Esc` cancels (sends `topology.edit.cancel`).
- A `conflict` answer shows "changed by another operator, try again" and reloads. An `invalid` answer shows the rule text.
- After a confirm, the TUI reloads status, preview, and policies.
- Forms and pickers own all keys while open, as the plan dialogs do. All dialogs fit 80 columns.

## Verification

- Edit rule tests for each edit kind: success, each refusal text, ID stability on move, capacity mode limits, WAN root rule, cycle refusal, duplicate instance under one parent.
- Remove tests: children, pin, last instance, cleared offline assignments, WAN source removal.
- Rebalance tests:
  - Equal WANs, one group of 10 subscribers with equal plans: `instance.add` below WAN2 gives 5 and 5.
  - A 2:1 capacity ratio gives a 2:1 split within one subscriber.
  - Two groups with different factors: each group splits in the capacity ratio, and the WAN totals follow the ratio.
  - Pins stay and count in their instance share.
  - `node.remove` moves its subscribers to the other instance.
  - An edit without an affected source does not read routers.
- Minimum tests: a group minimum of 30 over equal WANs gives 15 and 15; over 2:1 gives 20 and 10; the total stays exact with a remainder.
- Proposal tests: confirm commits the kept candidate; confirm after a commit by another operator gives `conflict`; confirm after expiry gives `conflict`; a new propose replaces the old one; cancel drops it.
- Dispatch tests for the three operations.
- TUI tests: Inventory selection, WAN form defaults and validation, parent picker entries, proposal popup and keys, conflict message, 80-column layouts.
- Full suite, format, Clippy with warnings denied, and whitespace checks.

## Left to the builder

- Exact keys, within the global key rules.
- Node and source ID format: stable, unique while present, and made with the existing ID helpers.
- Line format of the proposal popup and the number of moves it lists.
