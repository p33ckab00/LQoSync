---
type: session
created: 2026-11-12
tags: [performance, telemetry, health]
status: complete
related: ["[[2026-11-12-telegram-delivery-verification]]", "[[HANDOFF]]"]
---

# Production performance observation

Completed: 2026-11-12T14:56:30+08:00 (Asia/Shanghai). Agent: Codex `/root`.
The owner approved production performance observation after Telegram delivery verification.

Observed on 2026-11-12 from 14:50:16 to 14:55:16 (+08:00).
The read-only observer sampled cgroup counters and public cache metadata every two seconds.
It made no RouterOS or native socket requests.

| Measurement | Result |
| --- | --- |
| Samples / complete sync cycles | 151 / 3 |
| Average CPU, relative to one core | 0.285% |
| Median / maximum CPU over a two-second interval | 0.082% / 6.661% |
| Service cgroup memory | 25.69–28.04 MiB |
| Sync duration, minimum / median / maximum | 157 / 159 / 164 ms |
| Sync errors / automatic restarts | 0 / 0 |
| Maximum worker / health cache age | 2.27 / 2.26 seconds |
| Maximum subscriber / NIC cache age | 10.23 seconds |
| Maximum collection age / freshness budget | 120.54 / 190 seconds |

All six caches remained readable. No telemetry cache reported stale data.
No health component exceeded its freshness budget.
The known allocation difference was the only nonhealthy component.
Memory pressure and OOM event counters did not increase.
These results show no sustained CPU or memory bottleneck in this window.

They do not prove behavior at a larger subscriber count or under a burst of writes.
The window contained no managed-file change. It does not benchmark Telegram sends or TUI render time.

The separate I/O window lasted 267 seconds and recorded 54,505,472 service cgroup write bytes.
That is 199.36 KiB/s, or 16.43 GiB/day if the same rate continues.
The daily figure is an extrapolation, not a full-day measurement.
I/O pressure increased by only 105 microseconds, so the write volume did not show I/O saturation.

The worker cache contained 1801 history points and occupied about 401 KB.
The observer saw 150 cache versions in five minutes.
Source inspection confirms full JSON serialization and atomic replacement every two seconds.
This write pattern is consistent with most of the measured service write rate.
The cgroup total does not attribute each byte to a specific file.

The next optimization candidate is a RAM-backed runtime cache for worker telemetry.
Preserve the two-second refresh and the current history window.
Keep managed outputs, operation receipts, and Telegram delivery records durable.
A change needs compatibility checks for cache readers, service startup, and stale-data behavior.
No implementation or measured reduction is part of this observation.

Evidence: `/home/pi/.local/state/lqosync3/performance-observation/`.
`analysis.json` contains the calculated results. `samples.jsonl` contains metadata samples.
The observer excludes credentials, subscriber inventories, and message bodies.


## Verification and scope

Observer and analyzer syntax checks passed.
Result assertions passed for sample count, completed cycles, cache access, freshness, stable PID, and cycle errors.
The service stayed active as PID 882084, with zero automatic restarts.
No code-test run was necessary because product code did not change.
Documentation passed the diff whitespace check.

Only documentation and this session's private observation scripts changed.
No configuration change, forced sync, managed-file write, API send, service restart, deployment, commit, or push occurred.
Normal service work continued during observation.
The owner still defers the LibreQoS beta allocation issue.
Preserve unrelated profile/WAN work and prior uncommitted documentation.

## Next action

The proposed next task is to design and implement the RAM-backed telemetry cache. Preserve reader compatibility and freshness.
The observation task is complete. No cache migration has started.
