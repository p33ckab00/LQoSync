---
type: knowledge
created: 2026-10-31
tags: [updatecsv, python, rewrite]
status: draft
related: ["[[ADR-0003-router-failed-vs-offline]]", "[[ADR-0004-toml-policy-python-defaults]]", "[[2026-10-31-updatecsv-dissection]]"]
---

# Dissection of `updatecsv.py`

Source: `/opt/libreqos/src/updatecsv.py`, 1,815 lines, 67 functions, read in full on 2026-10-31.
This note keeps only the facts that change the Rust design. The full tables are in `docs/CONFIGURATION.md` and `docs/ARCHITECTURE.md`.

## State

- `ShapedDevices.csv` is the only state. The row key is Circuit Name.
- Circuit ID and Device ID are 8 random characters (`A-Z 0-9`). There is no collision check. A subscriber that returns gets new IDs.
- The Comment column is the source type (`PPP`, `HS`, `DHCP`) or the DHCP host name. A row with Comment `static` is never removed.
- With `preserve_network_config = false`, `network.json` is built again from an empty object in each cycle.

## Live production values (2026-10-31)

- Mode `root_promotion`. One router. API port 8728. PPPoE `per_plan_node = true`. Hotspot off. Four DHCP servers, all `per_site`.
- 27 circuits, 5 top-level nodes.
- `pppoe_aggregate_factor = 0.35`, `min_rate_percentage = 0.1`, PPPoE cap 620, DHCP cap 250.
- Check: `Tier-15M-WAN1` node = 9 × 50 × 0.35 = 157.5. `DHCP-LAN` node = 4 × 50 × 1.0 = 200.0. Both agree with the live file.

## Defects to fix

- One router fails, a different router succeeds: the cleanup removes the subscribers of the failed router. Refer to [[ADR-0003-router-failed-vs-offline]].
- `get_resource_data` returns an empty list on a read error.
- No API timeout. The routers are read one after the other.
- Files are written in place, not by a rename.
- The preflight result is ignored.
- The duplicate IP check runs only for new codes.
- The profile name rule reads the last number in the name.

## Output format for a shadow compare

- CSV rates use Python `str(float)`: `50.0`, not `50`.
- `network.json` uses `indent=4`, `sort_keys=True`, `ensure_ascii=False`, and a final newline.
- CSV rows are sorted by Parent Node, Circuit Name, Device Name, IPv4, MAC.
