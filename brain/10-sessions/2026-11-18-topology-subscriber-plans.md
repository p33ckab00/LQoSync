---
type: session
created: 2026-11-18
tags: [topology, tui, api, plans]
status: done
related: ["[[topology-manager-status]]", "[[2026-11-18-topology-tui-readonly]]"]
---

# Topology subscriber plans (Phase 2b-2)

IPoE clients have no speed control of their own. The owner wants LQoSync plans for DHCP/IPoE clients and a plan override for PPP subscribers.

## Results

- Plan: [2026-11-18-topology-subscriber-plans](../../docs/superpowers/plans/2026-11-18-topology-subscriber-plans.md). Seven tasks on branch `topology-subscriber-plans`.
- Rules: `src/topology_manager/plans.rs`. A plan is a `plan-<n>` profile with a `Local` base rate.
- API: `plan.create`, `plan.update`, `plan.delete`, `subscriber.plan`. `topology.policies` lists plans with their use count.
- TUI: `Enter` in Assignments opens the plan picker. `n`, `e`, and `d` in Policies edit plans.
- Storage stays the JSON registry. No schema change.
- No live service, router, or LibreQoS file changed.

Related: [[topology-manager-status]].
