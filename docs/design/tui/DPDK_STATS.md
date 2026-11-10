# DPDK statistics checkpoints

Date: 2026-11-10. Branch: `lqosync3`.

## Task 1: queue totals

The Dashboard retains five summary cards, the chart/health split, WAN panel, and alerts.
Dataplane health adds whole-bridge queue drops, AQM marks, and backlog bytes.
Its scrollable wrapped content supports the mouse wheel, scrollbar track/thumb, and `j`/`k` keys.
Workers retains its cards, filters, worker table, detail pane, pressure panel, and status.
The detail pane adds explicitly labeled whole-bridge queue values after selected-worker measurements.
The detail pane retains expansion and keyboard focus. It also retains wheel and scrollbar controls.

Counters show cumulative values and increments per valid sample. Backlog is a current byte gauge.
Missing fields stay unknown. Stale values carry a last-known label and cannot show current increments.
AQM marks, queue drops, RX exhaustion events, and pool drops remain distinct.
No queue measurement claims a WAN-specific total or a selected-worker total.

Actual terminal captures use isolated synthetic records:

- [Dashboard](captures/dashboard-queue-v1.png)
- [Workers](captures/workers-queue-v1.png)

The captures preserve the approved screen composition. They do not prove production activation.
The root-only live probe verified all three fields. Producer timestamps advanced between samples.
The probe observed zero cumulative queue drops, 13,899 cumulative AQM marks, and backlog 95,550 → 6,526 → 0 bytes.
These are three observations, not a long-term dataplane health verdict.

The implementation reuses the current poll and timeout. It retains the snapshot size limit and cache writer.
It adds no request per frame, flow scan, router scan, or runtime dependency.
The Ratatui dependency enables rendered-line information for exact wrapped scrollbar bounds.
The lockfile remains unchanged.

## Task 2: mapped and unmapped traffic

Dashboard adds Actual, Mapped, and Unmapped buttons inside the traffic panel.
The keyboard shortcut is `v`. Mouse targets remain clipped to the visible panel after resize and scrolling.
The time-window controls follow the selected source.
Actual means delivered bytes. Mapped and Unmapped mean enqueue classes for the whole bridge.
The legend retains that distinction. Small class rates use three decimal places and a sub-Mbps axis scale.
Stale legends hide current rates while historical traces remain available.

The scrollable Dashboard health and Workers detail panes add class rates and cumulative bytes.
The copy action retains the scope and freshness labels.
The screen keeps the approved panel order. No WAN-specific classification or subscriber identity is inferred.

Actual terminal captures use isolated synthetic records:

- [Mapped chart](captures/dashboard-mapped-v1.png)
- [Unmapped chart](captures/dashboard-unmapped-v1.png)
- [Worker details](captures/workers-classification-v1.png)

A separate root-only read probe confirmed both counters on the installed bridge.
Producer timestamps advanced. Measured download rates were 96.296–98.683 Mbps mapped and 0.031–0.055 Mbps unmapped.
These observations do not establish long-term traffic proportions or a health verdict.

The collector reuses the snapshot poll and limits. Each class has a bounded independent history.
No class total comes from the capped host/flow rows or subtraction from Actual.
The maximum-cache fixture measured 740,161 bytes with 4,096 workers and three full histories.
A single debug sample took 35.9 ms to encode and 26.1 ms to read/decode. These are synthetic measurements.
No new dependency or runtime configuration key is required.

## Next checkpoints
3. More worker measurements.
4. Bridge health and uptime.
5. Subscriber traffic, RTT, and retransmissions.
6. On-demand circuit queue diagnostics.

Each checkpoint requires its own tests, review, documentation, and local commit.
