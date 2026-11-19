---
type: session
created: 2026-11-19
tags: [topology, tui, api, speed, factors]
status: done
related: ["[[topology-manager-status]]", "[[2026-11-19-topology-rebalance-and-pins]]"]
---

# Topology speed policy and group factors (Phase 2b-4)

The owner wants one global minimum for every subscriber and an editable factor for each subscriber group.

## Results

- Plan: [2026-11-19-topology-speed-policy](../../docs/superpowers/plans/2026-11-19-topology-speed-policy.md). Four tasks on branch `topology-speed-policy`.
- Edit kinds `speed.defaults` and `group.factor` in `src/topology_manager/edits.rs`. Check-only proposals in `Runner::propose_check_only`.
- TUI: Policies speed rules and groups, `s` speed form, `f` factor form, popup rate samples, capacities, and warnings.
- No registry schema change. No live service, router, or LibreQoS file changed.

Related: [[topology-manager-status]].
