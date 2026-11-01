---
type: report
created: 2026-11-01
tags: [libreqos, upstream, v3]
---

# LibreQoS upstream review, 2026-11-01

This report compares the LibreQoS GitHub repository with lqosync3 `3.0.122`. It gives the upstream changes since LibreQoS v2.2 and their effect on lqosync3.

## Summary

- The newest GitHub release of LibreQoS is still `v2.2` (2026-08-18). The shaper has `2.2.202608181454`, which is that release.
- The branch `main` is now LibreQoS `3.0.7-beta17` (commits `3d30a7b` and `2d922cb`, 2026-10-27 to 2026-10-31). The package version is `3.0.7.202610310038`. It is a beta, with no GitHub release.
- No upstream change breaks lqosync3. lqosync3 needs no code change now.
- Recommendation: keep LibreQoS 2.2 on the shaper. A move to v3 changes the host network, so it needs its own plan.

## Scope

- Source: `https://github.com/LibreQoE/LibreQoS`, branch `main`, the 46 commits from 2026-08-18 to 2026-10-31.
- Read in detail: the files that lqosync3 depends on, which are `src/LibreQoS.py`, `src/ShapedDevices.example.csv`, `src/lqos.example`, and `src/rust/lqos_config/src/etc/v15/topology.rs`.
- Not read: the complete 68-file diff of the 3.0.7 projection, and the UISP and Powercode integration code.
- No upstream code ran. No package, service, or setting changed on the shaper.

## Upstream changes

| Change | Commits | Effect on lqosync3 |
|---|---|---|
| LibreQoS 3.0.7-beta17 on `main`: new dataplane package (`libreqos-dataplane`, `bridge_rs`), hugepages, mlx5 support, new service names | `3d30a7b`, `2d922cb` | None now. lqosync3 already has the v3 targets. |
| Atomic writes of the runtime state, and a parser that tolerates a torn `network.json` | `b14a687` | None. lqosync3 already writes each file to a temporary file, then renames it. |
| Node minimum bandwidth no longer accumulates across sibling nodes | `ea73749` | Positive. LibreQoS computes the node minimum from the `network.json` of lqosync3 more correctly. |
| Cumulative byte counters for each circuit, through a new bus request | `4339148` to `18cde06` | None. lqosync3 does not use the bus. A later TUI feature could use it, but the project rule forbids bus requests now. |
| UISP and Powercode integration fixes | `e6162ac`, `8a108d0`, `08aa228`, and others | None. lqosync3 replaces those integrations. |
| Upstream CI gates jobs on changed paths | `4577b9b` | None. |

## Compatibility of LibreQoS 3.0.7 with lqosync3

| Item | LibreQoS 3.0.7 | lqosync3 |
|---|---|---|
| `ShapedDevices.csv` | 15 columns, with `Anchor Node ID` | `v3_pair` writes 15 columns. |
| `LibreQoS.py --updateonly` | Prints `refreshShapersUpdateOnly: skipping the legacy file chain.` and does nothing | On v3, lqosync3 runs no reload command. The first full run applies only to v2. |
| `[topology] source` | Default `"shaped_devices"`. Also `"libreqos_network"` and the integrations | `shaped_devices` gives `v3_pair`. `libreqos_network` gives `v3_network`. An integration gives no target. |
| `[topology] compile_mode` | Default `"ap_site"`. Values: `flat`, `ap_only`, `ap_site`, `full` (`full2` is an alias of `full`) | Only `full` shapes the circuits under the node tree. `lqosync3 check` gives a warning for another value. |
| File writes | Atomic, and a torn `network.json` is tolerated | Atomic. |

The offline v3 checks of lqosync3 used the 3.0.7 compiler (refer to [LIBREQOS-V3-INGRESS.md](../LIBREQOS-V3-INGRESS.md)). No test ran on an operating v3 shaper.

## Risks of a move to LibreQoS v3

The risks are in the host, not in lqosync3:

- The v3 dataplane adopts the network bridge. The bridge migration is not proved on this shaper.
- v3 needs hugepages (3072 pages of 2 MiB in the checker plan).
- The Mellanox card and its firmware must be supported by the dataplane package.
- The packages install new service names and replace the old ones.
- Run `libreqos-v3-preflight` as root only with a backup of `/etc/lqos.conf`. Its config loader can write migration files.

## Actions

- Now: none. Keep LibreQoS 2.2 and lqosync3 `target = "v2_pair"`.
- At a move to v3, in lqosync3:
  1. Set `libreqos.target = "auto"` or `"v3_pair"`.
  2. Set `[topology] compile_mode = "full"` in `/etc/libreqos/lqos.conf` if you use the node tree.
  3. Follow "LibreQoS upgrade from 2.2 to 3.x" in [OPERATIONS.md](../OPERATIONS.md).
- Later, as an option: show the byte counters of LibreQoS in the TUI. This needs a bus request, so it needs a decision from the owner first.
