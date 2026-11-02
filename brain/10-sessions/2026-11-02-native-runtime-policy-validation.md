---
type: session
created: 2026-11-02
tags: [libreqos, topology, validation, defect]
status: complete
related: ["[[2026-11-02-native-topology-source-check]]", "[[2026-11-02-libreqos-v3-live-cutover]]"]
---

# Native runtime policy validation

Verified at 2026-11-02T14:29:00+08:00 (Asia/Shanghai).
The user asked how to validate whether the runtime repairs the native node-rate loss.
Result: FAIL. The isolated installed daemon emits incorrect native policy through the final transaction stage.

## Method

Used the installed `/usr/libexec/libreqos/lqosd` for two cold starts with empty state directories.
Both runs used the same source data and installed example configuration.
Only the source selector and scratch paths differed between the pair and native runs.
The example config does not equal the complete production config.

Reused the existing local `python:3.12-alpine` container image. No image was downloaded.
Mounted host executables and libraries read-only to provide their matching glibc dependencies.
Only the private test directory was writable on the host.
The containers had no external network, host NICs, host PID namespace, or host IPC.
The successful runs used container root with all capabilities dropped except `DAC_OVERRIDE` for scratch-file access.
They had no network-administration or system-administration capability.
Limits: 384 MiB RAM, no extra swap allowance, half a CPU, and 128 processes.
Runtime directories and shared memory were private to each container.

Earlier setup attempts failed on private-directory access, lock/bus permissions, or missing glibc helper executables.
Those failures did not validate runtime policy. Only `runtime-8ac8796e` supplied the successful comparison.
Both daemon processes remained active while the final artifacts were collected.
All task containers were stopped and removed afterward.

## Evidence

Artifact directory: `/home/pi/.local/state/libreqos-native-check-s12invb1/runtime-8ac8796e`.
Report: `policy-comparison.json`. Logs and generated files remain under `pair/` and `native/`.
The private parent directory has mode `0700` and contains subscriber data. Do not publish it.

The native import reports `writer = "watcher"`.
Its `source_sha256` equals the candidate file hash, which proves acceptance of the exact input.
Both runs produced the same five group IDs and the same 25 runtime circuit IDs.
The fixture contains 26 input circuits. The example configuration emitted 25 leaves in each run.
The reason for this shared exclusion was not investigated. No claim covers the excluded leaf.

| Policy | Pair source | Native source |
|--------|-------------|---------------|
| Five group ceilings, Mbps | 35, 100, 158, 193, 200 | 1 for every group |
| Node policy in operator view | Expected limits | 1 Mbps in each direction |
| Final `inner_ops` ceilings | Expected limits in kbps | `ceil_kbps = 1000` in each direction |
| Final matched leaf plans | Original per-circuit plans | All 25 change in both directions, with 1 Mbps ceilings |

An example leaf changes from 50 Mbps ceiling / 5 Mbps commit to 1 Mbps ceiling / 1 Mbps commit.
Both runtime topology status files report `ready = true` despite this difference.
Readiness and acceptance therefore do not prove rate-policy equivalence.

## Source trace

The native import initially preserves node types and intrinsic rates.
`compile_full_like` then reconstructs canonical nodes with `from_editor_and_network`.
The native compatibility tree is empty at that point, while the editor projection lacks the original rate metadata.
The reconstruction loses policy. Compatibility export supplies its 1 Mbps fallback.
The runtime consumes the compiled canonical state and emits the same limits into the dataplane transaction.
The observed runtime result does not repair the offline discrepancy.

Sources are saved below `source-review/` in the parent artifact directory.
Reference: [full compiler](https://github.com/LibreQoE/LibreQoS/blob/2d922cb6d6ceb14b554d12b88d3fcbb902af9919/src/rust/lqos_topology_compile/src/compile_mode.rs)
and [runtime topology builder](https://github.com/LibreQoE/LibreQoS/blob/2d922cb6d6ceb14b554d12b88d3fcbb902af9919/src/rust/lqos_topology/src/runtime.rs).

## Production and remaining proof

Production remains on `shaped_devices` / `full`. LQoSync and the DPDK bridge remain active.
The live pair and canonical config hashes remained unchanged through all tests.
The final production seam is healthy, with matching applied state and no rejection.
No production service was restarted. No live source switch occurred.

No physical dataplane or throughput test ran in the containers.
The result proves incorrect emitted policy, not measured client throughput at 1 Mbps.
Keep the pair source until a compiler correction passes this comparison.
Then test watcher rejection recovery, policy updates, source rollback, and physical shaping before migration.
No compiler fix was made in this validation task.

Temporary executable copies were removed to recover approximately 216 MiB. Logs and JSON evidence remain.
The branch is `lqosync3`, at user-owned commit `0978937`. No product source changed in this task.
The project graph was not rebuilt for these notes.
