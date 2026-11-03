# ConnectX-4 Lx firmware upgrade from 14.25.1020 to 14.32.1912

Date: 2026-11-03. Host: LibreQoS v3 production shaper (DPDK dataplane, ConnectX-4 Lx).
Data source: `ethtool -i`, `lspci`, `fwupdmgr`, `mokutil`, `/sys/kernel/security/lockdown`, NVIDIA release notes, `mstflint` image queries.
The agent had no root access. The agent did not read or write the NIC flash.
Related procedure: [mlx5 override removal](2026-11-03-mlx5-override-removal.md).

## 1. Summary

- The NIC has firmware `14.25.1020` from April 2019. The latest firmware for this card is `14.32.1912` from January 2026.
- The new firmware has 42 bug fixes since `14.25.1020`. Some fixes prevent a firmware hang when the peer port goes down. This is important for an inline shaper.
- NVIDIA recommends `14.32.1908` or later for "reliability improvements and security hardening enhancements".
- The agent downloaded and verified the new image, the old image (for rollback), and the `mstflint` package.
- Secure Boot is on and the kernel lockdown is `integrity`. This can prevent `mstflint` access to the NIC. Section 5 gives the options.
- The firmware upgrade does not prove a fix for the mlx5 `txq_mem_algn` problem. Keep `90-mlx5-compat.conf` active during the upgrade. Do the override test later, in a different window.

## 2. Card identification

| Item | Value |
|------|-------|
| Card | NVIDIA MCX4121A-ACAT, ConnectX-4 Lx EN, 25GbE dual-port SFP28, PCIe 3.0 x8 |
| NVIDIA SKU | 900-9X4B0-0053-0T1 |
| PSID | `MT_2420110034` |
| PCI ID | `15b3:1015`, subsystem `15b3:0003` (NVIDIA retail card, not a Dell or other OEM card) |
| Ports | `0000:01:00.0` (`enp1s0f0np0`, WAN), `0000:01:00.1` (`enp1s0f1np1`, LAN) |
| Flash | One flash for the two ports. Burn the image through `0000:01:00.0` only. |

The new firmware supports this PSID. The NVIDIA 14.32.1912 compatibility table lists:
"MCX4121A-ACAT MT_2420110034 ConnectX-4 Lx EN network interface card; 25GbE dual-port SFP28; PCIe3.0 x8; ROHS R6".

## 3. Firmware comparison

| Item | Current | Target |
|------|---------|--------|
| FW version | `14.25.1020` | `14.32.1912` |
| FW release date | 2019-04-30 | 2026-01-21 (GA February 2026) |
| UEFI ROM | 14.18.19 | 14.25.17 |
| PXE ROM (FlexBoot) | 3.5.701 | 3.6.502 |
| PSID | `MT_2420110034` | `MT_2420110034` |
| Image check (`mstflint -i <file> v`) | "FW image verification succeeded. Image is bootable." | "FW image verification succeeded. Image is bootable." |

Release history between the two versions (from the NVIDIA release notes):

| Version | Date | Change |
|---------|------|--------|
| 14.26.x to 14.31.x | Date not checked | Most of the 42 bug fixes. Refer to section 4.1. |
| 14.32.1010 | Date not checked | Link and firmware-hang fixes for optical modules |
| 14.32.1900 | 2024-09-02 | Contents not checked. Refer to its release notes. |
| 14.32.1908 | September 2025 | "important reliability improvements and security hardening enhancements". NVIDIA recommends this update. |
| 14.32.1912 | February 2026 | Transition to the "Microsoft Option ROM UEFI CA 2023" for Secure Boot. No new bug fixes. |

## 4. Reasons for the upgrade

### 4.1 Bug fixes that apply to this host

The NVIDIA bug fixes history for 14.32.1912 has 42 entries. NVIDIA fixed all of them after `14.25.1020`.
These entries apply to an inline shaper with SFP28 ports:

| Ref | Problem | Fixed in |
|-----|---------|----------|
| 2796324 | Firmware stops with an RXLOS optical transceiver when the remote port is down | 14.32.1010 |
| 2843888 | Rare system hang when a peer port goes down with an optical module | 14.32.1010 |
| 2771990 | Slow link-up time (fast link-up added) | 14.32.1010 |
| 2410395 | SFP28 cable does not link up at 25GbE | 14.29.2002 |
| 2479788 | Tx hang after a duplicate packet rollback | 14.30.1004 |
| 1906153 | Send hang when the device enters FLR | 14.28.1002 |
| 2119135 | Fragmented IP packets are dropped | 14.28.1002 |
| 2391109 | Fatal error and HCA hang with a large packet on a strided receive queue | 14.31.1014 |
| 2135671 | Race in the transport error handler stops CQ completions | 14.28.1002 |
| 2126484 | Device hang in the software reset flow under heavy stress | 14.28.1002 |
| 2321713 | Device stops (dead IRISC) when a firmware semaphore is not released | 14.29.1016 |
| 211746 | Firmware fatal assert from the `init_hca` timer lock | 14.28.1002 |
| 2252559 | Fatal error from PCI transport layer errors during FLR | 14.29.1016 |
| 1911080 | An incorrect flash write corrupts the firmware and the adapter disappears | 14.28.1300 |

