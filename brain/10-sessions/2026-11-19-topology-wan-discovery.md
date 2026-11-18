---
type: session
created: 2026-11-19
tags: [topology, tui, api, wan]
status: done
related: ["[[topology-manager-status]]", "[[2026-11-18-topology-subscriber-plans]]", "[[lqosync-owned-profiles-and-wan-policy]]"]
---

# Topology WAN discovery (Phase 2b-3a)

The owner wants two WAN nodes from the `pppoe-out` interfaces of the MikroTik core, with LQoSync balancing subscribers between them.

## Results

- Plan: [2026-11-19-topology-wan-discovery](../../docs/superpowers/plans/2026-11-19-topology-wan-discovery.md). Five tasks on branch `topology-wan-discovery`.
- Rules and read: `src/topology_manager/wan.rs`. One `/interface` print per router, one 30 s limit, no double-read check.
- API: `wan.discover`. TUI: `w` in the Builder shows the WAN candidates section.
- Legacy: `RouterCfg::reads_subscribers`. The legacy cycle skips a router without a subscriber protocol.
- No registry change, no configuration key, no router write.
- No live service, router, or LibreQoS file changed.

Related: [[topology-manager-status]].
