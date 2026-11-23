---
type: spec
created: 2026-11-23
status: approved-pending
related: ["2026-11-22-live-source-canvas-design", "2026-11-22-topology-auto-placement-design"]
---

# Canvas flows, layouts, and routing balance design

## Intent

The Topology canvas must stay small and readable when a card has more than one WAN. Today each WAN parent makes a copy of the card (an instance), and the copies of its child cards repeat below each copy. The owner wants one card for each node, with one flow line from each WAN, as in a Sankey diagram. The owner also wants layout directions, removable links, and subscriber labels that show the WAN of each subscriber.

The MikroTik routing (the `via-WAN` address lists) must balance all subscribers over the WANs, also when a card has no WAN link on the canvas.

## Owner decisions (2026-11-23)

1. One card for each node on the canvas, whatever the number of WAN parents. The child cards below it are also one card each. The canvas must not grow with the number of WANs ("dynamic").
2. Flow lines in the Sankey style between the cards. The cards stay cards.
3. A link can be removed. When the only WAN link of a card is removed, the card becomes a top-level card (a parent), and its child cards stay below it.
4. All subscribers are load balanced over the WANs through the address lists, also when their card has no WAN link. A card without a WAN link is balanced over all available WANs, by the load balance setting (Count, Capacity, or Quality).
5. A card with WAN links is balanced only over the WANs of its links.
6. The subscribers show as leaf labels with the WAN of their balance.
7. Layout directions: centered (top-down), reverse (bottom-up), left-to-right, and right-to-left.
8. Approach A (below).
9. When every WAN link of a card is down, routing moves its subscribers to a WAN that is up. Shaping stays below the linked WAN. The subscribers return by the failback rules (`auto_failback`). When all WANs are down, nobody moves (today's rule).
10. A fixed capacity on a card with several instances is the total of the card. The service splits it over the instances in the ratio of the capacities of their WANs.

## Goals

- The number of cards does not depend on the number of WAN parents.
- The line width shows how many subscribers go through each WAN.
- Every subscriber with an IPv4 address has exactly one `via-WAN` entry when the routing list writer is on.
- A WAN assignment does not change from cycle to cycle without a cause (a new subscriber, a WAN failure, a failback, a manual rebalance, or a link change).

## Non-goals

- No change of the registry data model: instances stay; the export of `network.json` does not change.
- No traffic measurement per WAN on the lines (only subscriber counts).
- No change of the WAN failover rules (WAN monitor, `auto_failback`).

## Anti-goals

- A card copy for each WAN parent.
- A subscriber that moves between WANs in each cycle (connections break when the route changes).
- A subscriber in two `via-` lists, or in a list of a WAN that is down.
- A canvas that is slow or unreadable with 5,000 subscribers.

## Constraints

- The export needs one parent for each node: instances stay in the registry and in `network.json`.
- Leptos 0.8 CSR, SVG for lines. No new dependency.
- The routing list writer rules stay: owned `via-` entries only, add before remove, the removal hold.
- Persisted text in ASD-STE100.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. Merge on the canvas, routing balance in the backend (chosen) | The canvas shows the instances of one source or one router as one card with one line for each WAN parent. A new routing balance gives a WAN to each subscriber of a card without a WAN link | No schema migration; export and aggregation do not change. New work is the merge in the view and one routing rule |
| B. One node with many parents in the registry | The registry becomes a graph (schema 10); the export splits it into instances | Cleaner model, but it changes the resolver, export, aggregation, and migration: a high shaping risk |
| C. Canvas only | Merge, layouts, and link removal; no routing change | Smallest, but subscribers of a card without a WAN link get no list, against decision 4 |

The owner chose A on 2026-11-23.

## Design

### 1. One card for each node (view merge)

- The cards of one group source are one card: key = the source ID. The cards of one router (router-node copies below different WANs) are one card: key = the router ID.
- An ordinary node without a source or router keeps its own card (key = node ID).
- The merged card has one incoming line for each distinct parent card. Duplicate lines between two merged cards are one line.
- The card shows the total members of its instances.
- Selecting the merged card opens one settings dialog. Name and location apply to every instance. A fixed capacity is the total of the card (decision 10): the service splits it over the instances in the ratio of their WAN capacities, as `split_minimum` does for minimums.
- The service web view gives each node its merge key (`card_id`), so that the browser does not guess it.

### 2. Flow lines

- Line width: the number of subscribers of the child card that go through that parent, scaled between 1.5 px and 10 px by the largest flow on the canvas. Without counts, 1.5 px.
- The counts come from the preview (members of each instance) for cards with WAN links, and from the routing balance (section 4) for cards without one.
- Each line has a text equivalent: the selected card lists its flows ("WAN1 15, WAN2 8").

### 3. Removing a link

- A click on a line selects it. The toolbar then shows "Remove link"; the Delete key does the same. Keyboard users remove a link in the card settings.
- The only parent link of a card: the card moves to the top level (`node.move` with no parent). Its child cards stay below it.
- One of several WAN links: the instance below that WAN is removed. Builder call: for a router card, the router copy below that WAN is removed together with the group instances below it; the groups stay below the other copies, and their subscribers go to those instances by the allocation mode. The service gets one edit for this (a new `node.instance_remove` or an extension of `node.remove`).
- Builder call: the link between a group card and its own router card cannot be removed (the router branch rule). The canvas does not offer it.
- The removal of the last instance of a source is refused by the service today. The canvas never offers it.

### 4. Routing balance

- A subscriber whose placement has a WAN above it keeps today's rule: the list of that WAN.
- Builder call: the subscribers of the routing balance are the subscribers in the prepared pair (shaped this cycle), also flat and fallback placements. A subscriber of an excluded source, or with a placement conflict, is not in the pair and gets no entry (the writer removal hold applies).
- A subscriber whose placement has a WAN above it, and all of whose linked WANs are down, gets a routing WAN from the balance among the WANs that are up (decision 9).
- A subscriber whose placement has no WAN above it (a card without a WAN link, or a flat placement) gets a routing WAN from the balance:
  - Candidates: every WAN node whose WAN monitor state allows new subscribers (the failover rules of the WAN allocation design).
  - Mode: the allocation mode of the source, else the topology default: Count, Capacity (by WAN capacity), or Quality (capacity × quality score).
  - Builder call: the balance compares the total load of each WAN, linked and unlinked subscribers together, so that the WANs even out.
  - Stability: a subscriber keeps its routing WAN while that WAN is a candidate. A new subscriber goes to the best WAN by the mode. A WAN that fails moves its subscribers; failback follows `auto_failback`. Builder call: one global "Rebalance routing" action in Topology settings shows how many subscribers change WAN, then applies after a confirmation (a route change breaks open connections).
  - Builder call: the routing assignments are runtime state in `topology-manager/routing-balance.json` (subscriber ID → WAN node ID), not a registry revision.
- The writer then writes one entry for each IPv4 address in the list of its routing WAN.
- The view gives, for each card without a WAN link, the count of its subscribers on each WAN.
- Builder call: a card without a WAN link gets no line from the WAN cards; its balance shows in its leaf-label line only. Only shaping links are lines.

### 5. Leaf labels

- Below a group card, one compact line: the count on each WAN ("WAN1 15, WAN2 8"), with a WAN colour mark. This line is always visible.
- "Show subscribers" expands up to 50 labels (name and WAN mark) below the card, with "+N more" that opens the Assignments tab filtered on the group. The expansion reserves space; the layout moves the cards below it, and nothing overlaps.
- Builder call: the expansion state is per browser tab and is not saved.

### 6. Layout directions

- A selector in the canvas toolbar: Top-down (centered, default), Bottom-up (reverse), Left-to-right, Right-to-left.
- The direction changes only the automatic positions and the line geometry. Ports follow the direction (top/bottom for vertical, left/right for horizontal).
- Builder call: the direction is saved in `canvas-layout.json` with the positions. Manual positions are kept for each direction, so a switch back restores them.

## Known limits and risks

- A card without a WAN link has no WAN node above it in `network.json`, so no WAN ceiling limits its shaping; each subscriber rate stays correct.
- While the routing list writer is off, the balance is visible on the canvas but MikroTik does not follow it (the existing warning stays).
- The merge key of an existing registry: group instances share a source; router copies share a router ID. A node made by hand without either keeps its own card.

## Left to the builder

- The exact line scaling, colours of the WAN marks (from the theme tokens), and label layout.
- The structure of `routing-balance.json` and its migration from no file (first run: assign by the mode, in subscriber ID order).
- The split into tasks; backend first is recommended (merge key, balance, counts), then the canvas.

## Verification

- Unit tests: the merge key groups instances of one source and copies of one router; the balance keeps a subscriber on its WAN, moves it only when the WAN fails, and divides new subscribers by each mode; every IPv4 address is in exactly one list; removing the only link moves the card to the top level and keeps its children; removing one of two links removes one instance.
- Native tests of the layout engine for the four directions: no overlap, ports on the correct sides.
- Browser checks: one card for a source with two WAN parents, line widths, link removal by mouse and keyboard, leaf label expansion, the four directions, light and dark themes, a narrow screen.
- Production: FSEHub-Distro with WAN1 and WAN2 links shows as one router card with two lines; its groups show once.
