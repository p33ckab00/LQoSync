---
type: decision
created: 2026-11-01
tags: [adr, libreqos, reload]
status: accepted
supersedes:
related: ["[[ADR-0002-v3-target-selection]]", "[[2026-11-01-btop-tui-and-fresh-install]]"]
---

# ADR-0010 — A full LibreQoS run before the first full run

## Context

A new LibreQoS 2.x has no subscriber data in `/opt/libreqos/src`. The tool writes `ShapedDevices.csv` and `network.json` in `apply` mode.
`LibreQoS.py --updateonly` reads `<state>/shaping/ShapedDevices.lastLoaded.csv`. Only a full run writes that file. Before the first full run, `--updateonly` fails.

## Decision

1. If `<libreqos.state_directory>/shaping/ShapedDevices.lastLoaded.csv` does not exist, the reload runs `v2_command` with `libreqos.v2_first_args` (default `[]`: a full run).
2. If the file exists, the reload uses `v2_args` (`--updateonly`).
3. A failed full run stays pending (`reload-pending`). The next cycle tries it again.
4. `lqosync3 check` gives a warning if `libreqos.state_directory` does not exist.
5. LibreQoS 3.x targets run no reload command. The rule does not apply to them.

## Rejected

- Key on a missing `ShapedDevices.csv`. A new LibreQoS can have a CSV without a full run. A CSV removed by hand on a live shaper then gives a full run. The first review found both cases.
- A marker file in the tool state. An old marker can force a full run later, for example after a v3 period.

## Consequences

A full run makes all queues again, so the traffic can stop for a short time. A wrong `libreqos.state_directory` gives a full run at each reload.
The rule depends on a LibreQoS 2.2 file path. Check it after a LibreQoS upgrade.