The shaper is inline. A NIC hang stops the internet for all subscribers.
Thus the link-down and hang fixes are the most important reasons for the upgrade.
The DPDK bridge starts and stops the ports at each bridge restart. The FLR, reset, and Tx hang fixes possibly apply to this use.

### 4.2 Security and support

- `14.32.1908` contains security hardening. NVIDIA recommends the update.
- `14.25.1020` is more than 6 years old. All current NVIDIA release notes for this card are in the 14.32 branch.
- The LibreQoS v3 mlx5 PMD is from DPDK 25.11 (2025). The `14.25` firmware is 6 years older than this PMD. The agent did not check the firmware list in the DPDK 25.11 test report.

### 4.3 The mlx5 `txq_mem_algn` problem

The bug fixes history has no entry for umem, DevX, or DPDK.
Thus the new firmware possibly does not fix the `Failed to register unique umem for all SQs` error.
After the firmware upgrade, do the test in the [mlx5 override removal](2026-11-03-mlx5-override-removal.md) procedure. Do it in a different maintenance window.

### 4.4 Risks of the upgrade

- The flash and the reset stop the dataplane. All subscribers lose the internet for some minutes.
- `mstflint` burns in failsafe mode by default. If a burn stops, the card usually keeps the old image. A power loss during the burn is still a risk. Do not use the `--nofs` option.
- `14.32.1912` uses the 2023 UEFI CA for the option ROM. Secure Boot is on. If the BIOS does not have the 2023 CA, the NIC UEFI driver does not load at boot. This affects only network boot (PXE/UEFI) from the NIC. The host boots from the internal SSD, so the OS boot does not use the NIC ROM.
- A host reboot is a possible activation method. The boot recovery of this host is not tested (refer to `brain/HANDOFF.md`). Section 6.4 uses `mstfwreset` first, so that the host does not reboot.

## 5. Secure Boot and kernel lockdown

The host state on 2026-11-03:

| Item | Value |
|------|-------|
| `mokutil --sb-state` | `SecureBoot enabled` |
| `/sys/kernel/security/lockdown` | `none [integrity] confidentiality` |
| `mstflint-dkms` | not installed (candidate `4.26.0+1-2ubuntu3`) |
| DKMS MOK key `/var/lib/shim-signed/mok/` | empty |

In `integrity` mode, the kernel can prevent direct PCI access from user space.
`mstflint` uses direct PCI access to the flash. The agent did not test this, because the test needs root.
If `mstflint` fails, the error is possibly `Operation not permitted`, and `dmesg` shows a line that starts with `Lockdown:`.

If the access fails, select one of these options:

| Option | Steps | Result |
|--------|-------|--------|
| A. Signed kernel module | Install `mstflint-dkms`. Enroll the DKMS MOK key with `mokutil --import`. Reboot and accept the key on the physical console. | Secure Boot stays on. `mstflint` uses the `mstflint_access` module. Needs one reboot and a physical console. |
| B. Secure Boot off for the upgrade | Disable Secure Boot in the BIOS. Do the upgrade. Enable Secure Boot again. | No new software. Needs two reboots and a physical console. |

NOTE: Each option needs a reboot. The boot recovery of this host is not tested. Do the reboot only in a maintenance window, with a physical console.

## 6. Procedure

### 6.1 Prepared files

The agent prepared these files in `/home/pi/nic-firmware/`:

| File | SHA-256 |
|------|---------|
| `fw-ConnectX4Lx-rel-14_32_1912-MCX4121A-ACA_Ax-UEFI-14.25.17-FlexBoot-3.6.502.bin` | `a509dc62000f31140b32162aa9cd1e47c5412608af582aab12f4e92b5588594d` |
| `fw-ConnectX4Lx-rel-14_25_1020-MCX4121A-ACA_Ax-UEFI-14.18.19-FlexBoot-3.5.701.bin` (rollback) | `6fff0b33519dba9aeb95a74ebb490b3753a403a0f724b423fa885d9a59902a78` |
| `mstflint_4.26.0+1-2ubuntu3_amd64.deb` (Ubuntu archive) | `916d2cfb304578d2f3f31ee64385b03dc16f248377135ef8d976c7b1e8cee3b0` |

The images come from `https://content.mellanox.com/firmware/`. The MD5 of each zip file agrees with the server `etag`:
`26b21da3cbdc7bf3f30ff184620cd0d3` (14.32.1912) and `148857e3f1c3f35985b58afd9cfeb357` (14.25.1020).

Before you use the files, do this check:

```bash
cd /home/pi/nic-firmware && sha256sum -c SHA256SUMS
```

Expected result: each line ends with `OK`.

### 6.2 Install mstflint and test the access

These steps do not stop the traffic.

1. Install the tool:

```bash
sudo apt install /home/pi/nic-firmware/mstflint_4.26.0+1-2ubuntu3_amd64.deb
```

2. Read the device information:

```bash
sudo mstflint -d 0000:01:00.0 q
```

