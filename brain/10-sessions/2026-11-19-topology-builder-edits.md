---
type: session
created: 2026-11-19
tags: [topology, tui, api, builder, rebalance]
status: done
related: ["[[topology-manager-status]]", "[[2026-11-19-topology-wan-discovery]]"]
---

# Topology Builder edits (Phase 2b-3b)

The owner wants two WAN nodes with one instance of each subscriber group below each WAN. LQoSync splits each group in the capacity ratio of the WANs.

## Results

- Plan: [2026-11-19-topology-builder-edits](../../docs/superpowers/plans/2026-11-19-topology-builder-edits.md). Six tasks on branch `topology-builder-edits`.
- Rules: `src/topology_manager/edits.rs`. Share objective: `src/topology_manager/rebalance.rs`.
- API: `topology.edit.propose`, `topology.edit.confirm`, `topology.edit.cancel`, `topology.tree`.
- TUI: Builder Tree from the catalog, Inventory selection, node form, parent picker, and the proposal popup.
- No registry schema change. No live service, router, or LibreQoS file changed.

Related: [[topology-manager-status]].
