---
type: session
created: 2026-11-09
tags: [session, libreqos-v3, telemetry, alerts]
status: deployment-pending
related: ["[[2026-11-09-worker-utilization]]", "[[2026-11-09-libreqos-beta19-integration-review]]", "[[HANDOFF]]"]
---

# Buffer and ring pressure alerts

## Scope and changes

The user authorized priority task 3 and its code review.
The scope includes checks, a commit, a push, deployment, and a restart.

The existing two-second analytics snapshot poll now reads optional diagnostics.
`src/worker_util/pressure.rs` evaluates buffer and handoff ring pressure.
`src/worker_util.rs` maintains counter baselines and publishes additive cache fields.
`src/tui/workers.rs` shows pressure causes and measurements in Workers, key 6.
`docs/OPERATIONS.md` describes the thresholds, counter semantics, and journal command.

The feature adds no dependency or configuration key.
It adds no socket request or Telegram event.

The monitor warns when free pool buffers reach 10 percent or less.
Any ring above 75 percent, or aggregate ring use at 75 percent or more, also produces a warning.
These are operational thresholds, not a dataplane failure verdict.

`rx_nombuf` and `pool_drop_packets` increments produce separate warnings.
The monitor does not combine these counters into a packet loss total.
The source contract does not establish `worker_push_drops` as packet handoff loss. This feature does not use that counter.
Source: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc/rust/libreqos_dpdk/analytics_ipc/src/lib.rs

## Integrity and performance review

Counter baselines reset after read failures, producer restarts, and gaps above six seconds.
If a counter decreases, its increment shows `unknown`. Repeated frames do not replay increments.
Missing or invalid diagnostics remain unavailable. They do not hide valid worker utilization.

Zero pool capacity remains unknown. A topology with zero rings can show `no rings`.
Stale snapshots show unknown current pressure, never an active warning or recovery verdict.
Only state changes produce journal messages. Changes to counts alone do not repeat a warning.

The TUI uses the current themes and formats only visible worker rows.
The transport retains its two-second deadline, 2 MiB snapshot bound, and descriptor ownership rules.
The public cache retains mode 0644 and its 1 MiB read bound. It contains no host or flow records.

Review found no unresolved code issue. The review did not measure live throughput.

## Checks

- Full suite: 511 passed, zero failed, six manual tests ignored.
- `cargo fmt --check`: passed.
- `cargo clippy --all-targets --locked -- -D warnings`: passed.
- Regression tests first failed for absent deltas, gap handling, zero-ring support, and cache isolation.
- UI tests cover stale data, missing fields, all three themes, and narrow terminals.
- Transport tests confirm diagnostic fields survive real socket descriptor transfer.
- CI and deployment: pending after commit and push.

## Live state and next action

Before deployment, the active service runs `3.0.142+g8ee064d`, PID 99607, with `v3_pair`.
Passwordless sudo is unavailable. The operator must run the prepared deployment script with sudo.
The script must verify its artifact, back up the binary, restart, and check the new cycle and telemetry.

The implementation plan is `docs/superpowers/plans/2026-11-09-buffer-pressure.md`.
The existing graph predates worker telemetry. Current code and this note take precedence.
