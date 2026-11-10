---
type: session
created: 2026-11-10
tags: [dpdk, telemetry, performance, deployment]
status: deployed-verified
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


## Publication and prepared artifact

Updated: 2026-11-10T23:19:10+08:00 (Asia/Shanghai).
Code commit `828061e` and all six earlier DPDK commits are on GitHub `lqosync3`.
CI run `42841241848` passed test and MSRV jobs.
URL: https://github.com/p33ckab00/LQoSync/actions/runs/42841241848.

The clean detached build reports `3.0.166+g828061e`.
Binary SHA256: `29ae89c2f5eac09b9a2597010460450d439ba8f43e6b0fb440d1b2e8de9a173b`.
The artifact TUI passed all seven routes, clean quit, and terminal cleanup against the live read-only configuration.
The smoke harness forces complete redraws. Differential terminal output can omit unchanged label letters.
The initial harness failures did not indicate a product failure.

Artifact directory: `/home/pi/.local/state/lqosync3/dpdk-828061e/`.
It contains the clean binary, circuit probe, manifest, CI result, deployment script, and terminal smoke report.
The script verifies checksums, current config, previous binary, backup, restart, cycle, collection, and all six telemetry sources.
It restores the prior binary and restarts the service if post-install checks fail.

Non-interactive sudo failed before mutation because the host requires a password.
The operator has a pending request to run `sudo /home/pi/.local/state/lqosync3/dpdk-828061e/deploy.sh`.
Production remains on the prior release until this command succeeds.
After execution, verify `DEPLOYED.json`, live process/hash/version, cycle identity, fresh caches, circuit samples, and the installed TUI.
This documentation commit does not change the code in the reviewed artifact.


## Deployment complete

The operator ran the prepared script. Deployment succeeded at 2026-11-10T23:23:22+08:00 (Asia/Shanghai).
Independent verification passed at 2026-11-10T23:24:06+08:00.
Installed release: `3.0.166+g828061e`. Its checksum matches the reviewed artifact.
The deployment verified the running executable hash. The current service and status retain PID 510745 and the same version.
The service is active/running, with zero automatic restarts.

Cycle 1 succeeded in `apply` mode for `v3_pair`. No service or configuration error exists.
The configuration checksum is unchanged. Collection matches the new cycle and the raw source counts.
Deployment checks passed for all six telemetry sources, including the root-only circuit endpoint.
Independent checks confirm fresh aggregate, classification, health, and host caches.
Their timestamps and bridge uptime advanced after deployment. The bridge reports healthy, with no degraded subsystems.
The three circuit samples matched their slot and advanced. Their gap/reset flags were false.

The installed TUI passed all seven routes, clean quit, terminal cleanup, and no panic.
Both the reviewed code and its preceding documentation CI passed.
No rollback was required. Backup: `/var/backups/lqosync3/dpdk-828061e-5hq48cg8`.
Evidence directory: `/home/pi/.local/state/lqosync3/dpdk-828061e/`.
Evidence files: `DEPLOYED.json`, `independent-verification.json`, `deployed-circuit-source.jsonl`, and `installed-tui-smoke.json`.

The task is complete. Existing TUI sessions must reopen to use the installed release.
Direct circuit queue reads still require a root TUI on this host.
No production configuration or RouterOS policy changed as part of the deployment script.
