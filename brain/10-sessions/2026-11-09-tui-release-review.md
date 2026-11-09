---
type: session
created: 2026-11-09
tags: [tui, performance, release]
status: deployment-pending
related: ["[[2026-11-09-approved-tui-implementation]]"]
---

# TUI release review

Updated: 2026-11-09T18:38:29+08:00 (Asia/Shanghai).

## Authorization and scope

The owner requested another performance review, then commit/push to `lqosync3`, deployment, and a service restart.
This replaces the earlier local-only restriction. Review covers the eight local TUI commits and connected data/apply paths.
Use the existing service configuration. Replace only the binary, with a rollback copy and post-restart checks.

## Review and correction

Visible rows bound table drawing. Source changes invalidate cached filters and previews.
History retains at most 1,801 points. Journal retention, the receive channel, and each receive batch remain bounded.
Drawing performs no file, socket, or RouterOS I/O. Settings retains preview, secret, conflict, backup, and reload verification gates.
No unresolved performance issue was found at the reviewed load.

Review found one integrity defect: a wall-clock regression during a failed analytics read left future points before the new gap.
That history failed cache order validation. Reset history and the rate baseline before accepting a regressed wall time.
The new regression test failed before the fix and passes after it.

## Performance evidence

A temporary benchmark ran the real App and TestBackend at 240x40 and 80x24, with 50,000 synthetic subscriber rows.
The fixture includes subscriber metadata and one router. Other panels use missing-data states.
Debug build: initial load 209.26 ms; unchanged snapshot mean 0.001 ms.
Thirty-frame means: 240x40 screens 5.599–7.703 ms/frame; 80x24 screens 2.013–4.871 ms/frame.
This measures in-memory TUI rendering, not live DPDK throughput or network/terminal latency.
The temporary benchmark source was removed. The evidence remains outside the repository.

## Checks and deployment

Full suite: 542 tests pass, zero fail, seven manual tests ignored.
All-target Clippy with warnings denied, formatting, and whitespace checks pass.
Evidence: `/home/pi/.local/state/lqosync3/tui-release-review/`.
Before deployment, the active service uses `/usr/local/bin/lqosync3`, version `3.0.144+g1d2b4d5`.
Its unit runs as root. Non-interactive sudo reports that a password is required.
Prepare a verified release and a concrete rollback/health-check script before asking for an operator step.
Deployment and restart remain pending until root execution succeeds. Do not claim live verification from fixture tests.
