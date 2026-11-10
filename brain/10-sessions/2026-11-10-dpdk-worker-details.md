---
type: session
created: 2026-11-10
tags: [dpdk, telemetry, workers]
status: implemented-not-deployed
related: ["[[2026-11-10-dpdk-mapped-unmapped]]", "[[HANDOFF]]"]
---

# DPDK statistics: task 3

Updated: 2026-11-10T20:01:01+08:00 (Asia/Shanghai).

The owner approved the next checkpoint and ran the root-only worker probe.
Three aggregate samples confirmed optional idle, laps/s, busy laps, cycles/busy lap, and ten phase counters.
Producer timestamps advanced. Evidence: `/home/pi/.local/state/lqosync3/dpdk-worker-task3/live-source.jsonl`.
The probe includes no subscriber or flow identity.

## Implementation and review

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

Implemented and locally verified. 576 tests passed, zero failed, seven manual tests ignored.
Formatting, all-target Clippy, debug build, and terminal capture review passed.
The operator probe verified all five optional fields, ten phases, and advancing producer timestamps.
Reported laps/s equaled busy ppm in the three samples. Keep its producer-reported label; do not infer packets/s.
Cache fallback preserves basic telemetry and histories within 1 MiB without mutating the source report.
The maximum fallback fixture measured 739,546 bytes and 154.6 ms for one debug publish/read sample.
No push, deployment, or restart occurred. Task 4 bridge health, uptime, and degraded subsystems is next.

Initial regression tests failed before implementation. The final suite includes cache preservation, source decoding, missing/invalid fields, duplicate frames, stale copy, and scrolling checks.
Captures: `docs/design/tui/captures/workers-detail-v2.png` and `workers-phases-v1.png`.
They use isolated synthetic records, not production telemetry.
Logs: `/home/pi/.local/state/lqosync3/dpdk-worker-detail-*.log`.
Source: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/lib.rs
Preserve the unrelated local-profile discussion in the vault.
