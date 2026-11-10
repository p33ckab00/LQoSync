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

## Remaining checkpoints

2. Mapped and unmapped traffic.
3. More worker measurements.
4. Bridge health and uptime.
5. Subscriber traffic, RTT, and retransmissions.
6. On-demand circuit queue diagnostics.

Each checkpoint requires its own tests, review, documentation, and local commit.
