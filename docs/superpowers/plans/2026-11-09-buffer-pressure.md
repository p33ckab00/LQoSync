# Buffer and ring pressure implementation plan

**Goal:** Show buffer and handoff ring pressure alerts in Workers and the service journal.

**Architecture:** Extend the existing two-second snapshot poll and public telemetry cache.
Use optional diagnostics fields. Keep the current transport bounds and freshness rules.
No new dependency, socket, service, configuration, or packet-path change is required.

**Reference:** [[2026-11-09-libreqos-beta19-integration-review]], priority 3.
Upstream contract: `analytics_ipc/src/lib.rs` at `9ebe29bc`.

## Acceptance

- New `rx_nombuf` and pool-drop increments produce warnings.
- A baseline, a counter reset, or a repeated snapshot produces no fabricated increment.
- Free pool capacity at or below 10 percent produces a warning.
- Any ring over 75 percent, or aggregate occupancy at or above 75 percent, produces a warning.
- These operational thresholds describe pressure, not a dataplane health verdict.
- Missing or invalid diagnostics show unknown/unavailable, independently of worker utilization.
- Stale telemetry never shows an active warning or a healthy pressure verdict.
- Journal messages occur on state changes. The TUI shows current measurements and alert causes.
- The implementation does not send new Telegram messages or change existing notification policy.

## Steps

- [x] Add failing tests for counter baselines, increments, resets, gaps, invalid gauges, and old frames.
- [x] Add optional diagnostics and pressure evaluation in `src/worker_util/pressure.rs`.
- [x] Integrate pressure into the tracker, cache validation, and monitor transition logs.
- [x] Add TUI tests, then show the pressure state and measurements in `src/tui/workers.rs`.
- [x] Review descriptor bounds, freshness, log volume, overflow, and compatibility.
- [x] Run full tests, formatting, Clippy, and CI. Results: 511 tests pass, six manual tests ignored. CI run `42547035993` passes.
- [x] Commit and push `1d2b4d5` to `lqosync3`. Build `3.0.144+g1d2b4d5` and check the artifact hash.
- [ ] Operator runs the prepared deployment script with sudo. Check the restarted service and fresh diagnostics.

The existing graph predates worker telemetry. Current source and session notes take precedence.
Execute inline under the user's authorization. No separate plan approval is required.
