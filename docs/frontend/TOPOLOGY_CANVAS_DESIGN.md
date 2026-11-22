# Topology Manager canvas design

Status: finalized product direction. Implementation remains pending.
Updated: 2026-11-21T23:46:58+08:00 (Asia/Manila).

This document defines the canvas and automatic apply workflow requested by the owner.
It supersedes the mandatory operator preview, review, confirmation, and per-edit approval steps in earlier Topology web designs.
Earlier documents still describe current code until implementation changes it.
This document does not claim deployment or authorize changes to live routers.

The [integration map](TOPOLOGY_CANVAS_INTEGRATION.md) links these requirements to current frontend and backend owners.
The [implementation and transition plan](../superpowers/plans/2026-11-22-topology-canvas-integration.md) defines acceptance gates before legacy retirement.

## Operator experience

Topology Manager uses an infinite canvas inside the current Rust web application.
The canvas supports pan, zoom, selection, node movement, connectors, and modal forms.
A fresh topology starts empty. Discovery adds resources to Inventory only.
Saved policies remain visible. The application must not reset them to create an empty canvas.

The FAB opens Router Inventory and resource actions.
The operator selects routers and discovery categories before a discovery request.
Inventory supports classification by router, interface, and subscriber source.
One router can supply both WAN interfaces and subscribers.
The operator selects resources or drags them from Inventory onto the canvas.

WAN candidates include the selected interfaces, such as `pppoe-out1` and `pppoe-out2`.
An interface name or link state alone does not prove an Internet path.
Subscriber resources include PPP profiles, PPPoE servers, and DHCP servers where discovery provides enough evidence.
PPP profiles and PPPoE servers remain distinct selectors. Unknown membership remains explicit.
The application must not infer an authorized subscriber from an unverified observation.

The canvas supports WANs, Sites, APs, intermediate nodes, subscriber groups, and visual groups.
Subscribers remain collapsed into groups by default. Inspection opens the subscriber list and placement controls.
Node configuration opens in a modal. Fields depend on the node role and source.

## Frontend layout and settings

All Topology settings open from icon FABs inside the canvas.
Do not use a permanent settings sidebar, Details column, or separate Policies page as the required editor.
Keep the application shell and its shared theme control.
The canvas fills the remaining feature area, with a compact title and save/apply status.

Place a vertical floating dock inside the right canvas edge on wide screens.
Keep it above the graph and outside node drag handles.
Each icon opens one modal. Only one modal can be open at a time.
Keep the dock clear of system banners, map attribution, and browser safe areas.

| Icon action | Modal content | Scope |
|---|---|---|
| Router inventory | Router selection, discovery categories, source classification, and selected resources | Observed inventory and source attachment |
| Topology settings | Topology defaults, factors, fallback, and canvas display options where supported | This topology |
| Subscriber allocation | Count/capacity/quality mode, eligible WANs, and supported allocation rules | Selected group or clearly named topology scope |
| Optional preview | Effective tree and export details | Read-only inspection |

Do not add settings in global Settings that duplicate these editors.
Router credentials remain under Routers. LibreQoS connection settings remain under LibreQoS.
Links can open those owning screens, but must not duplicate their configuration fields.

Selecting a node shows its selection state without opening a permanent panel.
Double-click, Enter on the selected node, or its configure action opens the node modal.
The modal identifies the node, source, and editing scope.
Sections cover General, Source, Capacity, Allocation, and Location only when applicable.
Site/AP forms show the full coordinate values and accept Google Maps paste.
Shared plan edits use the same authoritative records as the subscriber screens.

Inventory opens as a modal, not a permanent left column.
An Add to canvas action provides a keyboard alternative to drag placement.
Drag placement must close the modal and restore an interactive canvas before the placement gesture starts.
Do not attempt to drag through an inert modal backdrop.
Discovery adds no node until the operator selects a resource for placement.

Valid field changes use the automatic save/apply path while the modal stays open.
The modal footer shows save/apply status and a Close action, without a Save or Approve button.
Close and Escape dismiss the modal. They do not roll back edits that already applied.
Unsent invalid fields remain drafts for that node and show an inline error when reopened.
An Undo action submits a new revision instead of restoring files directly.

