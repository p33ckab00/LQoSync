---
type: session
created: 2026-11-10
tags: [dpdk, telemetry, performance, deployment]
status: reviewed-ready-to-deploy
related: ["[[2026-11-10-dpdk-circuit-queues]]", "[[HANDOFF]]"]
---

# DPDK performance and deployment review

Updated: 2026-11-10T23:07:32+08:00 (Asia/Shanghai).

The owner authorized review of all six DPDK checkpoints, improvements, commit/push, and deployment.
The review traced the source transport, bounds, caches, reset rules, sampler lifecycle, and TUI callers.
The load assumptions are one service monitor and one manual circuit request per TUI.
Limits remain 4,096 workers, 512 hosts, and 1,801 points per traffic history.

## Changes

A regression test reproduced an aggregate read that published after configuration reload.
The monitor now rejects a result from superseded settings before publication.
Shared rate formatting prevents small positive traffic from rounding to zero in the chart and detail views.
Display units can be bps, kbps, Mbps, or Gbps. Cache values stay in Mbps.

The chart shows source scope, sample age, window peaks, and valid-sample counts.
Axis labels match their plotted bounds. Missing samples and long timestamp gaps break lines.
Current rates remain unknown during a source failure. Historical peaks keep their window label.
Operational alerts now include reported bridge degradation and stale health.
Long notices wrap within the existing mouse scroll controls.

## Checks

613 tests passed. Zero failed. The suite ignores seven manual tests.
Formatting, all-target Clippy with warnings denied, build, whitespace, and terminal capture review passed.
The 1,801-point chart fixture took 770.1 ms for 100 debug redraws at 240x40 cells.
This is about 7.7 ms per redraw. It is a synthetic debug result, not a production guarantee.
The render path performs no network or file reads. Poll intervals and source permissions remain unchanged.

Review: `docs/design/tui/DPDK_REVIEW.md`.
Logs: `/home/pi/.local/state/lqosync3/dpdk-review-*.log`.
Captures use isolated synthetic files and preserve the approved panel order.

## Deployment handoff

Next actions are push, CI verification, a clean release build, and a deployment with backup and rollback.
The current service is active, PID 61153, version `3.0.157+g39459a7`, with no cycle or configuration error.
Non-interactive sudo requires a password. Prepare the complete deployment before requesting the operator's terminal action.
Verify the installed checksum, process identity, new cycle, collection, aggregate/health/host caches, and circuit source after restart.
Preserve the unrelated local-profile Knowledge link and note.
