---
type: spec
created: 2026-11-19
status: draft
related: ["[[topology-manager-status]]", "[[2026-11-19-topology-builder-edits]]"]
---

# Explicit rebalance and subscriber pins design (Phase 2b-3c)

## Intent

Builder edits rebalance the affected groups when they commit. Between edits, the resolver places new subscribers on the least loaded path, so the split of a group across its WAN instances can drift.
The owner wants an explicit rebalance action that splits each group again in the capacity ratio of its WANs, with a preview of the moves before confirm.
The owner also wants subscriber pins: one subscriber stays on one WAN.
Application pinning (one application on the WAN with the best quality, with DPI) is a later feature. It needs DPI and routing on the core. It is not in this slice.

## Owner decisions (2026-11-19)

- Explicit rebalance first. Subscriber pins are also in this slice.
- Rebalance actions: one group (the `SourcePool` share objective) and all groups (each group in turn, as Builder edits do).
- The old `Branch` sweep (WAN load only) is not offered, because it can undo the group split.
- A pin is set by WAN in the TUI. LQoSync finds the instance of the subscriber's group below that WAN.
- Unpin returns the subscriber to automatic placement. It stays on its current instance until a later rebalance (Decision 6).
- Pinned subscribers do not move in a rebalance. Their demand counts in the group split.
- Each rebalance, pin, and unpin shows a proposal before confirm and commits in one revision.
- No background or timer rebalance (Decision 5).
- Approach A (below).
- No effect on live shaping before Phase 4 activation.

## Non-goals

- Application pinning, DPI, and routing changes on the core.
- Pins to a node instance chosen directly, and pins of accounts without a subscriber record.
- Pin conflicts from later Builder edits beyond the existing remove refusal.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. New edit kinds in the existing propose and confirm (chosen) | `rebalance` and `subscriber.pin` edit kinds. The runner rebalance chain does the split | Reuses the proposal, popup, conflict handling, and expiry. One proposal slot, so a pin replaces a pending Builder proposal |
| B. Separate operations | `subscriber.pin` commits directly, like `subscriber.plan`. A separate rebalance propose and confirm | No move preview for a pin, against the owner decision. A second proposal flow |
| C. Rebalance that commits directly | One operation rebalances and commits | No preview of moves. Against the owner decision |

## Edit kinds (new)

| `kind` | Fields | Effect |
|---|---|---|
| `rebalance` | `source_id` or `null` | Marks the source, or every subscriber source with two or more instances, as affected. No structural change |
| `subscriber.pin` | `identity` (`router_id`, `source_id`, `kind`, `account_ref`), `wan_node_id` or `null` | Sets or clears the pin of the subscriber record with that identity |

### Rules

- `rebalance` with a `source_id`: the source must be a subscriber source with at least two instances ("the group has only one instance"). With `null`: sources with fewer than two instances are skipped; when none is left, the edit is refused ("no group has two or more instances").
- `subscriber.pin` with a WAN:
  - The record must exist ("confirm an edit or rebalance first, so the account gets a record").
  - The record must have a saved assignment ("the subscriber has no placement yet"). The source of that assignment node is the group of the subscriber.
  - The WAN node must exist and be a WAN node ("the node is not a WAN").
  - The group must have exactly one instance below that WAN ("the group has no instance below this WAN", or "the group has more than one instance below this WAN").
  - The edit sets `pin` and `assignment` to that instance and marks the group as affected, so other subscribers balance around the pin.
- `subscriber.pin` with `null`: the record must exist and have a pin ("the subscriber has no pin"). The edit clears `pin`, keeps `assignment`, and marks no source, so no router read and no move.
- A pin needs an online subscriber: the pin edit reads the sources, and an identity that is not in the read is refused ("the subscriber must be online to pin"). So the resolver checks each new pin against the live eligibility (designer call: an offline pin cannot be checked and could block publication when the subscriber returns).
- Unpin works for online and offline records. `subscriber.pin` with `null` also accepts `subscriber_id` instead of `identity`, for records without a preview row (designer call).
- A rebalance also clears the saved assignment of offline, unpinned records in each rebalanced group, so the balancer places them on the least loaded path when they return. They show in `cleared` (designer call: otherwise they return to their old instance and the split drifts again at once).
- Rebalance refusals stay as in Phase 2b-3b: placement conflicts, aggregate conflicts, and mixed bounded and unbounded destinations.
- A pin that the resolver finds ineligible gives `invalid` with the conflict text.

## TUI

- Builder: a key on a selected Inventory source proposes `rebalance` for that source. A key proposes `rebalance` for all groups. The keys are the builder's choice; `R`, `S`, and `T` are global and stay free.
- Assignments: a key on the selected subscriber opens a WAN picker. Entries: each WAN node from `topology.tree`, and "unpin" when the subscriber has a pin. Enter proposes `subscriber.pin`. Esc cancels.
- The Assignments pin column shows the name of the pinned node instead of "pinned".
- `topology.tree` adds `pins: [{subscriber_id, node_id, online}]` for every record with a pin. Assignments lists offline pinned records after the online rows, marked "offline", with unpin as their only action. So the operator can clear a pin that blocks `node.remove` (designer call).
- The proposal popup names the change: "rebalance <group>", "rebalance all groups", "pin <subscriber> to <WAN>", "unpin <subscriber>".

## Verification

- Edit rule tests: each refusal text; pin sets pin and assignment to the right instance; unpin keeps the assignment and marks no source; `rebalance null` skips single-instance groups.
- Service tests: a pin of an offline subscriber is refused; unpin by `subscriber_id` of an offline record works; a rebalance clears offline unpinned assignments of the group and lists them in `cleared`; a drifted group (all new subscribers on one WAN) gets back to the capacity split after `rebalance`; a pinned subscriber does not move and its demand counts; unpin reads no router and moves nothing; pin of an account without a record is refused.
- TUI tests: the rebalance keys propose the right edits; the WAN picker lists WAN nodes and "unpin" only for a pinned subscriber; the popup texts; the pin column; 80-column layouts.
- Full suite, format, Clippy with warnings denied, and whitespace checks.

## Left to the builder

- Exact keys.
- Popup line format.
