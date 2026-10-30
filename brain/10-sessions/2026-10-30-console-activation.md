---
type: session
created: 2026-10-30
tags: [lqosync-v3, activation]
status: complete
related: ["[[HANDOFF]]", "[[2026-10-30-flexible-topology]]"]
---

# Console activation

Updated: 2026-10-30T07:18:05+08:00 (Asia/Shanghai).
The user authorized the LQoSync v3 console restart and requested the next task.
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Active code: `ac4c922`.

The console now uses the verified binary on `127.0.0.1:9303`.
Its PID changed from `3152509` to `3379628`. The launch environment and data directory remain unchanged.
The previous executable remains in a private rollback directory.
The settings digest and credential file metadata remain unchanged.
The process IDs for the LibreQoS daemon, scheduler, and legacy sync remain unchanged.

The locked binary build passed. Health and six UI routes returned HTTP 200.
Fresh scheduled reads returned 14 and 12 inventory tables with zero errors.
Each of the ten automatic pools uses `online_clients` as its metric.
The saved candidate contains 26 clients with a 13/13 WAN split.

Live Validation passed the checks for inventory, speeds, and capacity.
Core membership remains Conflict. No committed generation exists.
Candidate assignments do not establish actual routing or applied QoS.

No router configuration or LibreQoS output changed. Application tests were not repeated for this restart.
These notes contain only totals from runtime checks. Files with private data remain ignored.
Next: review the hierarchy with the operator. Observe continuity across reconnects.
Review protected manual membership before a router migration.
See [[2026-10-30-flexible-topology]] and [[20-decisions/adr-0001-session-candidate-state]].
