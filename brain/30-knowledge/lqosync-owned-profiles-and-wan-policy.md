---
type: knowledge
created: 2026-11-10
tags: [profiles, subscriber-policy, topology, wan]
status: design-direction
related: ["[[2026-11-09-live-subscriber-collection]]"]
---

# LQoSync owns subscriber profiles

## Owner direction

The owner wants LQoSync to own profile definitions and subscriber assignments.
RouterOS subscriber discovery remains. RouterOS profiles must not supply the new service plans or speed values.
LibreQoS enforces the selected shaping limits. MikroTik retains routing, authentication, and session duties.
The owner wants WAN discovery to propose candidates, followed by operator confirmation.
The owner proposes a WAN selection in each local profile. Node ownership remains a design question.
This direction changes the RouterOS profile-sync behavior in the LQoSync4 planning documents.
No code, production configuration, router object, or service changed during this discussion.

## Current implementation evidence

`src/routeros.rs` reads `/ppp/profile` and profile fields from subscriber inventory.
`src/config.rs` defines RouterOS speed-source precedence.
`src/topology.rs` creates parents from network mode and source groups, including PPP profile groups.
The new local ownership model needs explicit collector, speed resolver, state, topology, and UI changes.

## Recommendations for discussion

Keep subscriber identity, local profile, WAN policy, actual WAN assignment, and shaping parent as distinct records.
A profile can reference a default WAN policy and a default parent policy.
The effective assignment resolves those policies for the subscriber and the actual topology.
A shared speed plan can serve several sites or WANs without duplicate rate definitions.
Local node CRUD owns approved topology. Discovery can propose nodes but must not replace approved relationships automatically.
A shaping parent does not choose a packet route. WAN changes need a separate MikroTik routing adapter and readback.
If WAN nodes represent real bottlenecks, their shaping membership must follow the effective routing assignment.
The deployment topology must confirm that each WAN fits a tree before this hierarchy becomes a contract.

WAN candidates can use routes, resolved gateways, interface lists, interface types, DHCP/PPPoE clients, and NAT evidence.
These are signals, not proof that an interface is an Internet WAN.
Keep unconfirmed, confirmed, stale, and failed states distinct. An Ethernet link speed is not an ISP service capacity.
Persist operator confirmation and stable upstream identity. Confirm configured capacity separately.

Use a local subscriber ID with router/source identity and reconciliation history. An IP address alone cannot identify a subscriber.
An unmatched subscriber needs an explicit unassigned policy. Do not silently recover its plan from RouterOS.
Retain existing behavior during a staged migration. Preview local assignments before activation and preserve a rollback path.
Existing RouterOS/RADIUS queue limits need a separate review before LibreQoS becomes the sole intended rate enforcer.
Do not remove PPP profiles needed for authentication, addressing, or session setup.

## Reference scope

Reviewed `origin/lqosync4` at `b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8`.
Product, feature map, profile/assignment phases, route contract, workflows, and visual system informed the discussion.
The `docs/lqosync4` tree contains visual specifications and CSS, but no PNG/JPEG/WebP/SVG screenshot files.
The proposed relationships above are recommendations. They are not implemented features or an approved rollout plan.
