---
type: session
created: 2026-11-16
tags: [dpdk, unmapped, investigation]
status: awaiting-native-sample
related: ["[[HANDOFF]]", "[[2026-11-10-dpdk-mapped-unmapped]]"]
---

# Native unmapped traffic investigation

Reviewed: 2026-11-16T11:27:23+08:00 (Asia/Shanghai). Agent: Codex `/root`.
The owner requested investigation of the LibreQoS native tracker, rather than the LQoSync mirror.
Branch/worktree: `lqosync3`, `/home/pi/LQoSync3.0`. Prior unrelated changes remain intact.

The mirror reads the native aggregate counters. It does not classify traffic.
Its retained history shows a 27.166588 Mbps unmapped download sample at 10:31:08+08:00.
Later samples show approximately 0.85–0.96 Mbps. These rates cannot identify the responsible IP.

The installed UI requests `UnknownIps` and displays IP, last seen, and cumulative directional bytes.
The public beta19 handler reads `best_effort_hosts` through bridge analytics on DPDK systems.
The legacy bus `AllUnknownIps` request returned 15 addresses, all absent from the current input CSV.
All rates in those three legacy samples were zero. This is not proof of current native DPDK host activity.
Do not present that legacy list as the native UI list or as confirmed unshaped subscribers.

The current CSV contains 28 rows. No exact native missing-client conclusion follows yet.
The native socket remains root-only. Direct access as pi fails with PermissionError. Passwordless sudo is unavailable.
Prepared `/home/pi/.local/state/lqosync3/unmapped-investigation/native-probe.py` with a local CBOR converter.
It reads state twice and three bounded snapshots, including the separate best-effort host class.
It compares best-effort IPs with CSV prefixes, checks producer/state stability, and calculates interval byte changes.
It reports the configured allow/ignore filters separately. The 512-row cap can limit coverage.
The probe sends no mutation request and captures no packets. It changes no service or configuration.

Checks passed: Python syntax, CBOR byte arrays, IPv4/IPv6 decode, and synthetic three-sample comparison.
The native live check remains pending. Raw local legacy evidence stays outside Git in a private directory.

Next command from the operator terminal:

```bash
sudo python3 /home/pi/.local/state/lqosync3/unmapped-investigation/native-probe.py | tee /home/pi/.local/state/lqosync3/unmapped-investigation/native-summary.json
```

Source reference: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqosd/src/node_manager/local_api/unknown_ips.rs
Public source is a protocol reference, not proof of all installed private implementation details.

## Native sample results

The operator supplied `native-summary.json` at 2026-11-16T11:28:13+08:00 (Asia/Shanghai).
Three native samples show 18 best-effort rows. The CSV and complete native state report remained unchanged.
The 512-row sample cap was not reached. Thirteen rows match the approximated UI range and age filters.
Three best-effort IPs also occur in the current subscriber CSV.
The last interval contains approximately 1.050812 Mbps unmapped download and 0.535102 Mbps unmapped upload.
CSV subscriber IPs account for approximately 99.71% of that download and 99.27% of that upload.
Two subscriber IPs carry most of this traffic. The third has a small upload increment.
This establishes subscriber traffic in the native best-effort class during this sample.
It does not establish the cause of the earlier 27.17 Mbps peak.

Read-only file review confirms all three subscriber entries in both `shaping_inputs.json` and the generated transaction.
The generated transaction contains explicit `/32` map operations and enabled circuit leaf operations for all three.
The current input CSV therefore does not omit these subscriber mappings.
The old last-loaded CSV differs, but it is not proof of the current native applied table.
Fresh broker snapshots still show a deferred 48-operation full-world reload.
The post-restart journal still reports `NON_SUFFIX_STRUCTURAL_REMOVE` with a 27-slot table.
This supports an unresolved policy convergence problem. Exact per-IP native table correlation remains pending.
Do not claim that every best-effort IP is a subscriber or that all subscriber traffic is unshaped.

Private identifiers remain in the local native summary outside Git. No configuration or service changed.
