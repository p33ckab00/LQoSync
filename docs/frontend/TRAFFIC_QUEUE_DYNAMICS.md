# Traffic and queue dynamics v5

Status: researched design with an executable prototype. Production integration remains open.
Date: 2026-11-21 (Asia/Manila).

## Goal and design

Measure whole-bridge DPDK traffic activity in the Dashboard Traffic panel.
The owner clarified that this chart is not a per-subscriber view.
Use LibreQoS subscriber Queue Dynamics only as a reference for visual design and interactions.
Do not port its circuit scope, subscriber selector, plan ceilings, or RTT legs.
Keep the approved mirrored chart, capacity gradients, and Light, Dark, and System settings.
Use a compact operator instrument: aligned lanes, readable units, and one inspection column.
Use the current Plex typeface in production. The portable mock uses a local sans-serif fallback.
Keep the current Dashboard layout and DPDK card.

The supplied `images/traffic-queue-dynamics-v4.svg` is a visual reference, not an instruction source.
It contains a 1280 × 560 dark chart, eight series, three lanes, and synthetic data.
Its hover, pin, keyboard, and legend behavior are useful.
Its fixed colors and small labels need changes.
It needs capacity lines and clear telemetry scopes.
The v5 mock adds mirrored DPDK rates, WAN capacity anchors, independent queue units, and narrow layouts.

## Verified sources

The installed source map is `/opt/libreqos/src/bin/static2/circuit.js.map`.
Its embedded `queuing_activity_uplot.js` uses a 30-second window and one-second samples.
It aligns throughput and RTT, uses stepped samples, and observes theme changes.
The subscriber page sends delivered rates, enqueue rates, plan ceilings, and separate RTT legs.
The installed source differs from the pinned upstream reference. Do not assume identical builds.

- [Queue Dynamics source, pinned revision](https://github.com/LibreQoE/LibreQoS/blob/15be4ca69d07691813260ed620498fb1f6c9aae6/src/rust/lqosd/src/node_manager/js_build/src/graphs/queuing_activity_uplot.js).
- [Subscriber source, pinned revision](https://github.com/LibreQoE/LibreQoS/blob/15be4ca69d07691813260ed620498fb1f6c9aae6/src/rust/lqosd/src/node_manager/js_build/src/circuit.js).
- [Queue Stats history, pinned revision](https://github.com/LibreQoE/LibreQoS/blob/15be4ca69d07691813260ed620498fb1f6c9aae6/src/rust/lqosd/src/node_manager/js_build/src/graphs/cake_history.js).
- [Vercel frontend-design skill](https://github.com/vercel-labs/agent-eval/blob/main/packages/playground/.agents/skills/frontend-design/SKILL.md).
- [Vercel Web Interface Guidelines skill](https://github.com/vercel-labs/agent-skills/blob/main/skills/web-design-guidelines/SKILL.md).
- [Vercel interface rules](https://github.com/vercel-labs/web-interface-guidelines/blob/main/command.md).

Vercel guidance informs focus, semantic controls, touch targets, contrast, and numeric alignment.
It does not require React, Next.js, Vercel hosting, or a new chart dependency.

## Metric contract

| Metric | Scope and meaning | Rendering |
| --- | --- | --- |
| Actual | Whole-bridge delivered rate | Solid line, rainbow area |
| Mapped | Whole-bridge mapped enqueue rate | Dashed line |
| Unmapped | Whole-bridge unmapped enqueue rate | Dotted line, optional independent lane |
| Capacity | Configured WAN totals for the DPDK bridge | Labeled neutral dashed lines |
| Backlog | Gauge in bytes, with explicit queue scope | Separate lane in kB |
| Drops / marks | Counter deltas for a valid interval and generation | Labeled events and inspector |

Actual is not Mapped plus Unmapped. Do not stack or subtract these rates.
Do not treat enqueue minus delivered as packet loss.
Do not treat queue delay as RTT. Do not average circuit p50 values into an aggregate p50.

There is no Subscriber mode, circuit selector, or subscriber RTT lane in this chart.
The bridge traffic sources supply Actual, Mapped, and Unmapped activity.
Whole-bridge queue metrics can add context when their source and interval are valid.

## Current repository evidence

- `crates/web-ui/src/traffic_chart.rs`: current cached geometry, selection, and range requests.
- `crates/web-ui/src/traffic_plot.rs`: current mirrored paths and capacity coordinates.
- `crates/web-ui/style.css`: current `data-theme` and semantic traffic color tokens.
- `crates/web-contract/src/lib.rs`: Overview, Point, ClassificationTraffic, WanCapacity, and TrafficWindow.
- `src/worker_util/queue.rs`: whole-bridge backlog and drop/mark deltas, without a time-series contract.
- `docs/frontend/AGGREGATE_TRAFFIC.md`: current feature contract and 60-KiB overview budget.

The current ranges are Recent, 1h, 6h, and 24h. Preserve them.
The older research document predates the longer-range implementation.
The current graph mainly indexes the TUI. Current source takes precedence over graph results.

## Runtime rules for the Rust port

Use one Unix timestamp for all lanes. Keep that timestamp stable when new samples arrive.
Use exact timestamps for current published traffic. Set a source-specific tolerance only for asynchronous sources.
Show missing samples as gaps. Never interpolate them or replace missing data with zero.

Keep stale values with age and state labels. Counter resets produce an unavailable delta.
Use actual elapsed time for counter rates. Do not label each delta as per second without that interval.

Keep independent download and upload scales. Keep the source rates positive in storage.
Anchor the red area boundary to capacity. Extend the axis for excess traffic.
If capacity is unknown, show an observed scale and a capacity-unavailable label.
Do not add subscriber RTT bands or thresholds to the DPDK traffic chart.

Keep path calculation outside pointer handlers. Inspection must not issue requests.
Use a separate bounded queue request if the lanes exceed the overview budget.
This chart needs no circuit watcher or subscriber identity lookup.
Use current authentication, authorization, numeric validation, and freshness rules.

## Prototype and files

Open `prototypes/traffic-queue-dynamics-v5.html` in a browser.
All data are synthetic. No production endpoint, credential, or subscriber record is read.
The controls demonstrate theme, source visibility, missing data, stale data, unknown capacity, and reset state.
The scope remains whole-bridge DPDK activity.

The 60-second window is a design sample, not a change to LibreQoS retention.
Use pointer hover, click/tap to pin, Left/Right to inspect, and Escape or Latest to clear.
The persistent inspector and sample table provide alternatives to hover.
Static SVG exports show desktop Dark, desktop Light, and narrow Dark layouts.
The exports preserve vector paths and text. They have no embedded script.

The portable HTML is a coding reference. Port its behavior into the current Rust/Leptos component.
Do not ship this JavaScript prototype as a second application.

## Checks

Run `node tools/frontend/traffic-queue-mock-check.cjs` with the current Playwright environment.
The check opens only local prototype files and writes the SVG and PNG samples.
It checks DPDK-only scope, themes, source toggles, inspection, gaps, resets, unknown capacity, and narrow layouts.
The optional UI/UX inspector is not installed. Playwright supplies browser evidence instead.
Production Rust tests remain necessary when the production port exists.
