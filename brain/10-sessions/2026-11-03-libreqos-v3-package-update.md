---
type: session
created: 2026-11-03
tags: [session, libreqos-v3, upgrade, dpdk, mlx5]
status: done
related: ["[[2026-11-02-libreqos-v3-live-cutover]]", "[[2026-11-02-v3-upgrade-preparation]]", "[[libreqos-v3-mlx5-compat]]", "[[HANDOFF]]"]
---

# 2026-11-03 — LibreQoS v3 package update 3.0.7.202611021200

## Goal

Update the LibreQoS v3 packages on the live shaper from `3.0.7.202610310038` to `3.0.7.202611021200`.
Verify the services, the dataplane, and LQoSync after the update.

## Changes

The operator ran these commands in their terminal (times are Asia/Manila):

1. `sudo apt upgrade -y` upgraded only `libreqos-preflight` and `docker-compose-plugin`.
   APT kept back `libreqos` and `libreqos-dataplane`.
2. `sudo apt full-upgrade` removed `libreqos-api` and upgraded `libreqos` and `libreqos-dataplane`.
3. At the `bridge.env` conffile prompt, the operator selected `Y` (package maintainer version).

Cause of the "kept back" result: the new `libreqos` has `Breaks: libreqos-api` and `Replaces: libreqos-api`.
`apt upgrade` does not remove packages. The packages were not on hold (`apt-mark showhold` showed only the systemd hotfix packages).

Package changes in this build:

- `libreqos` now contains the node API. `lqosd` serves it at `/api/v3`. The separate `libreqos-api` package is removed (state `rc`).
- Upstream source diff `2d922cb6...34342a28` (173 files) adds the API source (`src/rust/lqos_api/`, `openapi.json`) and `api_key_auth.rs`.
- New upstream document: `docs/v3/known-limitations.md`.
- The topology compiler files (`dataplane_compiler.rs`, `publish.rs`, `shaping_inputs.rs`) have comment changes only.

Preflight result during the install: `PASS`. It reported 2 dataplane ports, kernel 6.8, protocol 6 vs 6 (aligned), and NIC pair `enp1s0f0np0` + `enp1s0f1np1`.

## Results

Checks at 09:26 (Asia/Manila):

| Check | Result |
|-------|--------|
| Package versions | `libreqos` and `libreqos-dataplane` at `3.0.7.202611021200`. `libreqos-api` removed. |
| Services | `libreqos-lqosd`, `libreqos-bridge`, `libreqos-scheduler` active since 09:23:53. `libreqos-power` is a oneshot unit, result `success`. |
| Bridge restart | The bridge restarted at 09:23:54. The `status=15/TERM` log line is the stop of the old instance. It is not a failure. |
| Dataplane | `applied_mappings=26`, `installed_prefixes=26`, `tripwire=ok`, `nombuf[down=0,up=0]`. Traffic passes in both directions. |
| mlx5 override | `90-mlx5-compat.conf` is unchanged. The bridge command line contains `txq_mem_algn=0` on both ports. |
| Ports plan | `50-ports.conf`: `LIBREQOS_DPDK_BIND_MODE=none`, `LIBREQOS_DPDK_WORKERS=1`, 6 RX and 6 TX queues. |
| LQoSync | `lqosync3 3.0.136+ge8d6cbd`, `apply`, `v3_pair`. Cycles after the update: `added 0 updated 0 removed 0`. |
| Native probe | Ran again for the new version at 09:24:13. State `blocked`: `node PROBE-SITE compiled to 1/1 Mbps, expected 200/100`. |

`bridge.env` difference: the new file does not have `LIBREQOS_DPDK_TRANSACTION_FILE=/opt/libreqos/state/shaping/dataplane_transaction.json`.
This change is intentional. The new file says: "do NOT pin LIBREQOS_DPDK_TRANSACTION_FILE here". The bridge gets the transaction from `lqosd`.
The old file is at `/etc/libreqos/bridge.env.dpkg-old`.

`workers=1` is correct. The upstream autosizing rule applies only to a link above 1070 Mbps. This link is 620 Mbps.

The preflight line `bind mode uio_pci_generic (vfio unavailable — the legacy fallback)` does not apply to these ports.
The mlx5 ports use `BIND_MODE=none` and keep the kernel driver. Do not enable vfio for this NIC.

The native node rate defect is still present in this build. Keep `[topology] source = "shaped_devices"`.

## Open items

- [ ] After each LibreQoS update, compare `LIBREQOS_DPDK_EAL_ARGS` in `50-ports.conf` and `90-mlx5-compat.conf`. Refer to [[libreqos-v3-mlx5-compat]].
- [ ] Check the Web UI and `/api/v3` after the API move. This session did not check them.
- [ ] The LibreQoS packages are still not on hold.

## Links

- [[2026-11-02-libreqos-v3-live-cutover]]
- [[2026-11-02-v3-upgrade-preparation]]
- [[libreqos-v3-mlx5-compat]]
- [[2026-11-02-native-runtime-policy-validation]]
