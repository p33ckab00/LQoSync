---
type: session
created: 2026-11-02
tags: [session, libreqos, v3, reload, probe, monitoring]
status: done
related: ["[[ADR-0011-v3-import-request-and-check]]", "[[ADR-0012-no-subscriber-grace]]", "[[ADR-0013-native-probe]]", "[[network-ipv6-status]]", "[[2026-11-02-native-runtime-policy-validation]]"]
---

# 2026-11-02 — v3 import request, native probe, bridge monitor

## Goal

Make the live `v3_pair` path safe and fast after the LibreQoS v3 cutover.
Find if the native `libreqos_network` source (schema 2) is safe. Show the real load of the DPDK bridge.

## Changes

- `0978937`: LQoSync writes the scheduler reload request (`state/scheduler/reload_request.json`) after each change of the pair.
  Each cycle, it compares `state/topology/topology_import.json` with the written pair.
  The status shows `waiting for the LibreQoS import`, then `imported by LibreQoS`, or `no change`. After 40 minutes, the cycle reports an error.
  A test proves that the next cycle repairs a CSV that a crash left old. See [[ADR-0011-v3-import-request-and-check]].
- `68e6005`: `src/probe.rs` compiles a synthetic schema 2 topology with the installed binding. In `apply` mode, `v3_network` writes nothing until the probe is `ready`.
  `status.json` (`native`), `lqosync3 check`, and one Telegram notice show the result. See [[ADR-0013-native-probe]].
- `ff6756f`: defect report `docs/reports/2026-11-02-libreqos-native-node-rate-loss.md`, with `repro.py`, a regression test, `proposed-fix.md`, and an upstream issue draft.
- `727c068`: `src/bridge.rs` reads `budget_defers`, `iter_gt100ms`, `iter_gt1s` and `iter_max_ms` from the `libreqos-bridge` journal each cycle.
  `status.json` (`bridge`) and the Dashboard show them. Telegram sends one alert when a counter goes up.
- Decision: keep `subscriber_grace = 0`. A disconnect is an unplugged device and must give a new circuit. See [[ADR-0012-no-subscriber-grace]].
- Host: the operator changed the time zone from Etc/UTC to Asia/Manila (`timedatectl`).

## Results

- The installed binary was `3.0.128+gff6756f` at the end of the probe work. The bridge monitor build is `727c068`.
- Live proof of the reload request: `Reload requested by lqosync3` came in the same second as the write. The import was ready in about 3 s. The next cycle showed `imported by LibreQoS`.
- The scheduler imports the `shaped_devices` pair only at start, on a reload request, and every 30 minutes. A file write alone does not start an import.
- A removal of a circuit that is not last in the bridge slot table gives `NON_SUFFIX_STRUCTURAL_REMOVE`, an R3 cold rebuild, and the LibreQoS alert `BRIDGE_REBUILD_REQUIRED`. The rebuild completed in 3 to 45 ms. LibreQoS does this by design.
- LibreQoS 3.0.7 defect: `compile_full_like` rebuilds canonical nodes from the editor projection and an empty compatibility tree. Each node exports at 1 Mbps (`unwrap_or(1)`). `full` and `full2` fail. The synthetic `repro.py` reproduces it.
- The live probe result: `blocked: node PROBE-SITE compiled to 1/1 Mbps, expected 200/100`.
- CPU 1 is always 100%: it is `dpdk-worker1`, which polls the NIC. One worker (`LIBREQOS_DPDK_WORKERS=1`). In one hour, `budget_defers` stayed at 1 and `iter_gt100ms` stayed at 0.
- IPv6: the network and the ISP have no IPv6, and the uplinks have private IPv4 (ISP NAT). See [[network-ipv6-status]].

## Open items

- [ ] Operator: `sudo apt-mark hold libreqos libreqos-api libreqos-dataplane`. On 2026-11-02 the packages were not on hold.
- [ ] Operator: install the `727c068` release binary and restart `lqosync3`.
- [ ] Operator: post the upstream issue from the defect report.
- [ ] Check `budget_defers` and `iter_gt100ms` at evening peak hours.
- [ ] Ask the ISP for IPv6 (IPv6CP and DHCPv6-PD /56) and for a public IPv4.
- [ ] The Telegram alerts for the bridge counters and for a `ready` probe have unit tests only.
- [ ] The Rust regression test and the proposed LibreQoS fix were not compiled.
- [ ] The project graph was not rebuilt.

## Links

- `docs/LIBREQOS-V3-INGRESS.md`, `docs/OPERATIONS.md`
- `docs/reports/2026-11-02-libreqos-native-node-rate-loss.md`
