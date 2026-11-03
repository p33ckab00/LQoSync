---
type: knowledge
created: 2026-11-03
tags: [knowledge, libreqos-v3, dpdk, mlx5, nic]
status: active
related: ["[[2026-11-02-v3-upgrade-preparation]]", "[[2026-11-02-libreqos-v3-live-cutover]]", "[[2026-11-03-libreqos-v3-package-update]]"]
---

# LibreQoS v3 mlx5 compatibility override

## Hardware

- Two ConnectX-4 Lx ports: `enp1s0f0np0` (WAN, `0000:01:00.0`) and `enp1s0f1np1` (LAN, `0000:01:00.1`).
- Driver `mlx5_core`, firmware `14.25.1020 (MT_2420110034)`.

## Problem

With the default DPDK 25.11 mlx5 settings, the bridge does not start:

```
mlx5_net: Failed to register unique umem for all SQs.
mlx5_net: port 0 Tx queues memory allocation failed: Protocol not supported
```

The default allocation uses one shared memory region for all Tx queues.
This NIC and firmware do not support that allocation.

## Fix

`txq_mem_algn=0` selects the legacy allocation: each Tx queue has its own memory region.
Refer to the [DPDK 25.11 mlx5 guide](https://doc.dpdk.org/guides-25.11/nics/mlx5.html).

File `/etc/libreqos/bridge.env.d/90-mlx5-compat.conf`:

```
LIBREQOS_DPDK_EAL_ARGS=-d /usr/libexec/libreqos/pmds -a 0000:01:00.0,txq_mem_algn=0 -a 0000:01:00.1,txq_mem_algn=0
```

The bridge reads `90-` after `50-ports.conf`. Thus the EAL arguments in `90-` replace the managed value.

## Bind mode

The managed ports plan sets `LIBREQOS_DPDK_BIND_MODE=none` for these ports.
The mlx5 PMD uses the kernel driver. The ports do not need vfio or `uio_pci_generic`.
The preflight line `bind mode uio_pci_generic (vfio unavailable — the legacy fallback)` is a generic plan. It does not apply to these ports.

## Risk at a LibreQoS update

The override is a full copy of the EAL arguments: PMD path, BDF order, and device list.
If LibreQoS changes one of these values in `50-ports.conf`, the old value in `90-` stays active.

After each LibreQoS update, do these steps:

1. Compare `LIBREQOS_DPDK_EAL_ARGS` in `/etc/libreqos/bridge.env.d/50-ports.conf` and `90-mlx5-compat.conf`.
2. If the values are different (other than `txq_mem_algn=0`), copy the new managed value into `90-`. Add `,txq_mem_algn=0` to each `-a` argument.
3. Make sure that the bridge command line contains `txq_mem_algn=0`: `systemctl status libreqos-bridge`.

A NIC firmware upgrade to `14.32.1912` is prepared in `docs/reports/2026-11-03-nic-firmware-upgrade.md`. The images are in `/home/pi/nic-firmware/`.

To remove the override when a fix is available, use the procedure in `docs/reports/2026-11-03-mlx5-override-removal.md`.

The update to `3.0.7.202611021200` on 2026-11-03 did not change the managed EAL arguments. The override stayed active.
