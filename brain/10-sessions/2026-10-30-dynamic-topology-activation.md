---
type: session
created: 2026-10-30
tags: [lqosync-v3, activation, topology]
status: complete
related: ["[[2026-10-30-dynamic-topology]]", "[[20-decisions/adr-0002-independent-topology-routing]]"]
---

# Live dynamic topology activation

Updated: 2026-10-30T12:49:30+08:00 (Asia/Shanghai).
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Activated code: `9070d1b`; source checkout at activation: `4ee04b0`.
The user authorized the new build, live console restart, and actual topology configuration.
The user selected two WAN roots with profile/server branches and removal of fixed PPP/DHCP/Hotspot layers.

## Changes

Verified the isolated build with `cargo build --locked --bin lqosync-v3`.
The isolated and main `v3/` trees matched. The installed executable digest matched the rebuilt binary.
The live console PID changed from `3379628` to `3564997`. Its loopback bind, data directory, log destination, and launch environment remain unchanged.
A private executable copy, launch metadata, and full application data backup were retained before conversion.
Fresh router reads preceded the guarded dynamic conversion. All changes used existing guarded POST handlers.

The actual configuration now has two roots, 20 direct profile/server branches, ten source policies, and 20 eligible node/WAN pairs.
The selected sources comprise six PPP profiles, three DHCP servers, and one Hotspot profile.
Each source policy has one branch under each WAN and keeps automatic `online_clients` balancing.
The ten original source node IDs and deterministic selection IDs remain stable. Ten peer branches supply the other WAN destinations.
Six fixed service layers were removed. Branches use generic group nodes and retain computed capacity factors.
Root capacities, physical WAN mappings, core bindings, router settings, speed rules, and pins remain preserved.
Explicit replacement history maps former shared destinations into their per-source peer branches without changing their WAN.

## Checks

Fresh build and executable digest checks passed. Health and eight console routes returned HTTP 200.
Fresh reads returned 14 distribution tables and 12 core tables, with zero errors.
The candidate contains 26 active clients and 26 complete routing records, with a 13/13 WAN split.
All 26 continuous client identities retained their WAN assignments across the hierarchy edits.
Every assigned branch is a direct child of its recorded WAN root. Stored assignment parents agree with routing records.
Live inventory health, subscriber speeds, assignments, minimums, and shaping capacity checks passed.
Read-only browser checks passed on six configuration routes at 1440px, 390px, and 320px.
Native dynamic controls, root selection, dialog layer order, keyboard focus, and Escape passed without saving test edits.
Credential metadata and other monitored service process IDs remain unchanged.
One on-demand core read was superseded by a scheduled read. Its stale result was rejected; a fresh retry passed.
No application code changed in this task. The earlier full suite is historical; it was not repeated for this operational activation.

## Remaining state

Core membership remains Conflict, as before activation. No committed generation exists.
The candidate does not prove applied routing or shaping. Publication gates and protected manual RouterOS entries remain intact.
No router configuration, LibreQoS output, LibreQoS service, or package changed.
Private configuration, raw inventory, credentials, customer identities, and rollback contents remain out of Git and these notes.
Private rollback records are stored under the application activation directory in `/home/pi/.local/state/lqosync3/`.
Next: review the existing core membership conflict before any production publication or router migration.
See [[2026-10-30-dynamic-topology]] and [[20-decisions/adr-0002-independent-topology-routing]].
