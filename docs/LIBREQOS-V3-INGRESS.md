# LibreQoS v3 ingress

This document records what LibreQoS 3.0.7 reads as topology input. The data comes from an offline simulation on 2026-10-31.
LibreQoS v3 was not installed. The production shaper operates LibreQoS 2.2.

> [!warning] Evidence limit
> The data comes from the package files and the v3 Python binding. It does not come from a v3 `lqosd` that operates.
> The `lqosd` file watcher, the last-good behavior, and the shaping result need a test on a v3 shaper.

## Source package

| Item | Value |
|------|-------|
| Package | `libreqos_3.0.7.202610310038_amd64.deb` (beta repository) |
| SHA-256 | `2ba522e646366354fd941e482fa0afd2e55d0a426c968190ad3b7b9979f88a49` |
| Control plane | `/usr/libexec/libreqos/lqosd`, unit `libreqos-lqosd.service` |
| Configuration | `/etc/libreqos/lqos.conf` (environment `LQOS_CONFIG`) |
| Directories | `lqos_directory = "/opt/libreqos/src"`, `state_directory = "/opt/libreqos/state"` |

## Topology sources

The `[topology]` section of `lqos.conf` selects one source:

```toml
[topology]
source = "shaped_devices"   # uisp | splynx | netzur | visp | sonar | powercode | wispgate | shaped_devices | libreqos_network
compile_mode = "ap_site"    # flat | ap_only | ap_site | full
```

| Source | Input | Owner |
|--------|-------|-------|
| Built-in integration | `state/topology/topology_import.json` | LibreQoS. The format is internal. Do not write it. |
| `shaped_devices` | `ShapedDevices.csv` + `network.json` | The operator or a custom tool |
| `libreqos_network` | `libreqos_network.json` | The operator or a custom tool |

The scheduler does not produce topology for `libreqos_network`. The `lqosd` watcher owns it.

## Reload behavior

- In 3.0.7, `LibreQoS.py --updateonly` prints `refreshShapersUpdateOnly: skipping the legacy file chain.` and does nothing.
- `lqosd` watches the input files with inotify. The log strings are `ShapedDevices reload reason=` and `NetworkJson reload reason=`.
- The watcher does not accept a symbolic link (`watched path is a symlink; a regular file is required`).
- If `libreqos_network.json` is not valid, `lqosd` keeps the last good topology (`libreqos_network.json rejected; last-good topology retained`).

## `ShapedDevices.csv` in v3

- The v3 example adds the optional column `Anchor Node ID`.
- The current 13-column file from `updatecsv.py` passed `libreqos-v3-preflight`: `PASS`, `topology: validated source=legacy-pair`, `circuits: at least 27`.

## `libreqos_network.json` schema 2

The reader accepts only `schema_version = 2`. It rejects unknown fields.

```json
{
  "schema_version": 2,
  "match": { "mode": "ip" },
  "topology": [
    {
      "id": "r1", "type": "site", "parent": null, "name": "Router-1",
      "rate": { "commit_down_mbps": 100, "commit_up_mbps": 100, "ceil_down_mbps": 640, "ceil_up_mbps": 640 }
    }
  ],
  "circuits": [
    {
      "id": "c1", "parent": "r1", "display_name": "Subscriber 1",
      "plan": { "commit_down_mbps": 5, "commit_up_mbps": 5, "ceil_down_mbps": 10, "ceil_up_mbps": 10 },
      "devices": [ { "id": "d1", "name": "cpe-1", "mac": "AA:BB:CC:DD:EE:FF", "ipv4": ["10.0.0.2"], "ipv6": [], "comment": "" } ],
      "match": { "prefixes": ["10.0.0.2/32"] }
    }
  ]
}
```

### Fields

| Object | Fields | Required |
|--------|--------|----------|
| File | `schema_version`, `match`, `topology`, `circuits` | `schema_version` |
| `match` (file) | `mode`: `ip`, `vxlan`, `vxlan+vlan`, `vlan`, `mpls`. `steering_vlans`: list | — |
| Node (`topology[]`) | `id`, `type` (`site`, `node`, `ap`), `parent`, `name`, `latitude`, `longitude`, `rate` | `id`, `type`, `parent`, `name` |
| Rate, plan | `commit_down_mbps`, `commit_up_mbps`, `ceil_down_mbps`, `ceil_up_mbps` | — |
| Circuit | `id`, `parent`, `display_name`, `plan`, `devices`, `match` | `id`, `parent`, `plan` |
| Device | `id`, `name`, `mac`, `ipv4` (list), `ipv6` (list), `comment` | `id` |
| `match` (circuit) | `prefixes`, `vni`, `vlan`, `label` | `prefixes` in `ip` mode |

### Validation results

| Input | Result |
|-------|--------|
| `schema_version = 1` | Rejected: `schema_version 1 is not supported (this reader accepts 2)` |
| Unknown field | Rejected |
| Commit rate above the ceiling rate | Rejected: `commit must not exceed ceil` |
| Circuit parent that is not a node | Rejected: `parent 'nope' does not name a topology node` |
| Two circuits with the same ID | Rejected: `duplicate circuit id 'c1'` |
| Two nodes with the same ID | Rejected: `duplicate node id 'a'` |
| Two nodes with the same name | **Accepted.** LibreQoS 2.x rejects duplicate node names. |
| Circuit `parent = null` | Rejected: `invalid type: null, expected a string` |
| `ip` mode, circuit without devices | Rejected: `ip mode requires at least one device` |
| `ip` mode, empty `prefixes` | Rejected: `ip mode requires a non-empty 'prefixes' list` |
| Two circuits with the same IP | **Accepted.** The tool must reject this case. |

### Differences from the pair

- The node type `plan` does not exist. Use `node`.
- The compiler does not keep the device `comment`.
- Without `display_name`, the circuit name is the circuit ID.
- With `compile_mode = "flat"`, the circuits have no parent node.
- Each circuit must have a parent node. The network mode `no_parent` cannot use `v3_network`. Use `v3_pair` for that mode.

## Simulation method

The simulation used the v3 Python binding `liblqos_python.so` from the extracted package. It did not use `lqosd`.

1. Extract the package with `dpkg-deb -x` to a scratch directory. Do not install it.
2. Write a test `lqos.conf` with `lqos_directory` and `state_directory` in the scratch directory.
3. Set `LQOS_CONFIG` to the test file.
4. Call `write_compiled_topology_from_network_json_payload(source, compile_mode, json)` for `libreqos_network`.
5. Call `write_compiled_topology_from_legacy_artifacts(source, compile_mode)` for the pair.
6. Compare `state/shaping/topology_compiled_shaping.json` from both paths.

> [!caution]
> On the production shaper, `/run/lqos/bus` has mode `0777`. A process can send commands to the production `lqosd`.
> Run the binding under `strace -e inject=connect:error=EACCES` to block all socket connections. Do not call `validate_shaped_devices`, because it uses the bus.

### Result with the production data

The production pair has 5 nodes and 27 circuits. Both paths gave 27 devices with the same circuit ID, parent, IP, and rates. The only difference was the device `comment`.

## Open items

- [ ] Find the path that `lqosd` watches for `libreqos_network.json`. The probable path is `<lqos_directory>/libreqos_network.json`.
- [ ] Find the file that shows that `lqosd` accepted a new generation.
- [ ] Test the watcher and the last-good behavior on a v3 test shaper.
- [ ] Find if `lqosd` fills the node type and rate in `topology_canonical_state.json` for `libreqos_network`. The offline compiler leaves them empty.
