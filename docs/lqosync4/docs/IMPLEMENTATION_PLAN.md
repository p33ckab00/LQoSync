# LQoSync4 implementation plan — draft

Do not clone the future branch during planning. Confirm the target branch/path before code work.

| Phase | Work | Exit gate |
|---|---|---|
| 0. Baseline | Record LQoSync3 commit, tests, config/state formats, and MikroTik core write contract. Confirm remaining owner features. | Behavior matrix and sample read-only RouterOS data. |
| 1. Core port | Remove TUI command/module/dependencies. Preserve sync, shadow/apply, validation, atomic writes, and LibreQoS v2/v3 targets. | Golden output parity, config migration, fresh install and rollback tests. |
| 2. Web foundation | API, auth, roles, audit, migrations, responsive shell; read-only dashboard, routers, subscribers, LibreQoS preview. | Real data on all read screens, stale/error states visible, no secret exposure. |
| 3. Profiles | IPoE/PPPoE profiles, baseline speed, scheduled overrides, effective-speed preview and sync. | Midnight, overlap, timezone, restart and idempotency tests. |
| 4. Assignment | Upstream model, capacity, stable percentage/count/manual engine, dry-run, Allocations/Assignments/Discovery/History views. | Deterministic decisions, manual precedence, no unexplained movement. |
| 5. Health and apply | Health states and hold-down; narrow RouterOS writer with readback, retry, audit and rollback. | Outage/recovery and partial-failure simulation; reviewed shadow output before live apply. |
| 6. Telemetry | Traffic utilization history, then destination IP/ASN/organization/CDN/app roadmap. | Freshness, load, privacy and classification accuracy measured. |

Each phase delivers a working slice with relevant tests. Implementation does not imply production deployment.
