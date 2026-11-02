# Hardware audit and CPU/RAM upgrade options for the LibreQoS host

Date: 2026-11-02. Host: LibreQoS v3 production shaper (DPDK dataplane).
Data source: `lscpu`, `free`, `/proc/meminfo`, `udevadm` DMI data, sysfs PCIe and NIC data, `mpstat`.
The agent had no root access. `dmidecode` was not available.

## 1. Summary

- The CPU capacity is enough for the current load. Five of six cores are more than 90% idle at 620 Mbps.
- The memory is in **single-channel** mode. One of two DIMM slots is empty.
- The system reserves 6 GB of hugepages. DPDK uses approximately 228 MB of this memory.
- Swap use is 2.6 GB. Development tools on the same host cause most of this pressure.
- Recommended first upgrade: add one 16 GB DDR4-2666 DIMM. This gives 32 GB in dual-channel mode.
- Recommended CPU upgrade, only if load increases: Intel Core i7-9700 (8 cores, 65 W).

## 2. Current hardware

| Item | Value |
|------|-------|
| System | HP ProDesk 400 G5 SFF |
| Motherboard | HP 83F2, Intel B360 chipset, socket LGA1151 (Coffee Lake) |
| BIOS | Q08 Ver. 02.30.00, 12/30/2024 |
| CPU | Intel Core i5-9500, 6 cores / 6 threads, 3.0 GHz base, 4.4 GHz turbo, 9 MB L3, 65 W |
| RAM | 16 GB (1 x 16 GB DDR4-2666, dual rank, part `WHALEKOM WKVR426P16`) |
| DIMM slots | 2. `DIMM3` (Channel A) has the 16 GB module. `DIMM1` (Channel B) is empty. |
| Maximum RAM (firmware) | 32 GB |
| Power supply | 180 W (stock for this model) |
| Dataplane NIC | Mellanox ConnectX-4 Lx, dual port, 10 Gb/s per port, `mlx5_core`, firmware 14.25.1020 |
| Dataplane NIC slot | PCIe 3.0 x8 link (8.0 GT/s x8). The link runs at full width. |
| Management NIC | Realtek RTL8111, 1 Gb/s, PCIe x1, `enp3s0` |
| Ports | `to_internet = enp1s0f0np0`, `to_network = enp1s0f1np1` |

## 3. Current load

| Item | Value |
|------|-------|
| Circuits | 26 (`ShapedDevices.csv`). Most plans are 50/50 Mbps. |
| Configured uplink | 620 Mbps down, 620 Mbps up |
| CPU | One core at 100% (`bridge_rs`). Other cores 90% to 98% idle. |
| Load average | 1.12 / 1.37 / 1.56 |
| RAM total / available | 15.5 GiB / 4.2 GiB |
| Hugepages | 3072 x 2 MB = 6 GB reserved. 2958 pages free. 114 pages (228 MB) in use. |
| `lqosd` resident memory | 2.2 GB |
| Swap | 2.6 GB of 4.0 GB in use |
| Memory pressure (PSI) | `some avg300=0.19`, `full avg300=0.18` (low) |

**NOTE:** The `bridge_rs` core at 100% is normal. A DPDK poll-mode thread polls the NIC all the time. This value does not show overload.

**NOTE:** Other programs use memory on this host: `rust-analyzer`, `claude`, `codex`, `headroom`, and Docker. These programs are not part of LibreQoS.

## 4. Requirement comparison

The LibreQoS documentation gives these values:

- Small deployment (about 1,000 subscribers, 1 Gbps): 2 or more strong cores, 8 GB RAM.
- Medium deployment (1,000 to 5,000 subscribers, 1 to 10 Gbps): 6 to 16 strong cores, 32 to 64 GB RAM.
- RAM formula: 2 GB + 0.002 GB per subscriber.
- The shaper needs high single-thread performance. The documentation recommends that you disable hyper-threading.

At 26 circuits, the formula gives approximately 2.1 GB. The host is in the small tier. The CPU and the NIC have large headroom.

## 5. RAM upgrade options

| Option | Result | Benefit | Risk |
|--------|--------|---------|------|
| A. Add 1 x 16 GB DDR4-2666 to `DIMM1` (recommended) | 32 GB, dual channel | Two times the memory bandwidth. More free RAM. DPDK guidance recommends one DIMM in each channel. | Low. Use a module with the same specification. |
| B. Replace with a matched 2 x 16 GB kit | 32 GB, dual channel | Same timings on both modules. The old module becomes a spare. | Low. Higher cost than option A. |
| C. 2 x 32 GB | 64 GB | No benefit for this load | The firmware reports 32 GB maximum. Unsupported. Do not use. |

Specification for options A and B:

