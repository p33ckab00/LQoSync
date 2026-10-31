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
