# Procedure: remove the mlx5 compatibility override when a fix is available

Date: 2026-11-03. Host: LibreQoS v3 production shaper (DPDK dataplane, ConnectX-4 Lx).
Data source: `systemctl cat libreqos-bridge`, `/etc/libreqos/bridge.env.d/`, `/usr/libexec/libreqos/pmds/`, `fwupdmgr`.
Background: [libreqos-v3-mlx5-compat](../../brain/30-knowledge/libreqos-v3-mlx5-compat.md).

## 1. Summary

- The DPDK bridge starts on this NIC only with the `txq_mem_algn=0` argument.
- The argument is in an operator file: `/etc/libreqos/bridge.env.d/90-mlx5-compat.conf`.
- This file replaces the managed EAL arguments in `50-ports.conf`. A later LibreQoS change to these arguments does not apply while the file is active.
- When a fix is available, test the bridge without the file in a maintenance window.
- If the test passes, keep the file disabled. If the test fails, enable the file again.

## 2. Current state (2026-11-03)

| Item | Value |
|------|-------|
| NIC | ConnectX-4 Lx, 2 ports, `mlx5_core`, firmware `14.25.1020 (MT_2420110034)` |
| Ports | WAN `0000:01:00.0` (`enp1s0f0np0`), LAN `0000:01:00.1` (`enp1s0f1np1`) |
| mlx5 PMD | `/usr/libexec/libreqos/pmds/librte_net_mlx5.so.26.0` (DPDK 25.11 ABI) |
| LibreQoS | `libreqos` and `libreqos-dataplane` `3.0.7.202611021200` |
| Bind mode | `LIBREQOS_DPDK_BIND_MODE=none`. The ports keep the kernel driver. |
| Firmware update | `fwupdmgr get-upgrades` shows no update for the ConnectX-4 Lx. |

The error without the override:

```
mlx5_net: Failed to register unique umem for all SQs.
mlx5_net: port 0 Tx queues memory allocation failed: Protocol not supported
```

The override file:

```
LIBREQOS_DPDK_EAL_ARGS=-d /usr/libexec/libreqos/pmds -a 0000:01:00.0,txq_mem_algn=0 -a 0000:01:00.1,txq_mem_algn=0
```

The managed value in `50-ports.conf`:

```
LIBREQOS_DPDK_EAL_ARGS=-d /usr/libexec/libreqos/pmds -a 0000:01:00.0 -a 0000:01:00.1
```

The bridge unit loads `EnvironmentFile=-/etc/libreqos/bridge.env.d/*.conf` in name order.
A later file replaces a value from an earlier file. Thus `90-` replaces `50-`.
A file that does not end in `.conf` is not loaded.

## 3. Reasons for the approach

### 3.1 Why the override uses `txq_mem_algn=0`

