# Aggregate traffic comparison

Date: 2026-11-20 (Asia/Shanghai).
The owner approved the traffic consolidation from [the research proposal](MIRRORED_TRAFFIC_RESEARCH.md).

## One panel, three sources

The current `Traffic (Aggregate)` panel contains three telemetry sources.
It does not add another DPDK traffic card.
The DPDK health card remains available for worker, pressure, and NIC diagnostics.

| Source | Meaning | Appearance |
| --- | --- | --- |
| Actual | Whole-bridge delivered traffic | Rainbow area and solid outline |
| Mapped | Whole-bridge mapped enqueue traffic | Amber dashed outline |
| Unmapped | Whole-bridge unmapped enqueue traffic | Violet dotted outline |

Actual is the same source that supplied the original aggregate plot.
Mapped and Unmapped come from classification histories in `worker-utilization.json`.
The chart does not add these rates or infer them from subscriber rows.

Download appears above zero. Upload appears below zero.
The data contract keeps both rates positive.
All series use one time domain.
Each direction has its own rate scale and configured capacity anchor.

Red shows that direction's combined configured WAN capacity.
If a sample exceeds capacity, the axis extends and the capacity line remains visible.
If capacity is unavailable, the chart uses observed scales and an explicit label.

## Inspection

Each source button controls both download and upload curves.
Source controls remain stable through live updates.
The inspector retains figures for all sources, even when their curves are hidden.

Hover over the chart to inspect one timestamp.
Tap the chart to pin that timestamp.
Use Left/Right on the focused chart to select another sample.
Use Escape or Live to clear the selection.
A live refresh does not replace a pinned timestamp with another array position.
If that timestamp leaves the retained window, its figures show `—`.

The inspector uses exact published timestamps.
It does not interpolate a missing sample or borrow another source's point.
Zero displays as `0 bps`. Missing values display as `—`.
Independent state labels identify observed, stale/gap, and unavailable sources.

A collapsible table supplies the retained samples for all three sources.
The table creates its rows only when open.

## Data and performance

The authenticated overview stream remains the transport.
The broker reads the same worker cache as the TUI.
It does not open a collector socket or read private subscriber records.
The core and TUI do not change.

Each source starts with at most 300 recent observations.
Invalid rates create a discontinuity. Time gaps also split the curves.
Classification freshness uses a six-second limit and its own source availability.
The current Actual freshness rule remains unchanged.
A connection failure marks the retained sources stale.

The complete overview has a 60-KiB target budget.
This leaves space for the broker reply envelope within the 64-KiB IPC limit.
If necessary, the projection removes the oldest samples across the series.
The panel reports this reduction.
The broker also rejects an oversized reply before transmission.

The Rust UI caches the time index and SVG geometry between observations.
Pointer movement changes the inspector only when its selected timestamp changes.
It does not rebuild the paths or send a network request.
No chart engine or handwritten JavaScript application code was added.

## Scope limits

Only the Recent window is available in this release.
Longer ranges need a bounded window contract.
This change does not implement per-WAN attribution or per-subscriber queue screens.
Queue Dynamics, Queue Stats, and live Queue Tree remain separate tasks.

## Verification and deployment

See [browser evidence](evidence/aggregate-traffic/README.md) for checks and screenshots.
See [the atomic plan](plans/2026-11-20-aggregate-classification.md) for the authorized scope.
Vercel frontend-design and Google modern-web-guidance supplied interface guidance.

Build from the repository root with `tools/frontend/build.sh`.
Install the broker, gateway, and browser assets together with `sudo tools/frontend/install.sh`.
No core service replacement is necessary for this chart change.
