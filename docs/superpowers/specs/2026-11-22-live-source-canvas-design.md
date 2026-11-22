---
type: spec
created: 2026-11-22
status: approved-pending
related: ["[[topology-manager-status]]", "2026-11-20-topology-activation-design", "2026-11-22-topology-auto-placement-design"]
---

# Live source canvas design

## Intent

The Topology canvas shows what LQoSync reads from the MikroTik routers now, as the legacy `root_promotion` and `grouped_router_root` modes do. The operator does not build the subscriber groups by hand. The operator connects WANs as parents, adds router cards when necessary, and changes the settings of each card in a modal.

Why (owner, 2026-11-22): on the production shaper, the policy proposal made groups for PPP profiles without users. The operator could not remove them, and the canvas did not agree with what LQoSync shapes. The owner wants a canvas that follows the routers, in the style of the `traffic-intelligence` canvas.

This design replaces these parts of earlier designs:

- `docs/frontend/TOPOLOGY_CANVAS_DESIGN.md` (2026-11-21): "A fresh topology starts empty. Discovery adds resources to Inventory only." Subscriber groups and WANs now load on the canvas automatically.
- Commit `3f6491d` (a group for each PPP profile that a PPP secret uses). The live rule below uses active sessions. The builder adapts or reverts that commit.
- `docs/superpowers/specs/2026-11-20-topology-activation-design.md` gate 3 (exact parity). Reviewed changes now pass (section 6).

## Owner decisions (2026-11-22)

1. The canvas loads these cards automatically from the routers:
   - PPPoE: one group card for each PPP profile that has at least one active PPP session.
   - DHCP / IPoE: one group card for each DHCP server that has at least one lease that LQoSync shapes.
   - Hotspot: one group card for the Hotspot of a router that has at least one active session. Builder call: this is the legacy grouping (`GroupKey::Hotspot`, one group for each router).
   - WAN: one WAN card for each discovered WAN interface.
2. A card disappears when its source has no members. A source that is not on the canvas is not read for shaping.
3. A router card appears automatically for each router that has active subscriber sources. It is the parent of all group cards of that router on the canvas. Each router card has the setting "Exclude from network tree" (changed 2026-11-22, replaces "the operator adds it"):
   - Excluded: the canvas shows the card, but `network.json` has no router node. Its group nodes go to the parent of the router card (a WAN), else to no parent. This is the legacy `root_promotion` form.
   - Included: `network.json` has a router node with its group nodes below it. This is the legacy `grouped_router_root` form.
4. A connector makes a WAN the parent of a router card or of a group card. With two or more WAN parents, the subscribers are divided by the allocation mode: Count (default), Capacity, or Quality (earlier owner decision W4).
5. Each card has its own settings in a modal: concise and compact. The approved mockup (brainstorm screen `canvas-cards.html`):
   - Group card: capacity (from plans, or fixed), download and upload factor, minimum, the division between WANs, and a Subscribers tab.
   - WAN card: capacity, address list (for example `via-WAN1`), auto failback. Health is read-only.
   - Router card: "Exclude from network tree", location, and a link to the Routers screen.
