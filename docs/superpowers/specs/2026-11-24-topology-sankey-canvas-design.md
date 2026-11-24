# Topology Sankey canvas — design

Date: 2026-11-24. Status: approved by the owner on 2026-11-24 (with 7.2 and 7.2a).
Owner decisions: this conversation, 2026-11-24. Builder: Claude Code (the owner moved this
UI work from the Codex lane to Claude).

## 1. Intent

The owner does not accept the current Topology canvas. The canvas shows small cards on a
large empty dot grid, thin stroked lines, and toolbars with icons only.

The owner wants the network tree to look like the owner Sankey reference:
`docs/frontend/prototypes/topology-studio-v2/04-owner-sankey-reference.png` and the image
of 2026-11-24 in this conversation (same look). Columns go from left to right. Wide filled
ribbons connect the cards. The ribbon width shows real traffic. Each WAN has its own color.

The Sankey becomes the main Topology canvas. It is not a second view.

## 2. Goals

1. The Sankey is the main canvas on `/topology`, on an infinite canvas with pan and zoom.
2. Ribbon width is live download throughput. The value is the exact sum of the host rates,
   not an estimate and not a percentage of a total.
3. Each WAN is a separate stream with its own color. The stream stays separate through
   every column, from the WAN card to the last column.
4. At each card, the sum of the incoming streams is equal to the sum of the outgoing streams.
5. The page layout follows Stitch variant C plus the icon rail of variant A (see section 6).
6. The canvas has no large empty areas. "Fit" shows all columns at a readable size.

## 3. Non-goals

- No router CPU, drop rate, queue count, or other value that has no data source.
- No new backend data source. The design uses the host sample and the routing state that
  the service already writes.
- No change to shaping, placement, or routing decisions. The canvas only shows them.
- No change to the Studio router modal, the save flow, or the List, Assignments, and
  Changes views, except the toolbar that opens them.

## 4. Anti-goals

The work fails, even if it operates, if one of these occurs:

- The canvas shows an invented or estimated value as a live rate.
- A ribbon width does not agree with the numbers on its two cards.
- Ribbons go under cards, or a legend covers a card.
- A WAN stream changes color or merges with another WAN stream between two columns.
- An operator cannot do a task that the old canvas permitted, because no other path
  exists for it.

## 5. Constraints

- Stack: Leptos (Rust, Wasm) in `crates/web-ui`. Use the current components, CSS, and the
  IBM Plex fonts. Add no JavaScript library. Add no Rust crate for the layout.
- Do not show a value without a data source. The 2026-11-24 owner rule
  (`studio-sankey-owner-reference`) stays in force.
- The design system rules in the Stitch project "LQoSync v3 — Topology Studio" (DESIGN.md)
  stay in force: status as word + icon + color, accessible names, focus return, visible
  focus rings, reduced motion. One exception: the canvas no longer has free card drag or
  connector drag (see 7.4).
- Every web UI task applies Vercel frontend-design and Google Modern Web Guidance
  (`frontend-ui-skills`).
- Codex shares the repository. Use a separate branch and worktree. Stage explicit paths
  only. Record the lane change in `brain/HANDOFF.md`.

## 6. Approaches

### 6.1 Page layout (owner picked C + rail)

Stitch project "LQoSync v3 — Topology Studio" (`projects/9214741854721550458`) holds the
base screen and three variants:

| Variant | Layout | Trade-off |
|---|---|---|
| A | Icon rail for the app nav. Detail drawer on the right. | More room for details. The drawer is a permanent column, which DESIGN.md does not permit. |
| B | Inspector panel at the bottom. | The panel takes about one third of the canvas height. |
| C | Canvas only. A small popover opens near the selected card. "Open full settings" opens the current card modal. | Largest canvas. Uses the current modal. Popover can cover near cards. |

The owner picked **C with the icon rail of A**. Reason: the largest canvas, and the current
card modal stays the place for edits.

### 6.2 Sankey implementation

| Approach | Trade-off |
|---|---|
| **Own layout function in Rust** in `canvas_layout.rs`, rendered as SVG | No new dependency. Unit tests can check the geometry. The project already ports its canvas geometry to Rust. |
| A JavaScript Sankey library through Wasm interop | Adds a dependency and a second render path. The constraints do not permit it. |

Decision (builder call): own Rust layout, SVG render. Only one approach fits the
constraints.

### 6.3 Ribbon value (owner decision)

Options were: share of subscribers per WAN, configured WAN allocation percentage, or the
exact sum of the host rates per WAN. The owner picked **the exact sum per host per WAN,
from the parent card down to the children**.

## 7. Design

### 7.1 Data: per-WAN flows (backend)

Today `GET /api/v1/topology/throughput` (`crates/web-gateway/src/throughput.rs`) joins the
host sample (`subscriber-telemetry.json`) to the placed subscribers of `topology.json` by
IP address. It gives `down_bps` and `up_bps` for each node (subtree sum) and for each
parent→child card pair. It does not know the WAN of a host.