Expected result: `FW Version: 14.25.1020` and `PSID: MT_2420110034`.

3. If the command fails, do this check:

```bash
sudo dmesg | grep -i lockdown | tail -5
```

If a `Lockdown:` line names the access, do one option from section 5. Then do step 2 again.

### 6.3 Back up the current firmware and configuration

These steps read the flash only. They do not change the NIC.

```bash
sudo mstflint -d 0000:01:00.0 q full | tee /home/pi/nic-firmware/query-before.txt
sudo mstconfig -d 0000:01:00.0 q | tee /home/pi/nic-firmware/mstconfig-before.txt
sudo mstflint -d 0000:01:00.0 ri /home/pi/nic-firmware/backup-14.25.1020-from-device.bin
sha256sum /home/pi/nic-firmware/backup-14.25.1020-from-device.bin | tee -a /home/pi/nic-firmware/SHA256SUMS
```

Expected result: the backup file is approximately 16 MB. The query files contain the GUIDs, the MACs, and the configuration.

NOTE: Keep the backup file and the query files off the host also. If the card fails, these files have the original GUIDs and MACs.

### 6.4 Burn the new firmware

CAUTION: DO THESE STEPS ONLY IN A MAINTENANCE WINDOW. DO NOT STOP THE BURN AND DO NOT REMOVE THE POWER DURING THE BURN. AN INTERRUPTED BURN CAN MAKE THE CARD UNUSABLE.

1. Connect to the host through the management port `enp3s0`.
2. Stop the bridge:

```bash
sudo systemctl stop libreqos-bridge
```

3. Burn the image:

```bash
sudo mstflint -d 0000:01:00.0 -i /home/pi/nic-firmware/fw-ConnectX4Lx-rel-14_32_1912-MCX4121A-ACA_Ax-UEFI-14.25.17-FlexBoot-3.6.502.bin burn
```

Expected result: the tool shows the current version `14.25.1020` and the new version `14.32.1912`. Then it asks for confirmation. Type `y`.

4. Do not use the `-allow_psid_change` option. The PSID of the image and the card is the same.
5. Show the reset levels that the device supports:

```bash
sudo mstfwreset -d 0000:01:00.0 q
```

6. Activate the new firmware:

```bash
sudo mstfwreset -d 0000:01:00.0 -y reset
```

7. If `mstfwreset` fails, reboot the host. Use the physical console. Refer to the NOTE in section 5.

### 6.5 Make sure that the upgrade is correct

1. Read the firmware version of each port:

```bash
ethtool -i enp1s0f0np0 | grep firmware
ethtool -i enp1s0f1np1 | grep firmware
```

Expected result: `firmware-version: 14.32.1912 (MT_2420110034)` on each port.

2. Start the bridge:

```bash
sudo systemctl start libreqos-bridge
```

3. Do the checks in section 5.3 of the [mlx5 override removal](2026-11-03-mlx5-override-removal.md) procedure.
   One check is different: the bridge command line must contain `txq_mem_algn=0`, because the override stays active.
4. Monitor the bridge for 24 hours.

### 6.6 Rollback

If the bridge does not operate correctly with `14.32.1912`, do these steps:

1. Stop the bridge:

```bash
sudo systemctl stop libreqos-bridge
```

2. Burn the official `14.25.1020` image:

```bash
sudo mstflint -d 0000:01:00.0 -i /home/pi/nic-firmware/fw-ConnectX4Lx-rel-14_25_1020-MCX4121A-ACA_Ax-UEFI-14.18.19-FlexBoot-3.5.701.bin burn
```

3. If `mstflint` does not accept the official image, burn the device backup from section 6.3.
4. Do steps 5 and 6 of section 6.4. Then do section 6.5. The expected version is `14.25.1020`.

## 7. After the upgrade

1. Write the result in a brain session note and in `brain/HANDOFF.md`.
2. Update the firmware value in `brain/30-knowledge/libreqos-v3-mlx5-compat.md`.
3. In a different maintenance window, do the [mlx5 override removal](2026-11-03-mlx5-override-removal.md) test.

## 8. Sources

- [ConnectX-4 Lx FW 14.32.1912 — Firmware Compatible Products](https://networking-docs.nvidia.com/connectx4lxfwrn/14321912/firmware-compatible-products)
- [ConnectX-4 Lx FW 14.32.1912 — Changes and New Features](https://networking-docs.nvidia.com/connectx4lxfwrn/14321912/changes-and-new-features)
- [ConnectX-4 Lx FW 14.32.1912 — Bug Fixes History](https://networking-docs.nvidia.com/connectx4lxfwrn/14321912/bug-fixes-history)
- [ConnectX-4 Lx FW 14.32.1908 — Changes and New Features](https://networking-docs.nvidia.com/connectx4lxfwrn/14321908/changes-and-new-features)
- [ConnectX-4 Lx FW 14.32.1900 Release Notes](https://docs.nvidia.com/networking/display/connectx4lxfirmwarev14321900)
- [kernel_lockdown(7)](https://man7.org/linux/man-pages/man7/kernel_lockdown.7.html)
