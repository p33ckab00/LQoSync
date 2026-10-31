---
type: spec
created: 2026-10-31
status: accepted
related: ["[[ADR-0002-v3-target-selection]]", "[[ADR-0005-topology-follows-v3]]", "[[ADR-0006-v3-guards-and-node-types]]", "[[libreqos-v3-ingress-facts]]", "[[v3-node-handling]]"]
---

# lqosync3 LibreQoS v3 targets (Plan 3)

## Purpose

Write the topology input that LibreQoS 3.x reads, and select the target from the installed LibreQoS. The facts come from `docs/LIBREQOS-V3-INGRESS.md` and the offline simulation of 3.0.7.

## Target selection (ADR-0002)

With `libreqos.target = "auto"`, the tool selects the target at the start of each cycle:

1. Read the version of the package `libreqos` from `libreqos.dpkg_status` (default `/var/lib/dpkg/status`). The package must have the status `installed`.
2. Version `2.x`: `v2_pair`.
3. Version `3.x`: read `[topology] source` from `libreqos.v3_config` (default `/etc/libreqos/lqos.conf`).
   - `shaped_devices`: `v3_pair`.
   - `libreqos_network`: `v3_network`.
   - Any other value, or no value: stop the cycle with an error. A built-in integration owns the topology.
4. No package, or another version: stop the cycle with an error.

A fixed target in TOML is not changed by the detector. `lqosync3 check` and `lqosync3 detect` show the selected target and the reason.

## `v3_pair`

- `ShapedDevices.csv` with 15 columns: `Parent Node ID` and `Anchor Node ID` after `Parent Node`. `Anchor Node ID` stays empty.
- `network.json` as for `v2_pair`, with the `v3_type` node types.
- No reload command. `lqosd` watches the files. The status shows `reload: lqosd watches the files`.

## `v3_network`

The tool writes only `<libreqos.directory>/<libreqos.v3_network_file>` (default `libreqos_network.json`). It does not write the pair.

```json
{
  "schema_version": 2,
  "match": { "mode": "ip" },
  "topology": [ { "id": "<node id>", "type": "site", "parent": null, "name": "<node name>",
                  "rate": { "commit_down_mbps": 200, "commit_up_mbps": 200, "ceil_down_mbps": 200, "ceil_up_mbps": 200 } } ],
  "circuits": [ { "id": "<Circuit ID>", "parent": "<node id>", "display_name": "<Circuit Name>",
                  "plan": { "commit_down_mbps": 5, "commit_up_mbps": 5, "ceil_down_mbps": 50, "ceil_up_mbps": 50 },
                  "devices": [ { "id": "<Device ID>", "name": "<Device Name>", "mac": "<MAC>",
                                 "ipv4": ["10.0.0.2"], "ipv6": [], "comment": "<Comment>" } ],
                  "match": { "prefixes": ["10.0.0.2/32"] } } ]
}
```

- Nodes come from the same node tree as `network.json`, in tree order (a parent before its children).
- The node rate must have all four values (the v3 parser rejects a rate without `commit_down_mbps`). A node has one rate, so commit = ceil. The v3 import of a pair does the same.
- A node without `id` gets `lq3-` and a hash of its name. A node without `type` is a `site`, as in LibreQoS 2.x.
- A node type must be `site`, `node`, or `ap`. Another type stops the cycle with an error that names the node and the `[nodes]` key.
- Use `compile_mode = "full"`. With `ap_site` or `flat`, the v3 compiler gives no parent to the circuits of this file.
- The circuit parent is the `Parent Node ID` of the row. If it is empty, the tool finds the node by the `Parent Node` name.
- The rows of one Circuit ID make one circuit with one device for each row.
- An IPv4 value can hold more than one address, separated by commas. Each address gets a `/32` prefix. An IPv6 value is used as given.
- The tool skips a circuit and writes an alert line if:
  - the circuit has no parent node (v3 requires a parent),
  - the circuit has no IP address (`ip` mode requires a prefix),
  - a circuit ID, device ID, or node ID does not match `[A-Za-z0-9._-]{1,64}` with no `.` or `-` at the start or end.
- The duplicate IP rule and the node name rule of the cycle apply, because v3 accepts duplicates.
- No reload command. `lqosd` keeps the last good topology if it rejects the file.

## Config changes

- New keys: `libreqos.dpkg_status = "/var/lib/dpkg/status"`, `libreqos.v3_network_file = "libreqos_network.json"`.
- `libreqos.v3_status_timeout` stays, but has no effect in Plan 3. The file that shows that `lqosd` accepted a new generation is not known yet.

## CLI

- `lqosync3 detect`: print the selected target and the reason. Exit 1 if the detection fails.
- `lqosync3 diff`: the pair targets only. For `v3_network`, it stops with an error.

## Out of scope

- The check that `lqosd` accepted the files (needs a v3 shaper).
- `libreqos-v3-preflight` and `notify_on_preflight_warning`.
- A test on an operating v3 shaper. The writer is checked with the offline v3 parser.

## Offline check result (2026-10-31)

The production data (5 nodes, 27 circuits) went through `libreqos_network.json` and through the pair into the 3.0.7 binding, `compile_mode = "full"`. The compiled devices are the same in both paths, except `comment`. No socket connection left the process, and no file was written outside the scratch directory.
