# DPDK Dashboard card

Status: implemented and verified. Runtime installation is pending.

## Scope and sources

Add one read-only card below the traffic panels and before upstream allocation.
Use the existing Rust web UI, authenticated overview stream, and broker.
Keep the TUI and the core collectors unchanged.

The TUI reads these published caches:

| Source | TUI code | Freshness limit |
| --- | --- | --- |
| Worker analytics | `src/worker_util.rs` and `src/tui/workers/approved.rs` | 6 seconds |
| Bridge health | `src/worker_util/health.rs` | 15 seconds |
| DPDK NIC telemetry | `src/nic_stats.rs` | 15 seconds |

The source age uses server time, with elapsed time after receipt.
A different browser clock must not change the health result.
Source freshness is independent. Fresh analytics does not prove healthy bridge operation.

NIC RX/TX counters do not represent subscriber download/upload traffic.
Queue drops, pool drops, and NIC errors remain separate measurements.
Missing values remain unknown. A reported zero remains zero.

## Design

Preserve the Dashboard theme colors, Plex type, spacing, and panel treatment.
The card has a title, a source health label, and three content groups:

1. Worker utilization: current busy/idle bars, worker count, busiest worker, and a recent average-busy chart.

2. Buffer and queue pressure: pool and ring meters, drops, AQM marks, and backlog.

3. Bridge and NIC health: independently dated health, link state, negotiated speed, and counters.

Use a fixed 0–100 percent scale for busy/idle and pressure.
Label the busy trend as browser-session history. Do not claim stored history.
Do not animate chart geometry or announce every sample to assistive technology.

Provide text values and a table alternative to charts.
Keep details open while live updates arrive.
Stack the groups on narrow screens.

## Atomic implementation plan

1. Add optional, backward-compatible DPDK views to the browser contract.

2. Project only allowlisted cache fields. Validate bounds, versions, timestamps, worker IDs, and ratios.

3. Reuse the one-second worker read. Refresh bridge/NIC evidence within the existing five-second full projection.

4. Keep all-worker summary counts. Send at most 16 busiest workers and eight ports to the Dashboard.

5. Keep the IPC response below its existing 64 KiB limit. Bound optional details and counter names.

6. Add the card with worker bars, pressure meters, a bounded session chart, and static detail tables.

7. Preserve independent stale, unavailable, and degraded states. Reject future timestamps and invalid values.

8. Test the projection, age boundaries, missing data, reported zero, malformed caches, bounds, and reset indicators.

9. Test live changes, chart gaps, source expiry, failed connection, themes, keyboard access, and 320/768px layouts.

10. Build native web services and Wasm. Save evidence, update deployment notes, commit, and push.

## Limits

The Dashboard is a summary, not a replacement for the full TUI worker inspector.
Show the selected worker count and the total count when rows are capped.
The current collector has no persisted worker-utilization history.
A browser-session chart fills only from distinct fresh worker timestamps.
Longer history requires a backend history contract and retention policy.

A full Workers inspector needs an authenticated paged endpoint.
Neither contract exists in the current web monitoring slice.
The card exposes bounded current diagnostics and a session chart.
Do not infer per-WAN attribution or applied shaping from DPDK counters.

## Frontend guidance

Use Vercel frontend-design and Google modern-web-guidance.
Use semantic tables, named meters, keyboard-accessible details, and static chart geometry.
Keep current source values separate from last-known evidence.


## Implementation and checks

The optional `Overview.dpdk` contract preserves compatibility with older responses.
The broker reads analytics once per second through the existing telemetry path.
Bridge health travels with the analytics cache. The core samples it independently.
The full projection reads the NIC cache every five seconds.
The existing core samples NIC counters every ten seconds.

The worker total and mean use all validated rows.
The card shows eight worker bars and up to sixteen detailed worker rows.
The response includes eight ports and at most sixteen xstats per port.
Optional worker/port details have a sixteen-KiB budget.
The budget preserves source health and pressure verdicts.
A visible notice identifies capped diagnostics.

Counters use decimal text. Chart ratios use validated numeric values.

Eight projection tests cover freshness, reported zero, malformed data, counter resets, worker bounds, and response size.
Six native UI tests cover history, charts, and ticker activity.
The existing gateway tests passed.

Browser checks cover live values, retained details/selection, independent source states, one-hour client clock skew, themes, and narrow layouts.
The existing Dashboard/sign-in and ticker regressions passed.
Native web and Wasm release builds, Clippy, Rust 1.89, formatting, and whitespace checks passed.

The optional design-taste inspector is unavailable. Playwright supplied the browser checks and pictures.

A read-only probe against this host's published caches found one worker and two ports.
It reported live analytics/NIC data and healthy bridge status at that check.
One debug projection took approximately 24 ms. This is not a production latency benchmark.

See [browser evidence](evidence/dpdk-dashboard/README.md) and [installation](DEPLOYMENT.md#dpdk-dashboard-update).