The routing WAN of each subscriber is in the service:
`RoutingState.assigned` (subscriber ID → WAN node ID), read in
`src/topology_manager/web_view.rs` `card_balance`. The view does not send it for each
subscriber.

Changes:

1. Service: add the routing WAN to each subscriber in the topology view.
   Suggested field: `Active.wan: Option<String>` (WAN node ID), with `#[serde(default)]`.
2. Contract: add `wan: Option<String>` to `FlowThroughput`, with `#[serde(default)]`.
   `None` means "no routing WAN known".
3. Gateway: in `compute`, key each sum by (card pair, WAN). A host adds its rate to every
   card pair on its path, under the WAN of its subscriber.
4. Gateway: add the WAN→first-card streams. For each host, add its rate to the pair
   (WAN card, top card of the host path) under its WAN. These pairs come from routing, not
   from the shaping tree parent.

Result: every ribbon segment is `(from card, to card, wan, down_bps)`. The values are exact
sums, so in = out at each card by construction. Hosts that have no subscriber stay in
`unmapped_hosts` and are in no ribbon.

### 7.2 Columns

The layout calculates the columns. No column has a fixed card kind.

- The graph has two types of edges: the tree edges (parent → child) and the routing edges
  (WAN card → first card of the host path, see 7.1).
- The column of a card is the length of the longest path from a card without parents to
  that card. The first column is 0.
- Any card kind can be in any column. Example (owner, 2026-11-24): core router → WAN →
  LibreQoS → distro router → nodes → subscribers gives six columns. Without a LibreQoS card,
  the same network gives five columns.
- The color of a stream is the WAN of its hosts on every edge of the host path. This
  includes the edges before the WAN card.
- Column headers show the card kinds in that column, for example "WAN" or
  "Router · Group".
- The last column is the **Service tier**: one card for each source type (PPPoE,
  IPoE / DHCP, Hotspot). Its value is the sum of its groups. It has no settings.
- If a cycle occurs, the layout stops the path at the second visit and shows a warning.
  The tree should not have cycles.

### 7.2a Display-only cards (owner decision)

The owner wants cards such as "core router" and "LibreQoS" in the chain. These cards are
for display only. They are not in the shaping tree (`network.json`).

- Use the current `tree_excluded` mechanism. `publication.rs` does not export a
  tree-excluded node. Its children get the nearest exported ancestor as parent.
- Today `Edit::NodeTreeExclude` accepts only a router node (`edits.rs`, check
  `router_id.is_none()`). Change the rule: accept every `intermediate` node. WAN and group
  nodes stay not excludable.
- A display-only card has no capacity on the canvas. It shows its throughput (the sum of
  its subtree), and the label "Display only · not shaped".
- The builder must make sure that a WAN node can have a display-only parent (example:
  core router → WAN). If the tree rules do not permit this, the builder reports it to the
  owner before a change to those rules.

### 7.3 Layout rules

- Card height is proportional to the card throughput, with a minimum height for the card
  text. One scale for the full canvas.
- Inside each card, the streams stack along the left and right edges in WAN order. Streams
  never overlap.
- The order of the cards in a column keeps ribbon crossings to a minimum. The builder can
  use a simple barycenter sort. The order is stable between two refreshes, so cards do not jump.
- A ribbon is a filled SVG path between two cubic curves, not a stroked line.
- Each WAN has one color for the full canvas. Backup or bypass WANs use amber.
- A card with zero live traffic keeps the minimum height. Its streams are hairlines.
- A card without traffic data shows "No live data", not "0 Mbps".

### 7.4 Interactions

Keep:

- Pan, zoom, "Fit view", "100%", and the minimap.
- Click on a card: the popover opens near the card. The popover shows the card name, its
  live rate, the stream for each WAN (Mbps and subscribers), and the button
  "Open full settings". The button opens the current card modal.
- Click on a ribbon: the popover shows "WAN → From → To", Mbps down and up, and hosts.
- Hover on a ribbon or card: the related streams get the highlight, other streams dim.
- Keyboard: Tab moves between cards. Enter opens the popover. Escape closes it and returns
  focus to the card.
- Routing warning for a card while the routing writer is off.

Remove:

- Free card drag and the saved card `position`. The layout is automatic.
- The layout direction setting. The direction is always left to right.
- Connector drag to set a parent. Set the parent in the card modal. The card modal already
  has a keyboard path for this.
- "Remove link" on a line. Do this in the card modal.

The contract field `Node.position` stays, so old data does not cause errors. The canvas
does not read it.

### 7.5 Page layout (C + rail)

- App nav: a narrow icon rail with tooltips and accessible names. A button expands it to
  the full nav. The page keeps the choice in local storage. This changes the shared app
  shell for all pages.
- Toolbar: one row. All controls have text labels. The row contains:
  - the page title,
  - the current view tabs with text labels: "Sankey" (the current "Builder" tab),
    "Assignments", "Policies", "Changes",
  - the chips "Saved rev N", "Applied rev N", and "Live · Ns ago" (or "Partial",
    "Subscribers, not live"),
  - the WAN legend: one swatch and one name for each WAN,
  - the current preview controls (for example "Load saved topology"). There is no new
    "Save" button. Saves stay in the card modal and the Studio modal (non-goal in 3).
