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

## Task 3: extended worker measurements

Workers adds reported idle, laps/s, busy laps, cycles per busy lap, and ten named phase counters.
The approved cards, filters, table/detail split, pressure panel, and status remain in the same order.
Click the detail pane or use Tab to focus it. Wheel, scrollbar, Up/Down, PageUp/PageDown, Home, and End scroll details.
Use `e` to expand and `c` to copy. The title retains the worker ID and snapshot state during scrolling.
Copy output includes the same units, stale labels, and cache notices.

Laps/s remains explicitly producer-reported. Three live samples equaled busy ppm.
Do not treat it as independently verified processing throughput or packets/s.
Reported idle stays separate from the derived idle bar. Phase values are raw cumulative TSC cycles.
The verified order is `rx`, `dissect`, `select`, `charge`, `leaf`, `tx`, `begin_cycle`, `drop_overdue`, `handoff`, `empty_rx`.
These totals do not show interval percentages. Zero remains a valid reported value.
Missing fields stay unknown. Invalid idle or phase length does not hide valid basic worker values.
Stale values retain a last-known label. Freshness describes the complete snapshot.

The implementation reuses one snapshot poll. It adds no dependency or runtime configuration key.
If extended values exceed the 1 MiB public-cache budget, publication omits them with an explicit reason.
Basic workers and all three histories remain available. The source report stays intact in memory.
A later smaller report restores extended fields. An oversized basic report cannot replace the previous cache.

Actual terminal captures use isolated synthetic records:

- [Worker details](captures/workers-detail-v2.png)
- [Expanded phase counters](captures/workers-phases-v1.png)

A separate root-only probe confirmed all optional fields and advancing timestamps across three samples.
No production configuration, service, or binary changed.
Source contract: [LibreQoS analytics IPC](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/lib.rs).

The maximum fixture used 4,096 workers and three full histories. Fallback measured 739,546 bytes.
One debug publish/read sample took 154.6 ms. This is a synthetic measurement, not a production guarantee.
Verification: 576 tests passed, zero failed, seven manual tests ignored.
Formatting, all-target Clippy with warnings denied, debug build, and capture review passed.

## Next checkpoints
4. Bridge health, uptime, and degraded subsystems.
5. Subscriber traffic, RTT, and retransmissions.
6. On-demand circuit queue diagnostics.

Each checkpoint requires its own tests, review, documentation, and local commit.
