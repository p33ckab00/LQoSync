---
type: session
created: 2026-11-18
tags: [topology, tui, preview]
status: done
related: ["[[topology-manager-status]]", "[[2026-11-18-topology-service-api]]"]
---

# Topology TUI read-only views (Phase 2b-1)

The owner chose an on-demand preview before activation, so the service reads routers for the preview only when the operator asks.

## Results

- Plan: [2026-11-18-topology-tui-readonly](../../docs/superpowers/plans/2026-11-18-topology-tui-readonly.md). Ten tasks on branch `topology-tui-readonly`.
- Conflicts are data (`PreparationConflicts`). Prepared pairs keep a `PreviewView`.
- API: `topology.preview.refresh`, `topology.preview.get`, `topology.policies`. Async handlers.
- TUI: Topology route (page 4), API link thread, strip, and four read-only views.
- No live service, router, or LibreQoS file changed.

Related: [[topology-manager-status]].