- Column headers at the top of the canvas.
- Zoom cluster at the bottom left. Minimap at the bottom right.
- No KPI tiles. No permanent side panel.

### 7.6 States

| State | What the canvas shows |
|---|---|
| Live sample, complete | Ribbons in Mbps. Chip "Live · Ns ago". |
| Live sample, `complete: false` | Ribbons in Mbps. Chip "Partial" with the reason (for example unmapped hosts). |
| Stale or no host sample | Ribbon width from subscriber count. Chip "Subscribers, not live". Cards show subscribers, not Mbps. |
| No routing WAN for a host | The host goes in a grey stream "WAN unknown". |
| Routing writer off | The streams show the assigned routing WAN. A warning says that the routers may not use this path yet. |
| No topology | The current empty state stays. |

## 8. Decided by the owner

- The Sankey is the main canvas. Free drag goes away.
- Claude builds this work, not Codex.
- Ribbon width = exact live throughput per host, summed per WAN, from parent to children.
- Each WAN is a separate colored stream to the last column.
- Page layout: Stitch variant C plus the icon rail of variant A.
- Columns come from the graph, not from fixed card kinds (7.2).
- Core router and LibreQoS cards are display only, not in the shaping tree (7.2a).

## 9. Builder calls (owner can change)

- Own Rust layout and SVG render (6.2).
- The fourth column "Service tier" by source type (7.2).
- Fallback to subscriber count when the sample is stale (7.6).
- Grey "WAN unknown" stream (7.6).
- Field names `Active.wan` and `FlowThroughput.wan` (7.1).
- Barycenter sort for card order (7.3).
- Exact colors for WAN 4 and higher: next colors from a fixed palette.

## 10. Left to the builder

- Card minimum height, column gap, ribbon curve shape, and popover position rules.
- Refresh rate of the throughput fetch (the current fixture writes a sample each second).
- How the minimap draws.
- Test split between unit tests (layout geometry, per-WAN sums) and browser checks.

## 11. References

- Owner reference image: `docs/frontend/prototypes/topology-studio-v2/04-owner-sankey-reference.png`.
- Stitch screens (project `9214741854721550458`): base `c055ac3962c94be88cd883f33d9706b2`,
  variant A `a86f85d187b247dd9ba2cec63fe6ac68`, variant B `1273b097b8ea41d6bf43791b54e3003f`,
  variant C `67f9b8a9b0f44e7a9c1419f64aaf32ec`.
  NOTE: the ribbons in the Stitch screens are not correct geometry. Some text in them has no
  data source (CAKE ISP, overbooking, "Rebalance WAN", utilization). Use the screens for
  page layout and card style only.
- Current canvas: `crates/web-ui/src/topology_canvas.rs`, `canvas_layout.rs`,
  `topology_canvas_model.rs`.
- Throughput endpoint: `crates/web-gateway/src/throughput.rs` on branch
  `claude/topology-studio-backend` (merged into `lqosync3` through
  `integration/topology-studio`).

## 12. Changes after the first deploy (owner, 2026-11-24 evening)

### 12.1 Infinite canvas

The first build used a scroll box. The owner asked for a true infinite canvas.

- The viewport fills the canvas frame. It has no scrollbars and no edge.
- A drag on the empty background pans. The drag starts after 4 px, so a click stays a click.
- The mouse wheel zooms at the cursor (owner decision). A two-finger pinch zooms on touch screens.
- Arrow keys pan. Plus and minus zoom. The key 0 runs "Fit view".
- The dot grid follows the pan and the zoom.
- "Fit view" keeps clear of the floating controls. It runs when the page opens, and again after a
  window resize while the view shows the last fit.

### 12.2 Capacity ribbons with a live band

This change replaces goal 2 and section 6.3. The ribbon width is now the configured capacity.

- A ribbon from a WAN to the next card is the calculated download capacity (`calculated_mbps`,
  else `exported_mbps`) of the node instance below that WAN. This is the share of the WAN that
  the percent or fixed capacity gives.
- A ribbon from a card to its child is the calculated capacity of the child instance. Each WAN
  keeps its own stream to the last column.
- Without any capacity data, the ribbons count subscribers.
- The live download is a bright band inside the ribbon, top-aligned. Its height is live ÷
  capacity of the band, at most the full band. Without a fresh host sample, no band shows and the
  chip says "No live traffic". The capacity ribbons do not change.
- Cards show the capacity, and a "N Mbps live" line when a sample exists.
- A WAN card shows, most important first (owner): the configured capacity of the WAN
  (`1000 Mbps`), the allocated child capacity (`400 Mbps allocated`, the ribbon width), that share
  in percent (`40% allocated`), and the live download when telemetry exists. The card does not
  show the unallocated rest: it is configuration headroom, not free Internet bandwidth.
