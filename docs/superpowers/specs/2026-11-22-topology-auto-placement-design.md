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

## Owner decisions, round 2 (2026-11-22)

These decisions apply to later phases. They do not change the implemented rules R1–R6.

### WAN choice and routing

- Each subscriber uses one WAN. A MikroTik address-list on the core router selects that WAN (policy-based routing).
- There is one address-list for each WAN. The members are the client addresses. Example: LitoYambi, `10.100.0.234/32`.
- LQoSync decides the WAN of each client. When LQoSync publishes a change, it also updates the address-lists on the configured core router.
- At this time, the operator keeps the address-lists manually.
- The core or edge router holds the WANs. The distribution router or access concentrator holds the subscribers (DHCP, IPoE, PPPoE, Hotspot).
- The configured capacity of each WAN is the ceiling of that WAN.
- The core router writer is a later phase. It needs a separate scope approval before implementation, because it writes to MikroTik.
- List name: `lqosync-wan-<WAN name>`, one list for each WAN.
- The writer changes only the entries that have a comment that starts with `lqosync:`. It does not change manual entries.
- The operator makes the mangle rules and the routes one time. The writer keeps only the list members.
- Before the first write, the service reads the manual lists and keeps those WAN choices. No client changes its WAN at the cutover.
- When the address of a PPPoE or DHCP client changes, the writer changes the address in the list. The client keeps its WAN.
- When a write to the core router fails, the service does not publish the placement change of those clients. It tries again in the next cycle.

### Allocation

- The operator selects the allocation mode. The system must stay flexible.
- Modes: equal subscriber count (default), capacity, and quality.
- Quality mode: the share of each WAN is proportional to its capacity multiplied by its quality score (0 to 1). Example: WAN1 1000 Mbps with quality 1.0 and WAN2 300 Mbps with quality 0.5 give 1000 : 150, about 87% : 13%.
- When the quality data is stale, quality mode uses capacity mode.
- The saved tree in the Topology Manager is the authority for the structure.

### WAN monitoring and failover

- The service must monitor each WAN.
- These conditions make a WAN unavailable: bad ping quality, a disabled interface, and a WAN that does not return in time.
- Probe: one ping every 10 seconds through the WAN itself (with its routing mark), to two targets.
- A disabled or not running interface makes the WAN unavailable at once.
- Bad quality: loss of 50% or more, or latency more than 3 times the normal value, for 1 minute.
- The subscribers move only when the problem continues for more than 2 minutes. A short failure moves no subscriber.
- When a WAN is unavailable, its subscribers move to the other eligible WAN nodes.
- Failback: the subscribers return after the WAN is stable for 10 minutes.
- An on/off setting controls automatic rebalance. On: failback and rebalance occur automatically. Off: only the manual Rebalance moves subscribers.
- After a failover, the ceiling of the WAN that stays available applies to all its subscribers. The owner accepts this.
- Each node keeps its own bandwidth control: plan capacity multiplied by the number of clients, multiplied by the factor.

### Stability (S1–S4 = A)

- A saved assignment stays. A subscriber moves only for a tree change, a manual Rebalance, or a failover.
- A new child node takes new subscribers only. The other subscribers stay on their nodes.
- When the operator adds a router node, its automatic subscribers move one time, together.
- An assignment stays until the service or the operator removes the subscriber record. This is also true when the subscriber is offline.

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