Every FAB uses a consistent SVG icon, a tooltip, and an accessible name.
Use targets of at least 44 by 44 CSS pixels, visible focus, and meaningful disabled explanations.
Native or equivalent accessible dialogs trap focus, support Escape, and return focus to the opening control.
Background controls remain inert while a modal is open.
Announce automatic save/apply results through a polite live region without repeated focus changes.
Keep paste enabled and do not impose a decimal step that truncates coordinates.

On narrow screens, keep the icon dock compact and use a near-full-screen modal with its own scroll area.
Keep the modal title and Close control available above the software keyboard.
The viewport must not scroll sideways to reach fields or action buttons.
Use the existing Light, Dark, and System theme preference and respect reduced motion.

The visual direction remains an industrial network operations console.
Reuse IBM Plex Sans, the existing theme tokens, compact typography, and semantic status colors.
Keep nodes and connectors as the primary content. Reserve cyan selection emphasis for the active graph relationship.
Use a quiet grid, clear ports, restrained surface elevation, and consistent icon strokes.
Avoid decorative dashboard cards and animated background effects around the canvas.

See the [interactive design reference](evidence/topology-canvas/mockup.html).
It uses synthetic data and documentation-only browser interactions, with no service or RouterOS access.
The production implementation remains Rust/Leptos/Wasm.

## Automatic save and apply

The normal workflow is `Edit -> automatic save -> automatic validation -> automatic apply`.
No Verify, Preview, Approve, Confirm, or Publish button is required to complete an ordinary edit.
Preview remains an optional inspection tool. Opening or closing it does not control apply.
Automatic runtime checks report results without an operator approval step.

A completed connector or hierarchy action submits an edit automatically.
A valid modal change submits after a short debounce or field completion.
Partial numeric input remains a local draft. Closing the modal flushes valid pending edits.
An incomplete edit retains its values and identifies the field that needs correction.
Canvas movement saves layout after the gesture. It does not rebuild the shaping files.
Undo and redo submit new revisions through the same automatic path.

The service persists each accepted policy revision and schedules reconciliation automatically.
It may coalesce revisions, but only the newest valid revision can become the desired active revision.
One service owns validation, compilation, and publication of the coordinated LibreQoS file pair.
The browser never writes either shaping file directly.
The service rejects stale edits and returns the current revision. It must not silently overwrite another operator's changes.

Show saved policy revision and active revision separately.
Status labels distinguish local draft, saved, pending apply, applied, and failed apply.
A failed apply retains the last-good shaping state and exposes the failure and retry status.
Automatic retry must not restore a superseded revision.
An invalid or incomplete hierarchy must not replace a valid active tree.
Node removal must resolve affected children and subscribers or return an actionable error.

Automatic apply preserves authentication, eligibility, ownership, and compiler checks.
An inactive or conflicting writer produces a visible setup or ownership error, without a per-edit approval workaround.
The implementation must address the current activation boundary before it claims complete automatic apply.
This design does not silently transfer ownership from another active writer.

## Graph and export rules

Reuse the current inventory catalog, stable IDs, placement resolver, compiler, and sole publisher.
Separate observed resources, saved operator intent, canvas layout, and effective export state.
Do not add independent graph engines or a second registry without a demonstrated requirement.

Multiple WANs can connect to one logical downstream group.
A WAN can also connect to several downstream groups with deeper Site and AP branches.
Each connector has a defined meaning, such as hierarchy or WAN eligibility.
A visual connector does not automatically become an exported parent edge.

The compiler resolves logical groups into placements with one parent per exported node.
Each subscriber has one effective placement and one circuit identity, even with several devices or addresses.
Names remain editable labels. Stable IDs preserve identity through renames and movement.
Visual groups do not rely on the retired LibreQoS v3 virtual-node behavior.

A shared physical bottleneck must retain its total capacity across all WAN placements.
Do not duplicate a 300 Mbps link into two independent 300 Mbps allowances.
If the target tree cannot enforce the requested constraints, reject that candidate with a specific explanation.
Never show a topology as applied when its shared limits cannot be represented.
Inspect repeated Site placements in the LibreQoS map to detect duplicate physical locations.

## Subscriber allocation

The allocation unit is a subscriber, not a group or connector count.
Supported product modes are equal subscriber count, capacity allocation, and quality-aware allocation.
Count and quality modes are required behavior, not claims about the current implementation.
Quality mode requires fresh measurements through each specific WAN and an explicit stale-data policy.
Use cooldown and a minimum improvement threshold to avoid repeated subscriber movement.

