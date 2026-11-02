---
type: decision
created: 2026-11-02
tags: [adr, libreqos, v3, reload]
status: accepted
supersedes:
related: ["[[ADR-0010-first-full-libreqos-run]]", "[[libreqos-v3-ingress-facts]]", "[[ADR-0012-no-subscriber-grace]]"]
---

# ADR-0011 — Request the v3 import of the pair and check it

## Context

LibreQoS 3.0.7 with `[topology] source = "shaped_devices"` does not import the pair when the files change.
Its scheduler imports the pair at start, on a reload request, and every 30 minutes (`importAndShapePartialReload`).
Without a request, a subscriber change can wait up to 30 minutes.
The LibreQoS UI reload button writes `<state>/scheduler/reload_request.json`. The scheduler reads it each second.

## Decision

1. In `apply` mode with the `v3_pair` target, after each change of the pair, write the token
   `{"requested_at": <now>, "nonce": <random>, "requested_by": "lqosync3"}` to `<libreqos.state_directory>/scheduler/reload_request.json`.
   Write a temporary file, then rename it. A write error is a warning only.
2. After each change, write `<state_dir>/import-pending` with the time of the change.
3. Each cycle with `import-pending`, compare `<state>/topology/topology_import.json` with the written pair:
   nodes (`compatibility_network_json`), and device ID, circuit ID, parent and maximum rates (`shaped_devices`).
4. The status shows `waiting for the LibreQoS import (<n> s)`, then `imported by LibreQoS`. With no change, it shows `no change`.
5. After 40 minutes without a match, the cycle reports `LibreQoS did not import the new files`.

## Rejected

- Make the two-file write atomic. The `lqosd` watcher rejects a symbolic link, so a folder swap is not possible.
  The state is written first, so the next cycle repairs a CSV that a crash left old. A test proves this.
- A time-only check on `topology_runtime_status.json`. A new `generated_unix` does not prove that the import has our content.

## Consequences

- Live proof on 2026-11-02 (`0978937`, `3.0.126+g0978937`): `Reload requested by lqosync3` came in the same second as the write.
  The import was ready in 3 s. The bridge publish completed 3 s later. The next cycle showed `imported by LibreQoS`.
- The reload request is an internal LibreQoS interface. After a LibreQoS upgrade, the 40-minute error shows a broken request.
- A removal of a circuit that is not at the end of the bridge slot table gives `NON_SUFFIX_STRUCTURAL_REMOVE` and an R3 cold rebuild in LibreQoS. It completed in 3 ms. This is LibreQoS behavior.
