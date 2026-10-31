---
type: decision
created: 2026-10-31
tags: [adr, configuration]
status: superseded in part by ADR-0007
supersedes:
related: ["[[ADR-0003-router-failed-vs-offline]]", "[[updatecsv-py-dissection]]"]
---

# ADR-0004 — All policy in TOML, with defaults that give the `updatecsv.py` output

## Context

`updatecsv.py` has many fixed values in the code: retries, delays, node names, the static keyword, the speed source order, and the rate direction.
The operator wants these values in TOML, not in the code.
The cutover needs a `shadow` compare with the Python service. A compare is useful only if the defaults give the same output.

## Decision

1. Put each policy value and tuning value in TOML.
2. Keep in the code only what LibreQoS or RouterOS defines: API paths, CSV columns, file names, and the `libreqos_network.json` schema.
3. Set each default so that the output is the same as `updatecsv.py`.
4. These defaults are different, because they fix a defect or add an operator rule:
   - DHCP speed sources: `["server_comment", "settings"]`. The operator selected the server comment first.
   - Profile name rule: read a number with a unit (`Tier-15M-WAN1` gives 15, not 1).
   - Comment units: `k`, `M`, and `G`.
   - Router failure: refer to [[ADR-0003-router-failed-vs-offline]].
5. Make the speed source order a TOML list for PPPoE and for each DHCP server.

## Alternatives

1. New defaults with the RouterOS rate direction and units. The shadow compare then shows differences that are not defects.
2. Fixed values in the code, as in `updatecsv.py`. Each change then needs a new build.

## Consequences

- The open item "speed rule direction and units" becomes two TOML keys: `speed.rate_limit_order` and `speed.unitless_unit`. The operator can change them after the cutover.
- A change of a `[circuits]` template on a live system gives new Circuit Names and new IDs. The documents give a caution.
- The loader must reject values out of range.
- Refinement on 2026-10-31: `k` and `G` apply only to a rate pair (`1G/500M`). A single value accepts only `M`, because a name like `Wifi 5G` is not a speed.

## Status
accepted on 2026-10-31 (operator decision in chat). Items 3 and 4 and the parity goal are superseded by [[ADR-0007-fresh-design-not-a-port]] on 2026-10-31.
