# Live traffic and WAN capacity

Status: implemented and verified. Runtime installation is pending.

## Design

Keep the approved Dashboard layout, fonts, themes, and sample table.
Use the chart as the main visual change.
Use a vertical rainbow from violet at zero through blue, cyan, green, yellow, and orange to red.

Red means 100% of the configured capacity for that direction.
Download uses a solid line. Upload uses a dashed line.
Show both capacities and utilization as text. Color alone must not carry the meaning.

Sum the true base rates of WAN nodes in the validated topology catalog.
Keep download and upload sums separate. Do not add them together.
Do not sum subscriber plans, intermediate nodes, or shaping factors.
Configured capacity does not prove link availability, routing, or applied shaping.

Without current capacity evidence, use the existing rate scale and neutral fills.
Do not mark a local traffic peak as a capacity limit.

## Completed atomic implementation plan

1. Publish a small capacity summary through the existing service status cache.
   Check multiple WANs, reduced shaping factors, missing catalogs, and integer overflow.
2. Project only this summary into the browser contract.
   Reject invalid rates and old status evidence.

3. Sample native traffic once per second. Keep the existing counter source and reset rules.
   Keep 3601 points and the existing one-hour time limit. Bound the public worker cache at 2 MiB.
   Refresh the broker traffic cache once per second and other overview fields every five seconds.

4. Add an authenticated server-sent event stream with bounded connections.
   Recheck account and session state before each event. Stop on authorization or broker failure.

5. Consume the stream in Rust. Close it when the page is hidden or the Dashboard unmounts.
   Keep the existing overview poll for recovery and session checks.

6. Apply gradients in SVG user coordinates. Each red boundary must match its own capacity.
   Keep actual rates above capacity visible. Do not change source values.

7. Check stream revocation, stale evidence, chart geometry, themes, and narrow screens.
   Save browser evidence. Build, document, commit, and push the change.

## Source research

LibreQoS reference revision: `15be4ca69d07691813260ed620498fb1f6c9aae6`.
The deployed `/opt/libreqos` tree contains compiled browser assets only.
The upstream source confirms a one-second ticker and a timestamped ring buffer.
LibreQoS uses WebSockets. LQoSync uses server-sent events for this one-way monitor.

Both deliver source observations on a one-second cadence.
No LibreQoS application code is copied.

- [Ticker](https://github.com/LibreQoE/LibreQoS/blob/15be4ca69d07691813260ed620498fb1f6c9aae6/src/rust/lqosd/src/node_manager/ws/ticker.rs)
- [Throughput source and configured limits](https://github.com/LibreQoE/LibreQoS/blob/15be4ca69d07691813260ed620498fb1f6c9aae6/src/rust/lqosd/src/node_manager/ws/ticker/throughput.rs)
- [Live ring buffer](https://github.com/LibreQoE/LibreQoS/blob/15be4ca69d07691813260ed620498fb1f6c9aae6/src/rust/lqosd/src/node_manager/js_build/src/graphs/throughput_ring_graph.js)

Vercel frontend-design guides visual restraint and preservation of the approved design.
Google modern-web-guidance `visual-effects` requires text with gradient charts.
Its rule says to "always pair gradient charts with an accessible textual representation of the data".
SVG uses explicit color stops. CSS color interpolation syntax does not apply to SVG gradients.

See [implementation evidence](evidence/live-traffic/README.md) for the checks and browser pictures.
