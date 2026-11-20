# Dashboard implementation against the approved reference

Date: 2026-11-20 (Asia/Shanghai). Branch: `lqosync3`.

The [Dashboard picture](images/01-dashboard.png) defines the visual composition.
[SCREENS](SCREENS.md), [DESIGN](DESIGN.md), and the backend contracts define the behavior.
Written corrections take priority over generated sample values and labels.
Use Vercel frontend-design and Google modern-web-guidance for each frontend change.
Keep these references as the design acceptance criteria for later work.

## Atomic work


1. Add a safe IPoE count from bound DHCP observations. Test mixed sources, zero, absent evidence, and stale data.

2. Match the five summary cards, icons, section spacing, and shared header to the reference.

3. Match the traffic area, numeric scale, time labels, and available-window states.

4. Add the WAN distribution panel and the upstream health table with explicit unavailable data.

5. Keep diagnostics below the primary panels. Preserve sign-in, themes, sessions, and the TUI.

6. Build Rust/Wasm. Check the browser at desktop and phone widths. Save evidence, then commit and push.

## Composition

Keep the product mark and superscript v3 in the sidebar.
Use IBM Plex Sans, tabular values, the documented dark surfaces, and the existing light theme.
The main content has five summary cards.
The next row has a wide traffic panel and a WAN distribution panel.

The upstream health and allocation table spans the next row.
Service diagnostics follow below this table.
The shared header retains Light, Dark, and System plus account and sign-out controls.

## Backend boundaries

| Reference item | Current evidence or explicit gap |
|---|---|
| IPoE Subscribers | Count bound DHCP observations. Do not substitute total subscribers. |
| PPPoE online/offline | Use coherent collection source counts. |
| Routers | Use coherent router collection evidence. |
| LibreQoS | Show the actual publication phase. An import does not prove application. |
| Traffic | Use bounded aggregate worker samples. The source does not prove per-WAN routing. |
| 1h / 6h / 24h controls | Unavailable until history retention covers each window. Keep Recent available. |
| WAN allocation distribution | No safe published per-WAN traffic projection exists. Show unavailable, never invented shares. |
| Upstream capacity and health | No safe published WAN overview exists. Do not read private config or infer routes. |
| Navigation | Keep Dashboard active. Mark future destinations unavailable. Omit web account administration. |

The WAN adapter remains a backend gap under F06 and D03.
It needs stable WAN IDs, configured capacity, membership, observation times, and attributed byte counters.
It must distinguish shaping membership from packet routing and must not sum utilization percentages.
The current gateway continues to read the same four fixed public caches.

## Acceptance

Match the section order, density, card hierarchy, gradient series, and table structure of the reference.

Keep unknown values distinct from zero. Keep stale evidence visible.
All visible controls must work or state why they are unavailable.
Keep readable units and time labels on the chart. Keep table headers and keyboard scrolling on narrow screens.
The implementation must not create sample production data, start a collector, or change account policy.

## Current checkpoint

The composition and available cache connections are implemented.
See [verification and pictures](evidence/dashboard-alignment/README.md).
The WAN adapter and wider history remain explicit backend gaps.
