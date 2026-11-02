---
type: knowledge
created: 2026-10-31
tags: [libreqos-v3, ingress]
status: draft
related: ["[[ADR-0002-v3-target-selection]]", "[[v3-offline-simulation-method]]"]
---

# LibreQoS v3 ingress facts

The full reference is `docs/LIBREQOS-V3-INGRESS.md`. This note keeps only the facts that change the design.

- LibreQoS 3.0.7 keeps `LibreQoS.py`, but `--updateonly` does nothing.
- `lqosd` watches the input files with inotify. No reload command is necessary.
- The watcher rejects a symbolic link. Write a temporary file, then rename it.
- `[topology] source` selects the input. Values for a custom tool: `shaped_devices` and `libreqos_network`.
- `libreqos_network.json` accepts only `schema_version = 2`. It rejects unknown fields.
- The v3 compiler accepts a duplicate IP and a duplicate node name. The tool must reject both.
- The v3 compiler does not keep the device comment.
- A circuit must have a parent node in `libreqos_network.json`.
- The 13-column `ShapedDevices.csv` from `updatecsv.py` passes `libreqos-v3-preflight`.
- `topology_import.json` is internal to LibreQoS. Do not write it.

- The node rules (node `id`, `Parent Node ID`, rate floors, duplicate names) are in [[v3-node-handling]].

## Evidence

Offline simulation on 2026-10-31. Refer to [[v3-offline-simulation-method]] and [[2026-10-31-project-setup-and-v3-simulation]].

## Live pair ownership — 2026-11-02

The live setup uses LQoSync `apply` / `auto`, detected `v3_pair`, and LibreQoS `shaped_devices` / `full`.
LQoSync owns both source files: `/opt/libreqos/src/ShapedDevices.csv` and `/opt/libreqos/src/network.json`.
The CSV defines circuits and devices. The JSON defines their parent nodes and node rates.
LibreQoS owns the imported, compiled, and effective state under `/opt/libreqos/state/`.
Keep `network.json` for this tree configuration. Do not treat it as a disposable LibreQoS output.

The source writer writes JSON before CSV. Each file write uses a temporary file, sync, and rename.
The two files do not form one atomic transaction. Interrupted-update recovery has not had a live fault test.
The v3 path runs no legacy Python reload. The daemon watches the source files.

Live validation found 26 circuits, five site nodes, and no missing parent names or mismatched parent IDs.
The imported compatibility network equals the source JSON. Imported circuit IDs equal the source CSV IDs.
The runtime operator view contains the same 26 circuits and five site nodes, with topology ready.
Refer to [[2026-11-02-libreqos-v3-live-cutover]].

## Native-source migration gate

The native writer and installed compiler accepted the current 26 circuits with zero conversion alerts.
Compiled subscriber fields match the pair path, except comments.
However, the native compiled canonical output loses node kinds and rates and emits 1 Mbps compatibility node limits.
The native import preserves the supplied policy before compilation.
Do not activate `libreqos_network` until a controlled runtime test proves correct node policy or the compiler discrepancy is resolved.
The native watcher input path is `<lqos_directory>/libreqos_network.json`.
Refer to [[2026-11-02-native-topology-source-check]] for artifacts and evidence limits.

The follow-up isolated daemon test also emits 1 Mbps group limits in `dataplane_transaction.json`.
The native watcher accepted the exact candidate hash. The runtime did not repair the missing node policy.
All 25 matched runtime leaf plans also changed to 1 Mbps ceilings.
Both runs used the same example config and emitted 25 of the 26 input circuits.
There was no physical dataplane test. Production remains on the healthy pair path.
Refer to [[2026-11-02-native-runtime-policy-validation]].
