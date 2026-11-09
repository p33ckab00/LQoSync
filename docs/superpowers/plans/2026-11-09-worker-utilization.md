# Worker utilization implementation plan

Goal: Show actual DPDK worker utilization in the TUI without changes to shaping or worker counts.

Read the bridge's read-only analytics snapshot with `limit=1`. Ignore host and flow data.
Use a bounded CBOR decoder and descriptor transport, with a two-second request timeout.
Use `nix`, already present in the lockfile, for descriptor transfer. Add `ciborium` for CBOR.
The public HTTP API does not expose the required fields in beta19.

- [x] Add tests for old frames, invalid counters, repeated timestamps, reconnects, and stale cached data.
- [x] Implement bounded IPC and tests with real local sockets and descriptor transfer.
- [x] Poll every two seconds in one service task. Stop on process exit. Apply config reloads.
- [x] Publish only worker rows to a separate atomic cache. Preserve last-good values on read failure.
- [x] Add Workers tab 6 with busy, idle, backlog, sample age, and explicit unavailable states.
- [x] Preserve the existing dashboard journal counters and tabs 1 through 5.
- [x] Test transport failures, lifecycle, narrow terminals, and all current color themes.
- [x] Review performance and integrity. Run full tests, clippy, and formatting. CI follows the push.
- [x] Commit and push to `lqosync3`. Build, deploy, restart, and verify the active version and fresh production telemetry (`3.0.142+g8ee064d`).

Freshness needs an advancing producer timestamp. The first frame establishes the baseline.
Repeated timestamps become stale after six seconds. TUI cache age also expires after six seconds.
No unavailable field becomes a fabricated zero. No bridge restart, API key, or new daemon is required.
