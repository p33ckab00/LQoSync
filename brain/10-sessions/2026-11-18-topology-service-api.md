---
type: session
created: 2026-11-18
tags: [topology, api, socket]
status: done
related: ["[[topology-manager-status]]", "[[2026-11-18-topology-enrollment-cli]]"]
---

# Service API (Phase 2a)

The owner chose JSON lines over a Unix socket, root only. The TUI now, and a web frontend later, use the same API.

## Results

- Plan: [2026-11-18-topology-service-api](../../docs/superpowers/plans/2026-11-18-topology-service-api.md). Seven tasks on branch `topology-service-api`.
- Socket `<state_dir>/api.sock`, mode 0600, peer uid check, stale socket replaced only under the lock.
- Requests run between cycles on the task that owns the `Runner`. The cycle deadline does not move.
- Operations: `status`, `registry.get`, `policy.commit`. Command: `lqosync3 api status`.
- No live service, router, or LibreQoS file changed.

Related: [[topology-manager-status]].
