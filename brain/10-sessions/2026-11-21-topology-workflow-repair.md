---
type: session
created: 2026-11-21
tags: [topology, frontend, pppoe, dhcp, orchestration]
status: verified-source
related: ["[[2026-11-17-topology-runtime-owner]]", "[[2026-11-17-topology-source-coordinator]]"]
---

# Topology workflow repair

The owner requested Topology, PPPoE, and IPoE/DHCP layout and workflow improvements against the documentation pictures.
The current request authorizes this work and supersedes the earlier Topology ownership restriction.
Preserve the independent Routers work and its pending shared CSS/notes.

The service loop had no caller for the topology synchronization path.
It now dispatches to the controlled compiler when durable runtime ownership selects topology.
Explicit `topology activate` supplies the missing activation path. A preview compiles without live writes.
Only `activate --yes` publishes and selects the topology writer, under the instance lease and admission gates.

The topology cycle reuses collected source observations for web caches.
It preserves native/bridge diagnostics and circuit counts without legacy State.
Failed source batches retain the previous shaping pair and timestamps, with failed/stale evidence.
The native fixture proves that a confirmed reparent reaches the next scheduled export with stable subscriber identity.

The Builder now has nested branches, ancestor search, contextual actions, readable parent/source labels, and narrow-screen disclosure.
PPPoE/IPoE add IP/MAC search, local-plan filters, reset/reload, and selected side details.
Request generations reject late replies. Snapshot refresh preserves drafts and labels retained endpoints as historical.
WAN discovery already read all enabled routers. The UI now displays every result, including empty and failed routers.

Core, web, native binding, scale, Clippy, formatting, optimized Wasm, and three browser suites passed.
See [the evidence](../../docs/frontend/evidence/topology-workflow/README.md) for exact checks and screenshots.
See [the workflow and activation guide](../../docs/frontend/TOPOLOGY_WORKFLOW_REPAIR.md).
The repair remains source work. The live installation has not been deployed or activated by this task.
