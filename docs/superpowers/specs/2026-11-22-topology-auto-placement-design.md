# Topology automatic placement: design

Date: 2026-11-22. Status: design for implementation. Plan: `docs/superpowers/plans/2026-11-22-topology-auto-placement.md`.

## Intent

The operator designs the network tree in the Topology canvas. The operator has full control of the tree.
The topology manager in the service places each subscriber in that tree on each cycle.
The canvas is the editor and the display. Placement does not need an open browser.

A subscriber must not lose shaping because the operator did not place its source in the tree.
A subscriber must stay in the branch of its own access router.

## Placement rules

The resolver applies these rules to each subscriber, in this sequence. The first rule that matches gives the placement.

| ID | Condition | Placement |
|----|-----------|-----------|
| R1 | The tree has no nodes. | No parent (flat). |
| R2 | The operator placed the source (PPP profile, DHCP server, or Hotspot) of the subscriber. | The placed node, as today. |
| R3 | The router of the subscriber has a router node. The source is not placed. | The immediate child nodes of the router node. If the router node has no eligible child, the router node. |
| R4 | The router of the subscriber has no router node. The router has its own WAN node. | The WAN nodes of that router. |
| R5 | The router has no router node and no own WAN node. | The root nodes of the tree. |
| R6 | R3, R4, or R5 gives no eligible node. | No parent (flat). |

These constraints apply to all rules:

- **Family rule.** A subscriber never goes into the branch of a router node of a different router. The resolver removes these nodes from each eligible set. The edit validation refuses a source placement in the branch of a different router node.
- **Stability rule.** A saved `assignment` stays while its node is eligible. The resolver puts new subscribers on the new nodes. The `Rebalance` edit moves the subscribers that have an assignment. A tree edit that makes a saved node ineligible moves the affected subscribers one time.
- **Balance rule.** If an eligible set has more than one node, the resolver uses the current capacity allocation. The resolver already does this for each placement (`resolver.rs`, `Capacity` score).
- **Integrity rule.** A subscriber with accounts on two routers gets the intersection of the two eligible sets. If the intersection is empty, the resolver records a `NoEligibleTarget` conflict.

## Decisions

| Decision | Source |
|----------|--------|
| A new topology starts empty on the first run. | Owner, 2026-11-21. The owner confirmed it again on 2026-11-22. |
| An empty tree gives a flat output. A saved tree is the authority. | Owner, 2026-11-22. |
| A node can represent an access router (router node). Its subscribers stay in its branch. | Owner, 2026-11-22. |
| A router without a router node goes under the WAN node. | Owner, 2026-11-22. |
| With more than one WAN, use the own WAN of the router first, then capacity balance. | Proposal. The owner did not object. Confirm before the release. |
| Saved assignments stay. New subscribers go to new nodes. | Builder call. This is the current resolver behavior. Owner question is open. |
| An explicit `FallbackPolicy` keeps priority over R3–R5. The family rule filters it. | Builder call. It keeps saved policies unchanged. |
| A router node is an `Intermediate` or `SubscriberGroup` node. A WAN node cannot be a router node. | Builder call. A WAN node already has a router through its WAN source. |
| One router has a maximum of one router node. | Builder call. Two nodes would make R3 ambiguous. |

## Approaches

1. **Extend the resolver eligibility (selected).** Calculate the eligible nodes for each router one time for each index build. Use them where the current fallback path applies. This reuses the resolver, the sticky assignment, the allocation, and the publication path.
2. **Generate hidden group nodes for unplaced sources (rejected).** This adds nodes that the operator did not create. It changes the tree of the operator and the `network.json` names.
3. **Place subscribers in the frontend (rejected).** Placement must operate without a browser. Two placement engines can disagree.

## Data model

- `CatalogNode.router_id: Option<String>`. Registry schema 6. The value must be the ID of a `CatalogRouter`.
- Migration 5 to 6 adds no data. The store makes a backup before the first schema-6 write, as for schema 5.
- `Placement.target_id: Option<String>`. `None` means no parent. The CSV row gets an empty `Parent Node`.
- Web contract `Node` gets `router_id: Option<String>` and `auto_members: Option<u64>`.
- Web contract `Source` gets `auto_parent: Option<String>`. `None` means no parent.
- `BuilderEdit::NodeRouter { node_id: String, router_id: Option<String> }`.

## Frontend

The canvas stays in Leptos 0.8.23 and uses the current components, tokens, and modal pattern.

- The node modal gets an **Access router** select: `None` and each discovered router without a router node.
- A router node shows a router badge. A node with automatic subscribers shows a count chip, for example `+12 auto`.
- The Inventory shows each unplaced source with its automatic parent, for example `Auto: WAN1`.
- A refused edit shows the service sentence in the modal. The draft stays.

Apply these guides:

- Vercel Web Interface Guidelines: keyboard access, visible focus, focus return after Escape, labels on each control, and no layout shift on status change.
- Google web.dev Core Web Vitals: interaction to next paint (INP) of 200 ms or less, cumulative layout shift (CLS) of 0.1 or less.
- The `ui-ux` skill: dashboard density, contrast in light and dark themes, and widths of 375 px and 768 px.

## Constraints

- Do not deploy. Do not change the live registry, the routers, the services, or the writer ownership.
- Do not change the legacy producer. Canvas plan Tasks 7 and 8 control the ownership transfer.
- The resolver calculates each eligible set one time for each router for each cycle. The cost of each subscriber is O(size of its eligible set).
- Do not add a dependency.
- Keep `Tracker::write` as the only publication boundary.

## Non-goals

- Hotspot nodes for each hotspot profile. The Hotspot source stays one source for each router.
- MikroTik route control. A LibreQoS parent does not steer traffic to a WAN.
- Quality-aware allocation and strict-pin failure behavior.

## Anti-goals

The design fails if any of these occur, also when the tests pass:

- A subscriber goes into the branch of a different router.
- A subscriber loses shaping only because its source is not placed.
- Subscribers move on each cycle without a tree edit or a `Rebalance` edit.
- Placement stops when no browser is open.

## Acceptance

- Resolver tests prove R1–R6, the family rule, the stability rule, and the integrity rule.
- An old schema-5 registry loads, migrates, and keeps all IDs and data.
- An empty tree publishes a valid flat pair through the scratch publisher fixture.
- The canvas browser check passes for the router select, the chips, the Inventory status, and a refused edit.
- `tools/frontend/check-all.sh` and the workspace tests pass.