Eligibility constrains every placement, including pins.
An eligible WAN pin remains fixed during ordinary automatic allocation.
Pinned demand counts against the WAN capacity before the allocator places unpinned subscribers.
The allocator considers eligible WANs across child groups, while preserving each subscriber's source and access constraints.
A pin cannot authorize a path that the subscriber cannot use.

Actual MikroTik route changes remain a separate unresolved scope decision.
A LibreQoS parent change does not steer Internet traffic to another WAN.
The UI must distinguish a shaping assignment from an observed or enforced route.
Strict-pin failure behavior and quality probe policy require a separate decision before those modes can ship.

### Implementation state (2026-11-22)

- The operator sets a topology default mode (`allocation.default`) and, for each subscriber source, its own mode or the default (`group.allocation`, `null` for the default). Registry schema 7.
- Equal subscriber count: the default. New subscribers go to the node with the fewest subscribers. Pins stay fixed and count in their node.
- Capacity: by plan demand against the capacity above each node.
- Quality: as capacity, with each WAN capacity multiplied by its quality score. Stale quality data uses capacity.
- An upgrade from schema 6 or older keeps capacity mode for each source that the registry already has.
- A fixed capacity on one placement of a shared source is refused, because LibreQoS cannot hold one shared limit across parents.

### WAN monitor and failover (2026-11-22)

- The service probes each WAN node every 10 seconds through its router: it reads the interface state and pings two targets out of that interface. The `[wan_monitor]` config section holds the values.
- A disabled or not running interface is bad at once. A loss of 50% or more, or a median latency above 3 times the normal value, in a 60-second window, is bad.
- Subscribers move only after more than 2 minutes of bad state. They go to the other eligible WANs. A pin stays on its node.
- The saved node stays the home of a moved subscriber. When the WAN is stable for 10 minutes, the subscriber returns (automatic failback). With automatic failback off, the operator returns them in Topology settings.
- A probe error (no answer, or no permission) is no evidence. The WAN shows "No probe data" and nobody moves.
- When all eligible WANs are bad, nobody moves.
- With the routing list writer on, a failover also moves the client entries to the `via-` list of the other WAN. Without it, a failover changes the LibreQoS placement only.

### WAN routing lists (2026-11-22)

- The writer puts each client address in the `via-` list of the WAN above its current node, with the comment `lqosync: <client>`. It changes only such entries.
- Each publication writes the lists first. The shaping pair goes out only after the router confirms the lists.
- A write that removes too many clients holds for one cycle. A failed write keeps the previous pair.
- The one-time adoption (Topology settings, **Adopt current routing**) keeps the WAN that the old range lists give each client.
- Operator procedure: [WAN routing lists](../operations/WAN_ROUTING_LISTS.md).

### Automatic placement (2026-11-22)

Spec: [automatic placement design](../superpowers/specs/2026-11-22-topology-auto-placement-design.md).
The service places each subscriber on each cycle. A browser is not necessary.

| Rule | Condition | Placement |
|------|-----------|-----------|
| R1 | The tree has no nodes. | No parent (flat). |
| R2 | The source of the subscriber is placed. | The placed node. |
| R3 | The router has a router node. The source is not placed. | The child nodes of the router node, else the router node. |
| R4 | The router has no router node, but has its own WAN node. | The WAN nodes of the router. |
| R5 | The router has no router node and no own WAN node. | The root nodes. |
| R6 | R3, R4, or R5 gives no eligible node. | No parent (flat). |

- A subscriber never goes into the branch of a router node of a different router. The service refuses an edit that breaks this rule.
- A node that holds a subscriber source does not take automatic subscribers.
- A fallback policy has priority over R3–R5. The router rule also applies to it.
- The accounts of one subscriber use the intersection of their sets. An empty intersection is a `NoEligibleTarget` conflict.
- The service reads each enabled PPP and DHCP protocol. A protocol without a saved selection gets the selection of the policy proposal. The next publication saves it. Owner decision, 2026-11-22.
- Registry schema 6 adds `router_id` to a node. A rollback to a schema-5 binary needs the schema-5 backup.

Open owner questions:

