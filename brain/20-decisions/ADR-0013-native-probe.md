---
type: decision
created: 2026-11-02
tags: [adr, libreqos, v3, native, safety]
status: accepted
supersedes:
related: ["[[ADR-0011-v3-import-request-and-check]]", "[[2026-11-02-native-runtime-policy-validation]]"]
---

# ADR-0013 — Probe the LibreQoS native compiler before a `v3_network` write

## Context

LibreQoS 3.0.7 compiles `libreqos_network.json` (schema 2) with 1 Mbps nodes in `full` and `full2` mode.
The report is `docs/reports/2026-11-02-libreqos-native-node-rate-loss.md`.
The version number does not show when LibreQoS corrects the defect. LibreQoS has no API that gives the schema version.
With `target = "auto"`, a change of `[topology] source` to `libreqos_network` selects `v3_network` at once.

## Decision

1. On LibreQoS v3, LQoSync compiles a synthetic schema 2 topology with the installed `liblqos_python.so` (`src/probe.rs`).
   It uses `<state_dir>/native-probe/` and its own `lqos.conf`. The probe passes only if the node rates stay.
2. The result is cached in `<state_dir>/native-probe.json`. The key is the LibreQoS version, `compile_mode`, and the binding size and time.
3. In `apply` mode, `v3_network` writes nothing until the probe is `ready`.
4. `status.json` (`native`) and `lqosync3 check` show the result. Telegram sends one notice when the probe becomes `ready`.
5. LQoSync does not change `/etc/lqos.conf`. The operator changes the source.

## Rejected

- Detect the fix from the version number. A new version can keep the defect.
- An automatic change of `[topology] source`. LibreQoS owns `lqos.conf`, and the rollback of a source change has no live test.
- A probe in each cycle. The result changes only with the LibreQoS package, so the probe runs once for each key.

## Consequences

- On 2026-11-02 the probe gives `blocked: node PROBE-SITE compiled to 1/1 Mbps, expected 200/100` in 0.1 s.
- A `strace` of the probe found writes only below its scratch folder, and no successful `connect`.
- The probe runs the LibreQoS binding as the service user (root). The binding is the same code that the LibreQoS scheduler runs as root.
- The probe checks only `full` and `full2`. In `ap_site`, `ap_only` and `flat`, the synthetic topology has no AP, so the probe gives `blocked`.