- DDR4-2666 (PC4-21300), 16 GB, non-ECC, unbuffered DIMM (desktop UDIMM, not SO-DIMM)
- 1.2 V
- Dual rank (2Rx8), to match the installed module

A faster module (for example DDR4-3200) also operates, but the B360 chipset limits it to 2666 MT/s.

### No-cost memory option

The 6 GB hugepage reservation is much larger than the current use (228 MB). A smaller reservation can release approximately 4 GB to the system. LibreQoS v3 manages this value (`vm.nr_hugepages = 3072`).

**CAUTION:** DO NOT CHANGE `vm.nr_hugepages` BEFORE YOU READ THE LIBREQOS V3 DPDK DOCUMENTATION. IF THE RESERVATION IS TOO SMALL, THE DATAPLANE CAN FAIL TO START. A PACKAGE UPGRADE CAN ALSO WRITE THE VALUE AGAIN.

A second no-cost option is to stop the development tools on the production host when they are not necessary.

## 6. CPU upgrade options

The host already operates with a 9th-generation CPU (i5-9500). Thus, the BIOS has the microcode for 9th-generation Coffee Lake CPUs. Third-party specification pages list only 8th-generation CPUs for this model. This audit did not find an HP document that lists the 9th-generation options below for the 400 G5. The constraint is a 65 W TDP because of the SFF cooler and the 180 W power supply.

| CPU | Cores / threads | Turbo | TDP | Assessment |
|-----|-----------------|-------|-----|------------|
| i5-9500 (installed) | 6 / 6 | 4.4 GHz | 65 W | Enough for the current load |
| i5-9600 | 6 / 6 | 4.6 GHz | 65 W | Small increase. Low value. |
| i7-8700 | 6 / 12 | 4.6 GHz | 65 W | Hyper-threading does not help LibreQoS. Same physical cores. Low value. |
| **i7-9700 (recommended)** | **8 / 8** | **4.7 GHz** | **65 W** | Two more physical cores. Higher turbo. No hyper-threading. Best match for LibreQoS. |
| i9-9900 | 8 / 16 | 5.0 GHz | 65 W | Highest single-thread speed. More heat in the SFF case. Possible thermal throttle. Higher cost. |

Do not use these CPU types:

- **K models** (for example i7-9700K): 95 W TDP. Too much for the SFF cooler and the 180 W power supply.
- **F models** (for example i7-9700F): no integrated graphics. The system has no video card. Without a video output, you cannot see the BIOS or the console.
- **10th generation or newer:** a different socket (LGA1200). These CPUs do not fit this motherboard.

## 7. Platform limits

- Maximum on this platform: 8 cores, 32 GB RAM, one PCIe 3.0 x16 slot.
- The ConnectX-4 Lx already has the x16 slot. Thus, a second dataplane NIC is not possible.
- For more than approximately 1,000 subscribers or more than a few Gbps of traffic, replace the platform. The LibreQoS documentation gives the Intel Xeon E-2388G (8 cores) as an example for 10 Gbps aggregate throughput.

## 8. Recommendation

1. Add one 16 GB DDR4-2666 UDIMM to `DIMM1`. This is the best value. It removes the single-channel limit and the swap pressure.
2. Keep the i5-9500 for the current load.
3. If the circuit count or the traffic increases a lot, install an i7-9700 (not F, not K).
4. Optional: after you read the LibreQoS v3 documentation, decrease the hugepage reservation.

**CAUTION:** A HARDWARE CHANGE STOPS THE SHAPER. ALL SUBSCRIBERS LOSE SERVICE DURING THE WORK. DO THE WORK IN A MAINTENANCE WINDOW. NOBODY HAS TESTED A REBOOT WITH LIBREQOS V3 ON THIS HOST.

After a hardware change, do these checks:

1. Run `free -h`. Make sure that the total memory is approximately 31 GiB.
2. Run `lscpu`. Make sure that the CPU model and the core count are correct.
3. Make sure that the LibreQoS services are active.
4. Make sure that traffic flows in both directions.

## 9. Sources

- [LibreQoS system requirements (compute)](https://libreqos.readthedocs.io/en/latest/docs/SystemRequirements/Compute.html)
- [LibreQoS v2.0 requirements](https://libreqos.readthedocs.io/en/latest/docs/v2.0/requirements.html)
- [HP ProDesk 400 G5 SFF specifications (hardware-corner.net)](https://www.hardware-corner.net/desktop-models/HP-ProDesk-400-G5-SFF/)
- [DPDK: performance with NICs on Intel platforms](https://doc.dpdk.org/guides/linux_gsg/nic_perf_intel_platform.html)
- [HP community: 8th and 9th generation 65 W CPU support on a related model](https://h30434.www3.hp.com/t5/Sprout-by-HP/HP-ProDesk-600-G4-SFF/m-p/9146693)
