---
type: session
created: 2026-11-09
tags: [session, libreqos-v3, integration]
status: done
related: ["[[2026-11-03-libreqos-v3-package-update]]", "[[libreqos-v3-ingress-facts]]"]
---

# LibreQoS beta19 integration review

## Scope

Review the new LibreQoS changes for possible LQoSync3 integration.
The installed packages report `3.0.7.202611082352`.
The upstream comparison is `34342a28...9ebe29bc`, the beta19 public source projection.
This review did not prove that the package binaries match that commit exactly.

## Findings

- The upstream importer now retains `downloadBandwidthMbpsMin` and `uploadBandwidthMbpsMin` through canonical state reconstruction.
- An AP with child nodes now remains their logical parent. This preserves the AP ceiling over its subtree.
- LQoSync3 currently writes node ceilings only in `src/topology.rs:292`. Optional node minimums are a possible extension.
- Bridge analytics now include worker utilization, phase cycles, buffer exhaustion, pool drops, and handoff ring occupancy.
- The browser route `/local-api/cpu-workers` adds `utilization` with `busy_ppm`, `idle_ppm`, and `phase_cycles`.
- The public `/api/v3/cpu_workers` handler does not add that utilization section in the reviewed source.
- A service integration needs a supported telemetry interface. Do not assume both CPU routes expose identical fields.
- The package installer now caps hugepage allocation at 75 percent of total RAM by default.
- The setup code updates the license path. Trial signup now starts license delivery polling.
- The API move into `lqosd` predates this update. See the November 3 session.

## Installed state

The existing `/var/lib/lqosync3/native-probe.json` cache names the new package version and `full` compile mode.
Its state is `blocked`: `node PROBE-SITE compiled to 1/1 Mbps, expected 200/100`.
The status file reports `v3_pair`. This review did not rerun the probe or switch the source.
Keep the native guard until isolated compile and runtime checks prove that node policies survive.

## Suggested order

1. Add optional node minimums to the pair writer and configuration. Verify compiled minimums and parent limits.
2. Add worker utilization to the TUI after the service telemetry interface is confirmed.
3. Add buffer and ring pressure alerts after the telemetry interface is confirmed.
4. Validate nested AP policies with an isolated fixture before use.

## Checks and limits

The `lqosync3` branch at `dd5b95e` had a clean worktree before this note.
The existing project graph supplied context. Current files confirmed the relevant code paths.
No application code, live input, configuration, or service state changed. No application tests ran.

Source: https://github.com/LibreQoE/LibreQoS/compare/34342a28...9ebe29bc
