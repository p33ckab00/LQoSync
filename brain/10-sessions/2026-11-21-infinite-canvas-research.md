---
type: session
created: 2026-11-21
tags: [topology, research, canvas]
status: direction-finalized-implementation-pending
related: ["[[2026-11-21-frontend-owned-topology-requirement]]", "[[ADR-0005-topology-follows-v3]]"]
---

# Infinite canvas research

The owner requested research and discussion about an infinite canvas for Topology Manager.
The proposed canvas has a router inventory, selected discovery, connectors, and modal forms.
A router can supply WAN interfaces and subscriber sources.
The owner wants shared groups beneath several WANs, automatic subscriber allocation, and WAN pins.
Allocation modes can use subscriber count, capacity, or Internet quality.

## Current evidence

The workflow guide describes empty initialization, explicit source attachment, and shared sources with separate parent instances.
Each exported node has one parent. Each subscriber has one effective placement.

The current rebalance implementation uses demand relative to capacity and excludes pinned subscribers from movement.
These facts do not prove automatic quality allocation or router route control.
The branch was `lqosync3` in `/home/pi/LQoSync3.0`.
Prior edits to Routers and shared notes remained intact.
The project graph supplied source leads. The current source and workflow guide confirmed the relevant facts.

## Research findings

The official example has Site, PoP, and AP branches with several depths.
LibreQoS v3 still reads `network.json` and `ShapedDevices.csv`.
LibreQoS v3 treats exported virtual nodes as real sites.
A visual group thus needs an explicit export rule.

RouterOS exposes PPP profiles, account records, active sessions, DHCP servers, and leases.
PPP profile selection and PPPoE server selection are different filters.
RADIUS and uncertain session-to-server relations need explicit evidence.
RouterOS policy routing controls actual WAN selection. A LibreQoS parent change alone does not change the router route.

## Open questions

Does allocation include actual MikroTik route changes, or only LibreQoS placement?
What does a WAN pin do when that WAN fails?
Which groups represent real shared capacity limits?
How will probes measure quality through each specific WAN?

The later owner instruction finalized the canvas direction and requires automatic apply without a preview or approval gate.
The final product direction is in `docs/frontend/TOPOLOGY_CANVAS_DESIGN.md`.
Actual route control remains an open scope question. No application code, router, service, or live export changed.

## Shared reference and node coordinates

The owner supplied a [shared concept](https://chatgpt.com/s/t_6b01ba05db6c81919a87a46cbf8f03f9).
The page text extractor showed the login shell. The public page data contained the readable concept.
The review used that concept as a proposal, then checked the current source.

The inventory, editable intent, and compiled tree separation fits the current system.
Reuse the current catalog, resolver, validation logic, and sole publisher.
The final direction replaces operator confirmation with automatic save and apply.
Do not require three new graph engines or a second file writer.

The reference uses v2 virtual-node behavior. LibreQoS v3 does not preserve that behavior.
The reference puts pins before eligibility. The current resolver rejects a pin outside eligible targets.
Capacity allocation exists. Count and quality modes still need their own acceptance evidence.
Duplicated placements must not multiply a real shared bottleneck.
Repeated site placements also need map verification to detect duplicate physical locations.

The owner now requires an optional coordinate pair for each Site and AP.
The current exporter copies router coordinates to source-derived nodes.
Manual intermediate nodes receive no coordinates from that path.
The current exporter also writes `type: site` for every topology node.
Per-node coordinates and an explicit Site/AP export type thus require model and export changes.
Keep the current node role separate from the geographic type.

The upstream parser accepts numeric `latitude` and `longitude` metadata beside `type`.
Latitude ranges from -90 to 90. Longitude ranges from -180 to 180.
The upstream map excludes `(0,0)` and latitudes outside its Web Mercator range.
An AP without valid coordinates can inherit display coordinates from its immediate parent Site.
That display fallback does not prove location inheritance in the exported file.

Proposed node forms need General, Capacity, Source, and Location sections where applicable.
Save canvas positions separately from geographic coordinates.
An explicit node location must take precedence over the legacy router location.

Preserve the legacy fallback for current configurations until the owner chooses migration behavior.
Write coordinates to `network.json`, without new coordinate columns in `ShapedDevices.csv`.
Verify metadata through the compiler, runtime topology, and Site Map on the installed LibreQoS release.
The upstream source review does not establish deployed runtime acceptance.

## Google Maps coordinate precision requirement

The owner requires support for long decimal coordinates copied from Google Maps.
Accept a pasted pair in `latitude, longitude` order, then populate both labeled fields for automatic save.
Do not infer longitude-first order from the phrase "long, lat".
Accept negative values and spaces around the comma.
Do not limit input to a fixed number of decimal places.
Preserve the original validated decimal text in saved policy so edits do not lose digits.

Use at least `f64` for calculations. Binary floating point does not preserve every arbitrary decimal exactly.
Export numeric coordinate values without an intentional decimal-place reduction.
The export path must preserve the saved decimal value where the target format permits it.
Distinguish any shortened display text from the full value in the editor.

The current router configuration uses `Option<f64>` for coordinates.
The reviewed upstream LibreQoS node model uses `Option<f32>` for coordinates.
LibreQoS can thus round coordinates after import even when LQoSync preserves the input.
Do not promise exact decimal retention inside LibreQoS without an installed-version check.
Acceptance checks must cover paste, save, reload, edit, and numeric JSON export with long decimal values.

## Canvas settings refinement

The owner requires all Topology settings inside the canvas as icon FABs that open modals.
The design now defines the dock, modal scope, automatic field changes, focus rules, and narrow layouts.
The screen ownership table now points Topology/WAN settings to canvas modals.
Vercel frontend-design and Web Interface Guidelines informed the refinement.
The documentation-only mock is in `docs/frontend/evidence/topology-canvas/mockup.html`.
Its browser check passed for modal access, focus return, long decimals, themes, and narrow layouts without network requests.
Production UI and backend integration remain pending. No live state changed and no commit occurred.

## Reference links

- [Official topology example](https://github.com/LibreQoE/LibreQoS/blob/develop/src/network.example.json)
- [LibreQoS v3 changes](https://libreqos.readthedocs.io/en/latest/docs/v3/whats-new.html)
- [RouterOS PPP AAA](https://help.mikrotik.com/docs/spaces/ROS/pages/132350049/PPP%2BAAA)
- [RouterOS DHCP](https://help.mikrotik.com/docs/spaces/ROS/pages/24805500/DHCP)
- [RouterOS policy routing](https://help.mikrotik.com/docs/spaces/ROS/pages/59965508/Policy%2BRouting)
- [LibreQoS coordinate parser](https://github.com/LibreQoE/LibreQoS/blob/develop/src/rust/lqos_config/src/network_json.rs)
- [LibreQoS Site Map](https://github.com/LibreQoE/LibreQoS/blob/develop/src/rust/lqosd/src/node_manager/js_build/src/site_map.js)
