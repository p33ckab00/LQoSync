# Mirrored traffic and queue research

Status: research reference. Research date: 2026-11-20 (Asia/Shanghai).
The owner later approved the traffic consolidation. See [the implementation](AGGREGATE_TRAFFIC.md).
The subscriber queue views and remaining layout changes are still proposals.
This document does not change the application or approve a new screen layout.

## Goal

Show Actual, Mapped, and Unmapped traffic in the current aggregate traffic panel.
Use a mirrored chart with a capacity gradient and a compact DPDK summary.
Use the LibreQoS subscriber views as interaction references.
Keep the implementation in Rust and preserve the current TUI.

## Evidence

The research used the installed LibreQoS package and its embedded source maps.
The installed package version is `3.0.7.202611192341`.
The source map is `/opt/libreqos/src/bin/static2/circuit.js.map`.
Its SHA-256 is `6f9f5c21b9d9cf0c2cfb0db017510fe54c4a9b45381abc7f7e00541e999049d8`.
Temporary source extracts remain outside the repository.

The research also checked upstream revision `15be4ca69d07691813260ed620498fb1f6c9aae6`.
This revision is a separate reference, not a verified package build identity.

| Reference | Finding |
| --- | --- |
| [Queue Dynamics source](https://github.com/LibreQoE/LibreQoS/blob/15be4ca69d07691813260ed620498fb1f6c9aae6/src/rust/lqosd/src/node_manager/js_build/src/graphs/queuing_activity_uplot.js) | Two aligned plots, a 30-second window, shared cursor, local tooltips, and stepped samples |
| [Subscriber page source](https://github.com/LibreQoE/LibreQoS/blob/15be4ca69d07691813260ed620498fb1f6c9aae6/src/rust/lqosd/src/node_manager/js_build/src/circuit.js) | Compact figures, ancestor queues, linked Queue Stats charts, and a circuit-specific watcher |
| [CAKE history source](https://github.com/LibreQoE/LibreQoS/blob/15be4ca69d07691813260ed620498fb1f6c9aae6/src/rust/lqosd/src/node_manager/js_build/src/graphs/cake_history.js) | Raw one-second samples over 180 seconds, mirrored directions, and paired legends |
| [TUI chart contract](../design/tui/MIRRORED_TRAFFIC.md) | Three sources, independent direction scales, source focus, gaps, and explicit rate meanings |
| [Existing DPDK design](DPDK_DASHBOARD.md) | Current web data sources, freshness limits, response bounds, and incomplete features |

Vercel `frontend-design` guided the visual hierarchy and reuse of the approved Dashboard.
Google `modern-web-guidance` supplied guidance for hover, focus, touch, and tooltips.
The Google package search used skill version `2026_11_10-47b4c80e`.
The retrieved topic was `interest-triggered-tooltips`.
The research did not install a chart library or an application dependency.

## What LibreQoS presents

### Queue Dynamics

Queue Dynamics aligns throughput and RTT plots on one time axis.
Enqueued traffic uses dashed lines. Delivered traffic uses filled series.
Configured ceilings have separate lines.
A shared cursor selects the same time in both plots.
Compact figures show throughput, enqueue rate, utilization, and RTT legs.
Utilization uses delivered traffic divided by the configured ceiling.


The source uses `uPlot.sync` for its cursor connection.
It computes scales when a sample arrives, not for each pointer movement.
The tooltip reads the nearest cached sample and displays its relative age.
The access and Internet RTT legs have separate meanings.
Do not substitute queue delay for either RTT leg.

### Queue Tree

Queue Tree shows the selected circuit's ancestor path to the root.
It adds live throughput, retransmit, and latency data for upstream nodes.
It is not a general diagram of all configured nodes.
A valid port needs applied node identity and live node telemetry.
A configuration tree alone cannot supply this view.

### Queue Stats

Queue Stats shows six linked panels: backlog, delay, queue length, traffic, marks, and drops.
The view keeps raw one-second samples for three minutes.
Hover over one panel to inspect the same timestamp in all six panels.
The source uses an ECharts group and `echarts.connect` for this connection.
The watcher starts for a selected circuit and stops when the view closes.

Not every backend supplies per-tin drops and marks.
The source explicitly uses direction totals when the bridge protocol lacks per-tin values.
A port must expose source capabilities and preserve these limits.

## Proposed Dashboard layout

Keep the approved navigation, theme controls, summary cards, and continuous diagnostic banner.
Replace the current aggregate plot with one comparison plot.
The current aggregate series already represents Actual traffic from the worker cache.
Do not count this series twice.

```text
Continuous service ticker
Existing Dashboard summary cards

+------------------------------------------+-----------------------+
| Traffic: Actual / Mapped / Unmapped       | DPDK health           |
| Current rates, legend, range, Live        | Health and source age |
|                                          | Worker mean / peak    |
| DOWNLOAD: above zero                     | Pool / ring pressure  |
| ----- shared time axis / zero -----      | Queue / drops / marks |
| UPLOAD: below zero                       | Ports / NIC errors    |
|                                          | Diagnostics details   |
| Shared cursor and selected-time figures  |                       |
+------------------------------------------+-----------------------+
```

On a narrow screen, place the DPDK summary below the plot.
Show current values before optional detail rows.
Keep all available diagnostics in an expandable detail section.
Keep per-subscriber queue exploration on its own feature screen.

### Series and gradient

| Series | Meaning | Proposed appearance |
| --- | --- | --- |
| Actual | Delivered traffic for the whole bridge | Main line with a rainbow area fill |
| Mapped | Enqueued mapped traffic | Amber dashed line |
| Unmapped | Enqueued unmapped traffic | Violet dotted line |

Actual does not equal Mapped plus Unmapped.
Do not stack or sum these series.
Keep upload rates positive in storage. Negate only the plot coordinate.
Use separate download and upload scales, with visible units and labels.

A legend toggle controls both directions of one source.
A source focus mode makes small Unmapped values easier to inspect.

Anchor the red gradient to the configured combined WAN capacity for each direction.
Zero remains at the center. Red appears at each outer capacity boundary.
Use a capacity line and extend the axis when traffic exceeds capacity.
Do not clip these samples or redefine red as the observed peak.

If capacity is unknown, show an observed scale and an explicit capacity-unavailable label.
Use line styles and labels as well as color.
Check contrast in Light, Dark, and System themes.

### Compact DPDK figures

| Visible figure | Detail available on demand |
| --- | --- |
| Bridge health and source age | Raw verdict and independent cache ages |
| Worker mean and peak busy | Worker count, worker bars, and bounded worker details |
| Pool free and ring use | Reported values, units, and pressure evidence |
| Queue backlog | Queue counters and source scope |
| Queue drops and AQM marks | Delta interval, reset state, and raw counters |
| Active ports and NIC errors | Per-port state, counters, and xstats |

Compute peak worker busy from typed source values when those values are available.
Do not derive chart values from formatted `Reading` strings.
Calculate counter deltas only across valid samples from the same source generation.
Show NIC and worker intervals separately. Their cadences differ.
Preserve unknown, zero, stale, reset, and unavailable as distinct states.

## Time and inspection behavior

Use one selected timestamp for all linked plots, not a rolling array index.
Show a crosshair and an inspector with Actual, Mapped, and Unmapped download/upload rates.
Include the timestamp, units, capacity, source age, and data quality.
Find the nearest sample within a defined source cadence tolerance.
Do not interpolate across a gap or reuse an old sample without a stale label.

Support pointer hover, touch selection, and keyboard movement.
Use Left/Right to select samples and Escape to clear a pinned selection.
Keep essential figures visible outside the tooltip.
Keep tooltip text readable and do not move keyboard focus on hover.
Provide an accessible table or inspector alternative.

Add explicit 5-minute, 15-minute, and 1-hour ranges only when the API supplies those windows.
Keep a selected historical range stable while new samples arrive.
Use `Live` to return to the latest window.
A range brush can select an interval for minimum, mean, maximum, and transferred-byte figures.
Show valid coverage and gaps for interval calculations.
Do not offer a 24-hour range without the required retention.

The LibreQoS sources confirm this linked hover behavior.
The proposed range controls are a LQoSync design extension, not a claim about every LibreQoS chart.

## Backend readiness

| Feature | Available evidence | Work needed for the web UI |
| --- | --- | --- |
| Three traffic histories | Actual and classification histories in core/TUI caches | Typed Mapped/Unmapped browser series with timestamps, quality, and units |
| Longer traffic ranges | Core retains up to 3601 points | Bounded window API; current overview sends at most 300 Actual points |
| Compact DPDK summary | Existing optional DPDK overview | Typed numeric metrics for chart use and required counter histories |
| Queue Dynamics | Subscriber cache and circuit queue snapshot | Identity-scoped API, aligned history, capacity, and verified RTT semantics |
| Six Queue Stats panels | Native circuit queue diagnostics, counters, and optional tins | Authenticated bounded watcher/history, deltas, capability flags, and gaps |
| Live Queue Tree | Configuration topology and applied identity checks | Verified live ancestor identity and per-node metric contracts |

Relevant local sources are `src/worker_util/traffic.rs`, `src/worker_util/classification.rs`, and `src/subscriber_stats.rs`.
Circuit diagnostics use `src/circuit_queue.rs` and `src/circuit_queue/worker.rs`.
The web contracts are in `crates/web-contract/src/lib.rs` and `crates/web-contract/src/dpdk.rs`.
The current DPDK projection is in `crates/web-gateway/src/dpdk.rs`.

The broker response limit is 64 KiB.
Optional DPDK row details already have a 16-KiB budget.
Do not append full classification histories without a measured response budget.
Use a bounded traffic window response or a separately budgeted stream message.
Keep the current freshness and server-time rules.

Circuit queue access must resolve a circuit against the applied mapping generation.
The native implementation checks the mapping before and after the read.
Keep that protection in the web path.
Do not accept arbitrary queue slots or socket paths from the browser.
Bound active watchers and stop them when the view closes.

## Rust implementation direction

Recreate the interactions with the current Rust Leptos/Wasm components.
Start with bounded SVG paths and a Rust cursor/inspector model.
Use the LibreQoS chart engines as references, not automatic dependency choices.
A later performance result can justify a separate renderer decision.
Review upstream licenses before any direct source reuse.

Build paths when source data, the visible range, or the enabled series changes.
Pointer movement must read local data without a network request.
Coalesce pointer updates with animation frames.
Suspend unnecessary rendering for hidden views.
Bound sample counts, response size, and watcher count.
Do not smooth away spikes, bridge gaps, or replace missing values with zero.

## Atomic implementation proposal

1. Define the typed traffic window contract and its response budget.
   Check source meanings, independent timestamps, missing values, resets, and maximum payload size.

2. Build the mirrored comparison chart within the current traffic card.
   Check scales, gradient capacity anchors, above-capacity samples, and source focus.

3. Add the shared inspector and time selection.
   Check rolling-buffer stability, gaps, touch, keyboard access, and range coverage.

4. Refine the compact DPDK summary and its full diagnostic details.
   Check counter intervals, source ages, narrow screens, and all three theme modes.

5. Define a separate circuit queue contract before the subscriber queue screen.
   Check mapping generation, watcher bounds, disconnect cleanup, and unsupported metrics.

6. Add Queue Dynamics and Queue Stats after their contracts pass validation.
   Add live Queue Tree only after live ancestor metrics exist.

Each implementation step needs focused data tests and browser evidence.
Compare figures with the TUI at matching source timestamps.
Record missing backend functions instead of simulated production values.
This research session changes documentation only.
