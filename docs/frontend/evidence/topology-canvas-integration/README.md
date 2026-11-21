# Topology canvas integration: baseline and acceptance manifest

Status: Task 0 baseline. Read-only evidence. No service, router, or shaping file changed.
Recorded: 2026-11-22T02:00+08:00 (Asia/Manila) by Claude Code.
Plan: [canvas integration plan](../../../superpowers/plans/2026-11-22-topology-canvas-integration.md).

## Source and worktree

| Item | Value |
|---|---|
| Branch | `lqosync3` in `/home/pi/LQoSync3.0` |
| Implementation base | `2aa4fb0` (feat(traffic): compact the Traffic card header) |
| Uncommitted work at base | Traffic/overview work of another agent (`web-contract`, `web-gateway`, `web-ui` dashboard and traffic files, `src/status.rs`). It is not part of this plan. |

## Installed state (read-only)

| Item | Value | Source |
|---|---|---|
| Service | `lqosync3.service`, active, `run --config /etc/lqosync3/lqosync3.toml` | `systemctl status` |
| Deployed binary | `/usr/local/bin/lqosync3`, `3.0.388+g70f5594.dirty`, SHA-256 below | `--version`, `sha256sum` |
| Web services | `lqosync3-web.service`, `lqosync3-web-broker.service`, both active | `systemctl list-units` |
| Runtime mode | `apply` | `status.json` |
| Target | `v3_pair` (LibreQoS v3, `/opt/libreqos/src`) | `status.json` |
| Config digest | `c3abec6e67014ccb` | `status.json` |
| Native status | `blocked: node PROBE-SITE compiled to 1/1 Mbps, expected 200/100` | `status.json` |
| Sources | `FSEHub-Distro` (PPPoE per plan, DHCP with 4 servers); `FSE-CORE` (WAN only, PPPoE/DHCP off) | `lqosync3.toml` |
| `network.json` | 1.1 KiB, SHA-256 `12efa9b3…9679c2cd` | `/opt/libreqos/src` |
| `ShapedDevices.csv` | 4.0 KiB, SHA-256 `6ae32b83…eede9848` | `/opt/libreqos/src` |

Full binary SHA-256: `45b2f94876c230ecd41e6f81b821685e8028e119fa75044acf3715a6357e0bfe`.

## Unverified items

The current writer is not verified.
`/var/lib/lqosync3/topology-manager/` (it holds `runtime-owner.json`) and `topology.json` have root-only permissions.
`sudo` needs a password in this session. Do not record the writer as topology or legacy until the owner reads that directory.

## Other writers

| Writer | State |
|---|---|
| `lqosync-updatecsv.service` (old Python sync) | disabled, inactive |
| `libreqos-scheduler.service` | active; UISP, Splynx, Netzur, VISP, Powercode, and Sonar integrations are disabled in `/etc/lqos.conf` |

Before a transfer, check again that no other process writes the shaping pair.

## Protected data

- Static records in the live pair that no router source produces. The Task 6 shadow comparison must list each one by stable identity.
- Router coordinates in `lqosync3.toml` (`12.236199459585354, 122.02419129509887`). These are the legacy fallback for Site locations.
- `/var/lib/lqosync3/applied/` (ten previous pairs) and `/var/lib/lqosync3/config-backups/`.

## Supported configurations

- Target `v3_pair` only. Other targets stay on their current path.
- Sources: MikroTik PPPoE (per plan) and DHCP (named servers, bound leases only).
- One LQoSync service instance with the instance lease.

## Backups before a live window

- Copy of `/usr/local/bin/lqosync3` and the web binaries.
- `/etc/lqosync3/lqosync3.toml`.
- `/var/lib/lqosync3/topology-manager/` (registry and owner marker), by the owner as root.
- The complete matching pair from `/opt/libreqos/src` and its write receipt.

## Semantic comparison fields

Circuit ID, device ID, MAC, IPv4 and IPv6 endpoints, resolved parent node, download/upload minimum and maximum rates, node type, node coordinates, node capacities, circuit count, device count, and protected static records.

## Required live events (Task 7)

1. First topology publication under the exclusive lease, with matching import generation and native ACK.
2. One canvas policy edit that applies without a preview or approval step.
3. One layout-only move. It must not change the pair or request a reload.
4. Three no-op cycles with no file change.
5. One service restart. Recovery must complete before the next publication.
6. One source outage simulation (one router unreachable). Saved intent must stay; observations must show stale.
7. One controlled rollback rehearsal by the documented procedure.

## Observation window

At least 24 hours after the first topology publication, with all events above.
A shorter idle interval does not prove reliability.
