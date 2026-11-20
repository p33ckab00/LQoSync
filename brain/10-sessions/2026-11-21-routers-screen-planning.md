---
type: session
created: 2026-11-21
tags: [frontend, routers, planning, wan]
status: proposed
related: ["[[2026-11-20-frontend-blueprint-research]]"]
---

# Routers screen planning

The owner reserved the Dashboard work for Claude Code and requested router design and implementation plans before application changes.
The inspected HEAD was `c324203`. The shared worktree contained Dashboard/worker changes and a staged removal of `dpdk_history.rs`.
This task preserves those changes.

Read LQoSync4 reference documentation from `origin/lqosync4` at `b5284bd` through `git show`.
No LQoSync4 branch or folder was created.
The old router design supplies inventory, safe details, registration, connection tests, and read jobs.

The owner selected add/edit and disable/enable without router deletion.
Disabled must block all MikroTik I/O, including discovery, retries, tests, and future writer readback.
The disable review must include removal from active shaping.
The router I/O result and LibreQoS publication result stay separate.

The RouterOS writer scope is WAN pinning through address lists on the core router that owns the WAN.
Inputs can be subscriber IPs or future DPI application-IP evidence.
The owner approved official Microsoft Fluent 2 guidance because a local Microsoft frontend skill was unavailable.
Vercel frontend-design, Google modern-web-guidance, and writing-plans also guided the plan.

The core has read sessions and cached collection evidence. It lacks a router-wide administrative flag and narrow web management jobs.
The WAN discovery path can connect independently of subscriber-source flags.
No inspected typed API supplies DPI application-IP evidence or the proposed WAN writer.
A visible future UI must show these unavailable capabilities honestly.

The proposed child screens are Overview, Connection, Sources, Read and write, WAN pinning, and Activity.
Registration has a three-step form. Disable has a separate impact review.
R00–R07 deliver the initial connected slice and future UI surface.
R06 has four separate UI commits: inventory, detail/operations, registration/editor, and disable review.
R08–R09 define later address-evidence and RouterOS writer packages.

See [the design](../../docs/frontend/ROUTERS_DESIGN.md) and [atomic plan](../../docs/frontend/ROUTERS_IMPLEMENTATION_PLAN.md).
The existing Routers entry in SCREENS links to them.
Documentation links, whitespace, and the required decision checklist received review.
The STE checker is heuristic. Technical terms received manual review.
No application code, tests, services, or router settings changed in this planning task.

Next action: review the plans, then choose the first implementation task.
Coordinate shared shell integration with the Dashboard owner before R06.
