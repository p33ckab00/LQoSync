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

## Task 4: bridge health, uptime, and degraded subsystems

Dashboard health adds source status, version, reported uptime, topology tier, classification state, and named degraded subsystems.
The panel also shows aggregate worker rollup/AQM evidence. The content remains wrapped and scrollable.
Workers adds the same whole-bridge summary inside its existing detail pane and copy output.
The approved cards, chart/health split, worker table/details, pressure/status panels, and footer retain their order.
No new navigation route or privileged UI action exists.
Health copy also works when no worker row exists.

Health has its own LIVE/STALE/unavailable labels and update age. Snapshot freshness remains separate.
The first read and a lower uptime require a new advancing baseline. Duplicate uptime and failed reads cannot refresh health.
Expiry is fifteen seconds. Uptime and rollup age remain observations at read; neither is extrapolated.
Missing fields and empty topology tiers show unknown. An empty degraded-subsystem list means none reported.
Unknown source statuses remain source text. No CPU or snapshot-age heuristic invents a health verdict.

Actual terminal captures use isolated synthetic records:

- [Dashboard health](captures/dashboard-health-v1.png)
- [Expanded Workers health](captures/workers-health-v1.png)

The fixture deliberately shows degraded `stats_summary` to verify the named-subsystem display.
It is separate from the operator's three live samples, which reported healthy, no degraded subsystems, and zero AQM-degraded workers.
Live uptime advanced from 44,962,108 to 44,966,115 ms. One worker had observed rollup ages of 84–93 ms.
The source topology tier was empty and remains unknown in the UI.
These samples do not prove long-term bridge health or production activation of this implementation.
Source: [LibreQoS health contract](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/lib.rs).

One monitor requests health every ten seconds alongside the existing two-second snapshot cadence.
Due requests run concurrently with separate outcomes and the existing transport bounds.
The public cache retains only an aggregate summary. Strings, subsystem counts, and worker counts are bounded.
The maximum fixture includes 4,096 workers, three full histories, and a full bounded health summary.
Cache fallback measured 742,194 bytes. One debug publish/read sample took 155.3 ms.
These synthetic measurements are not a production performance guarantee.

Verification: 585 tests passed, zero failed, seven manual tests ignored.
Formatting, all-target Clippy with warnings denied, build, source probe, and terminal capture review passed.
Tests cover request kinds, independent health/snapshot failures, cadence, cache compatibility, expiry, duplicate uptime, restart, missing fields, bounds, and stale copy.
No push, deployment, restart, new dependency, or runtime key belongs to this checkpoint.

## Task 5: subscriber traffic, RTT, and retransmissions

Subscribers keeps the approved cards, filters, table/details split, expanded details, actions, and footer.
The selected detail pane adds per-IP delivered rates/bytes, RTT sample medians, and observed TCP retransmission counters.
The title keeps the selected subscriber visible during scrolling. Wrapped text has exact scrollbar bounds.
Mouse focus, wheel, scrollbar, Up/Down, page keys, Home, End, expansion, and copy remain available.

The default view shows an unassigned collection record as unavailable. The expanded view shows an exact shaping match.
Actual terminal captures use isolated synthetic records:

- [Default Subscribers layout](captures/subscribers-dpdk-default-v1.png)
- [Expanded DPDK observations](captures/subscribers-dpdk-v1.png)

The source caps shaped hosts at 512 for this sampler. Missing hosts and fields remain unknown.
No host sum becomes a bridge or whole-circuit total. No absence changes collection state.
RTT is the median of retained nonzero samples, with pooled and directional fields kept separate.
TCP retransmissions are observed cumulative counters. They do not claim packet-loss percentages.
Stale observations keep their last-known label and hide current rates.

The operator's three read-only samples contained 26 shaped hosts. All 26 matched applied circuit names and exact IPs.
The probe verified one-based slot mapping, advancing timestamps, delivered-byte fields, and pooled/directional TCP fields.
It printed only counts and source timestamps, without subscriber identities.
The live CSV had no routed-prefix or IPv6 fields. IPv6 canonicalization is covered by isolated tests.
Sources: [host contract](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/lib.rs), [applied-state contract](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/ipc.rs).

A bounded sampler reads state/snapshot/state every ten seconds. State generations and slot names must agree.
Two stable observations and producer progress establish attribution. Remaps, errors, resets, duplicates, and gaps cannot invent rates.
The sampler shares monitor reload/shutdown ownership. Aggregate snapshot cadence, limits, history, and cache remain unchanged.
A separate 1 MiB public cache stores only bounded host summaries. Each source reply keeps the 2 MiB transport bound.
Exact circuit/IP matching rejects ambiguous local ownership, including overlapping prefixes and projected collection-only rows.
Non-host prefixes remain unsupported. At most sixteen exact host addresses are supported per row.

