---
type: session
created: 2026-11-19
tags: [topology, tui, api, rebalance, pins]
status: done
related: ["[[topology-manager-status]]", "[[2026-11-19-topology-builder-edits]]"]
---

# Topology explicit rebalance and pins (Phase 2b-3c)

New subscribers between edits can make a group drift to one WAN. The owner wants an explicit rebalance and subscriber pins by WAN.

## Results

- Plan: [2026-11-19-topology-rebalance-and-pins](../../docs/superpowers/plans/2026-11-19-topology-rebalance-and-pins.md). Four tasks on branch `topology-rebalance-pins`.
- Edit kinds `rebalance` and `subscriber.pin` in `src/topology_manager/edits.rs`. Online check and offline clearing in `Runner::api_edit_propose`.
- TUI: `g` and `G` in the Builder, `P` and the WAN picker in Assignments, offline pinned rows.
- The owner plans application pinning with DPI later. It is a separate feature.
- No registry schema change. No live service, router, or LibreQoS file changed.

Related: [[topology-manager-status]].