- Stability: saved assignments stay, and new subscribers go to new nodes. This is a builder decision. Make sure that the owner accepts it before the release.
- More than one WAN: the own WAN of the router first, then capacity balance. This is a proposal. Make sure that the owner accepts it before the release.

### Proposal for owner decision: quality mode and strict pins

This proposal is not approved. Do not implement quality mode until the owner accepts or changes it.

- Measurement: one probe per WAN path, through that WAN only (for example a ping to a fixed target with a routing mark). Record loss and latency.
- Freshness: a measurement older than 3 probe intervals is stale. With any eligible WAN stale, quality mode uses capacity mode for that rebalance and shows "quality data stale".
- Movement: move a subscriber only when the score of the destination is better by at least 20%, and not more often than once per 30 minutes per subscriber.
- Strict pin, WAN unavailable: keep the pin and the shaping placement. Show the subscriber as "pinned to an unavailable WAN". Do not move it automatically.
- Routing: quality mode changes only the LibreQoS placement. MikroTik routing stays a separate decision.

## Site and AP coordinates

Each Site and AP supports an optional location in its configuration modal.
Geographic type (`site` or `ap`) remains separate from the current topology role.
The owner copies coordinates from Google Maps in `latitude, longitude` order.
Support a pasted pair and separately labeled fields, with spaces and negative values.
Do not impose a fixed decimal-place limit or silently swap coordinate order.

Preserve the complete validated decimal text in saved policy and the editor.
Use at least `f64` for calculations. Do not intentionally round numeric coordinate exports.
Latitude must be within -90 to 90. Longitude must be within -180 to 180.
Both coordinates must be present or both absent.
Explain the upstream map treatment of `(0,0)` and its Web Mercator latitude limit.

Export numeric `latitude` and `longitude` fields with the correct `type` in `network.json`.
Do not add coordinate columns to `ShapedDevices.csv`.
Explicit node coordinates take precedence over legacy router coordinates.
Preserve legacy fallback behavior until a defined migration changes it.
Canvas `x/y` positions remain separate. Moving a card never changes its geographic location.

The reviewed upstream model uses `f32`, so LibreQoS can reduce precision after import.
Do not promise exact preservation inside LibreQoS without a check against the installed release.
The upstream map can display an AP at its immediate parent Site when its own coordinates are absent.
Label inherited display locations and distinguish them from explicit coordinates.

## Required acceptance evidence

- Discovery supplies Inventory without creating or changing active topology.
- Completed valid edits reach the service and apply without preview or approval.
- Preview remains optional and cannot gate publication.
- Rapid edits, stale responses, concurrent edits, reconnects, and retries preserve revision order.
- Invalid edits and failed compilation retain the active last-good pair.
- Layout movement does not cause a shaping reload.
- Shared sources preserve one circuit per subscriber and enforce eligibility and pins.
- Shared bottleneck cases either enforce the total limit or fail with an explicit error.
- Long Google Maps decimals survive paste, save, reload, edit, and numeric JSON export.
- Site/AP types and coordinates reach runtime topology and the installed LibreQoS Site Map.
- Source outages retain saved intent and clearly identify stale observations.
- All Topology settings remain accessible through canvas FABs and modal editors.
- Keyboard access, focus return, Escape, full coordinate paste, and narrow modal layouts pass browser checks.

## References

- [Research and compatibility findings](../../brain/10-sessions/2026-11-21-infinite-canvas-research.md)
- [Current workflow and activation boundary](TOPOLOGY_WORKFLOW_REPAIR.md)
- [Original web design](TOPOLOGY_DESIGN.md)
- [LibreQoS v3 changes](https://libreqos.readthedocs.io/en/latest/docs/v3/whats-new.html)
- [LibreQoS coordinate parser](https://github.com/LibreQoE/LibreQoS/blob/develop/src/rust/lqos_config/src/network_json.rs)
- [LibreQoS Site Map](https://github.com/LibreQoE/LibreQoS/blob/develop/src/rust/lqosd/src/node_manager/js_build/src/site_map.js)
- [Vercel frontend-design skill](https://github.com/vercel-labs/open-agents/blob/main/.agents/skills/frontend-design/SKILL.md)
- [Vercel Web Interface Guidelines](https://github.com/vercel-labs/web-interface-guidelines/blob/main/command.md)
