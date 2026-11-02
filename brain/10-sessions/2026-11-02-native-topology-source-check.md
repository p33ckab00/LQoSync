---
type: session
created: 2026-11-02
tags: [libreqos, topology, validation]
status: complete
related: ["[[2026-11-02-libreqos-v3-live-cutover]]", "[[libreqos-v3-ingress-facts]]"]
---

# Native topology source validation

Follow-up: [[2026-11-02-native-runtime-policy-validation]] reproduced the loss in the installed daemon's isolated runtime.
The final native dataplane transaction contains 1 Mbps group limits. Read that note for current evidence and limits.

The user requested a check of `libreqos_network` as the active topology source.
Scope: validate support and production-data conversion. Do not switch the live source.

## Result

LQoSync has a `v3_network` writer for `libreqos_network.json`, schema 2.
LibreQoS 3.0.7 has the matching source selector and file watcher.
The input path resolves to `/opt/libreqos/src/libreqos_network.json` on this host.
The file combines topology nodes, circuits, devices, match prefixes, and rates.
In this mode, LQoSync would own the single source file. LibreQoS would own the derived runtime state.

The current source remains `shaped_devices`, with full compile mode and LQoSync apply/auto.
The native source file does not exist in the live input directory.

## Isolated checks

Artifacts: `/home/pi/.local/state/libreqos-native-check-s12invb1`.
The directory has mode `0700`. It contains subscriber data. Do not publish its contents.

Used the existing `v3_offline` test binary and the unchanged native writer.
It converted a snapshot of the live pair into five nodes and 26 circuits, with zero alerts.
Used the installed `/opt/libreqos/src/liblqos_python.so` to compile both input formats.
Its hash matches the module from the extracted package.
Both compiler calls returned success.

Each run used separate input and state directories from the installed example config.
`strace` blocked every socket connection with `EACCES`.
The trace audit found no writes outside the scratch directory.
The live pair and canonical config hashes remained unchanged throughout the check.
No daemon was started for the test. The production services remained active.

## Comparison

The compiled device records preserve IDs, names, parents, MACs, IPs, and all circuit rates.
Only device comments differ between the two compiled paths.
Both paths retain five node IDs. Circuit match maps also match.
The native path produces 26 circuit anchors. The pair path produces no explicit anchors in this offline test.

However, node policy is not equivalent:

- The native import initially contains all five site types and the supplied node rates.
- The native compiled canonical output loses all five node kinds and rate inputs.
- The native compatibility tree emits 1 Mbps download and upload rates for those nodes.
- The pair compiler retains the expected node types and bandwidth values.

This reproduces the existing open item about native canonical metadata.
It is an observed compiler-output discrepancy, not proof that the live watcher necessarily applies 1 Mbps limits.
The live watcher path has not had a controlled native-source test.

Do not activate the native source until node policy preservation is proven or the discrepancy is resolved.
Passing the parser and preserving subscriber plans alone does not prove topology equivalence.

## Evidence and next step

Candidate: `native/src/libreqos_network.json` below the artifact directory.
Summary: `comparison.json`. Compiler output and syscall traces are under `pair/` and `native/`.

The [upstream watcher](https://github.com/LibreQoE/LibreQoS/blob/2d922cb6d6ceb14b554d12b88d3fcbb902af9919/src/rust/lqosd/src/libreqos_network_ingest.rs)
gates ingestion on the source selector and records accepted or rejected revisions.
The [reader](https://github.com/LibreQoE/LibreQoS/blob/2d922cb6d6ceb14b554d12b88d3fcbb902af9919/src/rust/lqos_topology_compile/src/libreqos_network.rs)
defines schema 2 and resolves the input path through `legacy_runtime_file_path`.

Next: reproduce the node policy discrepancy in an isolated full runtime or inspect the exact compiler transformation.
Then verify node caps, watcher acceptance, rejection recovery, and source rollback before a live migration.
This task did not authorize compiler fixes or a live source switch.

Existing edits in `src/files.rs` and `src/runner.rs` were present at task start and remain untouched.
No product code or live config changed. The project graph was not rebuilt.