The maximum 512-host cache fixture measured 351,809 bytes. One debug publish/read sample took 35.1 ms.
A 75,000-row ownership index took 634.9 ms in one debug run. Index work occurs on row changes, not on frames.
Projected ownership is cached across telemetry-only updates. Frames use bounded selected-host lookups.
These synthetic measurements are not production performance guarantees.

Verification: 596 tests passed, zero failed, seven manual tests ignored.
Formatting, all-target Clippy with warnings denied, build, source probe, and terminal capture review passed.
The suite covers source decoding, slot order, mapping changes, counter resets, omissions, optional fields, ownership conflicts, cache bounds, lifecycle, copy, and scrolling.
No deployment, restart, or production configuration change occurred.

## Task 6: on-demand circuit queue diagnostics

The approved Subscribers cards, filters, table/detail split, and collection status remain in their approved order.
The existing detail pane adds one manual action: click `Read / refresh queue diagnostics` or press `d`.
Wheel, scrollbar, click focus, page keys, expansion, and copy retain their existing paths.
The action works in read-only mode when the operating system permits socket access.
The installed root-only socket requires a root TUI. A permission error explains this requirement.

The reader requests one nonzero slot with `queue_stats`, `limit = 1`, and one `queue_slots` entry.
It reads the applied state before and after the queue request. Generation and authoritative slot order must agree.
The source must return exactly the requested row. Collection-only records cannot request queues.
One request can run per TUI. A two-second cooldown bounds repeated clicks.
A cancellable background worker keeps socket reads outside input handlers and frame rendering.
Each source request retains the two-second deadline and 2 MiB bound. The job also has a seven-second deadline.
There is no automatic queue poll, cache writer, daemon endpoint, permission change, or new dependency.

The view shows separate directional depth, drop counters, queue-full drop counters, and AQM marks.
Each direction permits at most sixteen tins. Tins show sent bytes, backlog bytes, and reported p99 sojourn.
Sojourn converts nanoseconds to milliseconds. It does not represent ping or host RTT.
The source does not document the unit of `memory_used`. The view labels it as a reported raw value.
The pooled RTT histogram has fifty bins. Bins cover ten milliseconds each, with a final `>=490 ms` bin.
The view shows nonempty bins and the reported sample count. It does not derive percentiles or interval rates.

Freshness uses both source time and response time. Missing timestamps cannot establish freshness.
Older observations, in-flight refreshes, and failed refreshes retain a last-observed label.
Gap/reset flags, source clock skew, missing fields, and malformed histograms remain explicit.
A result for another circuit cannot appear under the current selection.

Actual terminal captures: [default composition](captures/subscribers-queues-default-v1.png) and [expanded circuit queues](captures/subscribers-queues-v1.png).
The capture uses an isolated synthetic analytics socket. It is not a live network measurement.
The approved composition stays intact. Queue controls occupy the existing detail-action area.
The expanded capture includes source age, sequence, queue counters, and tin observations with explicit units.
The persistent detail title keeps the queue state visible during scrolling.
The default capture also verifies that a collection-only record has a disabled queue control.

The operator ran three read-only samples against the installed endpoint.
Each returned one matching slot, eight tins per direction, and fifty RTT bins with eight samples.
The sample sequence advanced from 243961 to 243971 to 243981. Elapsed time and cadence both reported 200 ms.
The epoch stayed stable. All three reported `actor_gap = false` and `counter_reset = false`.
Direction counters reported zero depth, drops, and marks during these observations.
These short observations do not establish long-term health or absence of congestion.
Evidence: `/home/pi/.local/state/lqosync3/dpdk-circuit-task6/live-source.jsonl`.
SHA256: `ee882dc464805eaaf91432bca8d5906dc9c5eecc2d8bab5d9df24d29b539b472`.
Source: [LibreQoS queue contract](https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/lib.rs).

All six checkpoints have local implementations. Publication and deployment require a later operator request.


Verification: 606 tests passed, zero failed, seven manual tests ignored.
Formatting, all-target Clippy with warnings denied, build, whitespace, and terminal capture review passed.
The tests cover mapping changes, reply identity, unknown fields, histogram bounds, age, request throttling, and cancellation.
They also cover mouse activation, selection changes, copy, and access from a read-only TUI.
A synthetic fixture with sixteen tins per direction and fifty nonempty bins produced 5,212 bytes of detail text.
One thousand debug formats took 61.0 ms. This fixture measurement is not a production performance guarantee.
No production binary, configuration, service, permission, or public telemetry cache changed.