The `txq_mem_algn` device argument of the mlx5 PMD sets how the driver allocates memory for the Tx queues.
The [DPDK 25.11 mlx5 guide](https://doc.dpdk.org/guides-25.11/nics/mlx5.html) gives these values:

| Item | Default (`50-ports.conf`, no argument) | `txq_mem_algn=0` (`90-mlx5-compat.conf`) |
|------|----------------------------------------|------------------------------------------|
| Value | log2(4K) = 12. On some distributions, log2(64K). | 0 |
| Allocation | Consolidated. One memory block for all Tx queues. Each queue starts after the previous queue. All queues use one umem and one memory region. | Legacy, per queue. Each Tx queue has its own memory and its own memory region (MR). The guide says: "0 indicates legacy per queue memory allocation and separate Memory Regions (MR)". |
| On this host | One umem for the 6 Tx queues of each port | 6 separate MRs for each port, 12 in total |
| Result on ConnectX-4 Lx, firmware `14.25.1020` | The bridge does not start: `Failed to register unique umem for all SQs` and `Protocol not supported` | The bridge starts and operates since 2026-11-02 |

The consolidated allocation must register one umem for all send queues (SQ) with the NIC.
The NIC firmware rejects this registration.
The probable cause is the age of firmware `14.25` (2019). The DPDK guide gives no firmware requirement, so this cause is not proved.

The legacy allocation was the DPDK method before the consolidated allocation became the default.
The consolidated allocation gives better cache behavior and fewer MRs. This benefit is important only at a high packet rate.
At the current load of 620 Mbps, the performance difference is very small.

### 3.2 Why the fix is a drop-in file

These are the alternatives and the reasons for the selection:

| Alternative | Result | Selected |
|-------------|--------|----------|
| Edit `50-ports.conf` | `lqos_setup ports-plan` writes this file again at each `libreqos` install or upgrade. The edit is lost, and the bridge does not start after the upgrade. The file header says: "Do not hand-edit". | No |
| Edit `bridge.env` | `bridge.env` is a package conffile. At an upgrade, dpkg asks to replace it. On 2026-11-03 the operator selected the maintainer version. An edit in this file would be lost. Also, `50-ports.conf` loads after `bridge.env` and replaces an EAL argument value in `bridge.env`. | No |
| Drop-in file `90-mlx5-compat.conf` | No package owns the file, so an upgrade does not change it. It loads after `50-`, so its value is used. The `50-ports.conf` header says: "Drop-ins sorted after this one still win". LibreQoS supports this method. | Yes |
| Use other hardware (a NIC that the default allocation supports) | This removes the problem, but it costs money and needs a planned hardware change. | No, not necessary at this time |

The drop-in file has one disadvantage: it contains a full copy of the managed EAL arguments.
Thus section 4 tells you to compare the two files after each LibreQoS update.

### 3.3 Why the test procedure has these rules

- **Rename the file, do not delete it.** The rollback is one `mv` command. The file contents stay correct for the rollback.
- **The name `.conf.disabled`.** The unit loads only `*.conf`. Thus a rename disables the file without a change to the unit or to the file contents.
- **Maintenance window.** The production shaper has no second dataplane. If the bridge does not start, all subscribers lose the internet.
- **Management port `enp3s0`.** This port does not use the dataplane. The SSH connection stays available if the bridge fails.
- **One change at a time.** If a package update and the override removal occur in the same restart, you cannot find the cause of a failure.
- **Monitor for 24 hours, and keep the disabled file for one more update.** A fault can occur only under load or only with a later package version. The disabled file gives a fast rollback for these faults.

## 4. Signs that a fix is available

Do this check after each LibreQoS update. Do the test in section 5 if one of these conditions is true:

| Sign | Check command | Current value |
|------|---------------|---------------|
| LibreQoS adds the argument to the managed plan | `grep -n txq_mem_algn /etc/libreqos/bridge.env.d/50-ports.conf` | no match |
| The DPDK version of the mlx5 PMD changes | `ls /usr/libexec/libreqos/pmds/librte_net_mlx5.so.*` | `.so.26`, `.so.26.0` |
| LibreQoS documents mlx5 or ConnectX-4 Lx support | `docs/v3/known-limitations.md` and the release notes | no mention |
| A new NIC firmware is installed | `ethtool -i enp1s0f0np0` (line `firmware-version`) | `14.25.1020` |

NOTE: A new DPDK version or a new firmware does not prove a fix. Only the test in section 5 proves it.

NOTE: A managed EAL argument can change, for example the PMD path or a BDF. If only this occurs, do not do the test. Copy the new managed value into `90-mlx5-compat.conf`. Then add `,txq_mem_algn=0` to each `-a` argument.

## 5. Test procedure

### 5.1 Before the test

1. Select a maintenance window. When the bridge restarts, the subscribers have no traffic for some seconds.
2. Connect to the host through the management port `enp3s0`. This port does not use the dataplane.
3. Record the current state:

```bash
systemctl status libreqos-bridge --no-pager | grep -o 'bridge_rs .*'
journalctl -u libreqos-bridge -n 200 --no-pager | grep CLASS-COVER | tail -1
```

Expected result: the command line contains `txq_mem_algn=0`. The CLASS-COVER line shows `applied_mappings=26` (or the current circuit count) and `tripwire=ok`.

CAUTION: DO THE TEST ONLY IN A MAINTENANCE WINDOW. IF THE BRIDGE DOES NOT START, ALL SUBSCRIBERS LOSE THE INTERNET UNTIL YOU DO THE ROLLBACK.

### 5.2 Disable the override

1. Rename the file. The bridge unit then does not load it:

```bash
sudo mv /etc/libreqos/bridge.env.d/90-mlx5-compat.conf /etc/libreqos/bridge.env.d/90-mlx5-compat.conf.disabled
```

2. Restart the bridge:

```bash
sudo systemctl restart libreqos-bridge
```

3. Wait 30 seconds.

### 5.3 Pass criteria

Do all of these checks. The test passes only if each result is correct.

| Check | Command | Correct result |
|-------|---------|----------------|
| Service | `systemctl is-active libreqos-bridge` | `active` |
| No restart loop | `systemctl show -p NRestarts --value libreqos-bridge`. Do it two times, 30 seconds apart. | the value does not increase |
| No mlx5 error | `journalctl -u libreqos-bridge --since "-2 min" --no-pager \| grep -E "umem\|allocation failed"` | no output |
| Default allocation | `systemctl status libreqos-bridge --no-pager \| grep -c txq_mem_algn` | `0` |
| Circuits applied | `journalctl -u libreqos-bridge --since "-1 min" --no-pager \| grep CLASS-COVER \| tail -1` | `applied_mappings` equal to the record, `tripwire=ok` |
| No buffer loss | `journalctl -u libreqos-bridge --since "-1 min" --no-pager \| grep WATCHDOG-BASELINE \| tail -1` | `nombuf[down=0,up=0]` |
| Client traffic | A test subscriber opens a web page and runs a speed test | the page opens, the speed agrees with the plan |
| LQoSync | `journalctl -u lqosync3 -n 4 --no-pager` | `removed 0`, no error |

### 5.4 Rollback

If one check fails, do these steps immediately:

1. Enable the override again:

```bash
sudo mv /etc/libreqos/bridge.env.d/90-mlx5-compat.conf.disabled /etc/libreqos/bridge.env.d/90-mlx5-compat.conf
```

2. Restart the bridge:

```bash
sudo systemctl restart libreqos-bridge
```

3. Do the checks in section 5.1 step 3 again.

Expected result: the command line contains `txq_mem_algn=0`, and the circuit count agrees with the record.

4. Record the failure and the LibreQoS version in the brain session note.

NOTE: The unit has `Restart=on-failure` and `RestartSec=5`. If the bridge fails, systemd starts it again every 5 seconds. The rollback stops this loop.

## 6. After a pass

1. Monitor the bridge for 24 hours. Use the checks in section 5.3.
2. Keep `90-mlx5-compat.conf.disabled` for one more LibreQoS update. It is the rollback file.
3. After the next LibreQoS update passes the checks, delete the disabled file.
4. Update these notes:
   - `brain/30-knowledge/libreqos-v3-mlx5-compat.md`: write the fixed version and set `status: superseded`.
   - `brain/HANDOFF.md`: add a new entry.
   - A new session note in `brain/10-sessions/`.

## 7. Do not do these actions

- Do not edit `50-ports.conf`. LibreQoS writes it. The upstream document `docs/v3/known-limitations.md` says: "do not hand-edit `50-ports.conf`".
- Do not enable vfio or `uio_pci_generic` for these ports. The mlx5 PMD uses the kernel driver (`BIND_MODE=none`).
- Do not delete `90-mlx5-compat.conf` before the test passes. Rename it only.
- Do not do the test and a LibreQoS package update in the same restart. Do one change at a time.