6. The operator can connect WANs and change settings before activation. The activation does not wait for a canvas without changes.
7. Port the infinite canvas, the cards, and the connectors of `traffic-intelligence` (`https://github.com/p33ckab00/traffic-intelligence`, the owner's repository).
8. Approach A (below).
9. WAN cards: only an interface with a default route (`0.0.0.0/0`) on a core or edge router appears automatically, for example `pppoe-out1` and `pppoe-out2` of FSE-CORE. The other discovered interfaces are in an "Add WAN" list for manual addition.
10. Exclude: in the modal of a group card, the operator can select "Do not read". The card disappears, and LQoSync does not shape its subscribers. An "Excluded" list shows these groups so that the operator can put them back. Manual Site and AP nodes stay.
11. Routing mismatch: the connector is allowed. The card and the review show a warning: "MikroTik routing does not follow yet (writer off)". The rate of each subscriber stays correct.

## Goals

- The canvas agrees with the routers within one cycle plus the grace period.
- No subscriber loses its Circuit ID or Device ID because a card disappears and comes back.
- The operator connects a WAN with one drag, and changes a card in one modal.
- The activation can include the operator's canvas changes, after a review.

## Non-goals

- No new UI framework. The web UI stays Leptos 0.8 CSR.
- No router writes. The routing list writer stays off (`[wan_writer] enabled = false`) until the owner turns it on.
- No historical view, time rail, or 3D view from `traffic-intelligence`.
- No change of the router connection workflow on the Routers screen.

## Anti-goals

- Cards and shaping that switch on and off when a router reboots or a lease expires for a short time.
- A failed or partial router read that removes cards or subscribers.
- A changed Circuit ID or node ID for a subscriber or a group that comes back.
- Automatic placement that makes cards cover one another.
- Two writers of the LibreQoS pair.

## Constraints

- Rust 2024, Rust 1.89 compatible. No new dependency without a reason.
- Leptos 0.8.23 CSR web UI, Wasm through Trunk. Browser checks with `tools/frontend/check-all.sh`.
- Registry schema 8 today. A schema change needs a migration arm and a rollback note.
- The legacy `[network] mode` stays the start point of the first enrollment (activation design, decision 2).
- Persisted text in ASD-STE100.
- The production FSE-Distro router reboots after power loss. Leases disappear and come back with a delay.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. Develop the current canvas and backend (chosen) | Groups follow the current router read, with a grace period and kept settings. Router card rule. Gate change. Port the `traffic-intelligence` canvas engine into the current Leptos canvas, and use the current forms for the modals | Smallest risk. Keeps the tested canvas work (T0–T6). The engine must fit the current canvas code |
| B. New canvas from a full port | Replace the current canvas with a new component. Port the engine and the UI from Dioxus to Leptos | Closer to `traffic-intelligence`. Discards tested work. Larger and slower. The same backend work |
| C. Keep Inventory and drag, add an "Auto-load" button | Fill the canvas once from the current read | Smallest change. Cards do not appear and disappear by themselves, so it does not meet decision 2 |

The owner chose A on 2026-11-22.

## Design

### 1. Live sources (backend)

Each cycle reads the routers as today. A source is active when the last successful read of its router has at least one member that LQoSync shapes:

- PPP profile: an active PPP session of a secret with that profile.
- DHCP server: a lease that LQoSync shapes (with `require_bound`, a bound lease).
- Hotspot: an active Hotspot session of that source.

Rules:

- The service creates, hides, and shows these group nodes automatically, without an operator review. This is owner decisions 1 and 2. It replaces the auto-placement rule "no hidden group nodes that the operator did not create". An operator change (WAN connector, settings, router card) still goes through the review flow.
- After activation, a new or returning group changes `network.json`. With the bulk threshold of section 6, this is an incremental LibreQoS change.
- A new active source gets a group node automatically. Its node ID is stable: the same source always gets the same ID. For sources that exist in the legacy tree, use the legacy name and the `lq3-` node ID that the enrollment aliases give today.
- A source becomes inactive only after it has had no members for the grace period. Builder call: use `runtime.subscriber_grace` (600 s on production). Its card then disappears from the canvas, and its node is not exported.
- An inactive node keeps its settings, parent, WAN connections, and layout position. When the source is active again, the same node with the same ID and settings comes back.
- A failed, partial, or stale router read changes no source state. Only a successful read can make a source inactive.
- A source of a router that is not enabled, or a protocol that is not enabled in the configuration, is not read.
- An excluded source (decision 10) is not read and not shaped. Its node and settings stay, and the "Excluded" list shows it. The exclusion is an operator change through the review flow.

This replaces the "group for each PPP profile" rule of the policy proposal for live operation. The first enrollment still has to reproduce the legacy tree. The legacy tree has a node only for an active profile or a server with shaped leases, so the live rule and the legacy tree agree.

### 2. WAN cards

An interface with a default route (`0.0.0.0/0`) on a router with the WAN role appears as a WAN card automatically (decision 9). Builder call: the service runs this discovery in the cycle and keeps the result, so that the WAN cards stay after a restart. The other interfaces are in the "Add WAN" list. The current `wan.create` fields apply (capacity, factor, address list). Builder call: a WAN card without a saved capacity shows "not configured". The operator saves the capacity in the modal. A WAN without a capacity is not a parent in the export.

### 3. Router cards

- Builder call: the default of "Exclude from network tree" follows `[network] mode`. With `root_promotion` (production), each router card starts excluded, so the first tree stays equal to the legacy tree and parity passes.
- Builder call: a WAN connector to an excluded router card passes through. In the export, the WAN is the parent of the group nodes of that router.
- Builder call: a router with only WAN interfaces (for example FSE-CORE) gets no router card. Its name shows on its WAN cards.
- A change of the setting is an operator change through the review flow.

The approved mockup shows two WANs as parents of one router card. Thus a router card can have two or more WAN parents, and its subscribers are divided by the allocation mode (Count default). Builder call: a group card below a router card takes its WAN parents from the router card. A WAN connector straight to such a group card is refused, because it would break the router branch rule.

- An included router card is a router node. The existing placement rule R3 (the router node rule) applies.
- The router node keeps the current rule: a subscriber never goes into the branch of another router. This rule also applies to an excluded router card on the canvas.
- A router card disappears with the grace rule of section 1, when its router has no active sources.

### 4. Connectors

- Drag from a WAN card to a router card or a group card to make the WAN its parent. Drag a connector away, or use the modal, to remove it.
- A card with two or more WAN parents uses the allocation mode of its group (Count default, Capacity, Quality). The existing instance and allocation code applies ("Add parent").
- Connectors show their meaning with text and line style: configured parent (solid), automatic parent of a router card (solid, label "auto"), observed only (dashed). Selecting a connector shows why it exists.

### 5. Canvas engine (port from `traffic-intelligence`)

Port the framework-independent parts of `src/ui/canvas.rs` of `traffic-intelligence` into the web UI crate. Adapt them from fixed slots to a variable number of cards:

- Automatic placement that never makes cards overlap (16 px gap). Manual positions stay as overrides. Later automatic placement avoids new collisions with them. "Reset layout" removes the overrides.
- Builder call: a layered layout (WAN cards on the top row, router cards in the middle, group cards below their parent).
- Connector geometry: a straight vertical path when two cards share a column, otherwise a side-to-side curve with the label at the middle.
- Fit, zoom limits, pan with pointer capture, and a dedicated drag handle on each card so that a card drag does not pan the canvas.
- A list view with the same cards, settings, and connectors, for keyboard use and narrow screens.
- Visual system: compact NOC style, light and dark themes from the existing theme tokens, small radii, monospace for IDs and numbers.

Keep the existing saved layout file (`topology-manager/canvas-layout.json`) for positions.

### 6. Activation and live test with reviewed changes

- `topology live-test --accept-changes` and `topology activate --report <file> --accept-changes`: the parity gate refuses `MissingRow`, `DuplicateRow`, `UnclassifiedRow`, and `ProtectedRow` as today. `AddedRow`, `ChangedRow`, and `ChangedNetwork` pass, and the command lists them.
- Without `--accept-changes`, the gate stays exact.
- The live test report records the accepted changes and the registry revision. The activation requires a report of the same revision.
- Builder call: the active and inactive state of sources is runtime state, not a registry revision. Thus a lease change does not make a live test report stale. The report also records a fingerprint of the active sources. The activation prepares a fresh pair and lists the sources that changed since the report.
- Operational prerequisite (runbook): set the LibreQoS bulk generation threshold to 200 before an activation with many changes. Otherwise the dataplane holds the change until the maintenance window (see `docs/operations/LIBREQOS_V3_DATAPLANE_SETTINGS.md`).

### 7. Card modals

Use the existing forms (`crates/web-ui/src/topology_forms.rs`) and the review flow. Fields as in decision 5. A field that the service cannot change yet stays read-only with a short reason.

## Known limits and risks

- Routing: a group that the operator connects to WAN2 is shaped under WAN2. MikroTik routes it by the old `WAN1-CUSTOMERS` and `WAN2-CUSTOMERS` ranges until the routing list writer is on. The rate of each subscriber stays correct. The WAN ceiling counts its traffic under WAN2. The owner accepts this with a warning (decision 11).
- A large first activation is a full-world change in LibreQoS (see section 6).
- Hotspot: production has Hotspot off, and the first policy proposal refuses Hotspot sources today. The Hotspot rule needs fixtures, not production evidence.

## Left to the builder

- The data structure for active and inactive sources and nodes, and the schema change.
- The exact layered layout, card sizes, and connector routing around cards.
- The split into tasks and the order (backend first is recommended: the canvas needs the live source view).
- Whether `3f6491d` is reverted or kept for the first proposal.

## Verification

- Unit tests:
  - A source becomes active on its first member, and inactive only after the grace.
  - A failed read changes nothing.
  - A returning source keeps its node ID and settings.
  - Excluding and including a router card changes only the export: the group nodes move between the router node and its parent.
  - The gate with `--accept-changes` passes reviewed changes and refuses `MissingRow`.
- Native tests of the ported engine: no overlap for 1 to 200 cards, manual overrides kept, reset, fit.
- Browser checks (`tools/frontend/check-all.sh`): drag a WAN connector, open each modal, list view, keyboard access, light and dark themes, narrow screen.
- Production: the canvas shows `Tier-15M-WAN1`, `DHCP-LAN`, `DHCP-Wifi5Soft`, `DHCP-LibreQoEMgt`, and the FSE-CORE WANs, and no group for a profile without active sessions.
