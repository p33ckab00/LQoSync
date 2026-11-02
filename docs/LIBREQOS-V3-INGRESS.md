# LibreQoS v3 ingress

This document records what LibreQoS 3.0.7 reads as topology input.
The initial evidence came from an offline simulation on 2026-10-31.
The production shaper completed its v3 cutover on 2026-11-02.
The live `v3_pair` path now has evidence from input files, imported topology, and applied runtime state.

> [!warning] Evidence limit
> The native `v3_network` path still has offline evidence only.
> Live checks cover the current `shaped_devices` pair and full topology mode.
> Rejection recovery and interrupted multi-file updates have not had live fault tests.

## Live input ownership

With `source = "shaped_devices"`, LQoSync writes both `/opt/libreqos/src/ShapedDevices.csv` and `/opt/libreqos/src/network.json`.
The CSV supplies circuits and device details. The JSON supplies their parent topology and node rates.
LibreQoS reads this pair and owns the derived state under `/opt/libreqos/state/`.
It does not replace LQoSync as the writer of the source pair.

The live check found 26 circuits and five site nodes, with no missing parent names or IDs.
The imported compatibility network equals the source `network.json`. The imported circuit IDs equal the source CSV IDs.
The operator view contains 26 circuit nodes and five site nodes. The runtime topology is ready.

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
- With `source = "shaped_devices"`, the scheduler imports the pair into `state/topology/topology_import.json`.
  The scheduler log shows an import at start, on `Reload requested by admin`, and every 30 minutes (`importAndShapePartialReload`).
  On 2026-11-02, no log line showed an import that a file write started. Without a reload request, a new pair can wait up to 30 minutes.
- The scheduler reads `state/scheduler/reload_request.json` each second (`reload_request_tick`). The LibreQoS UI reload button uses this file.
  The token is `{"requested_at": <Unix seconds>, "nonce": "<unique>", "requested_by": "<name>"}`.
  The scheduler ignores a token older than its start or with a nonce that it used before. It deletes the file and runs one `importAndShapeFullReload`.
  This interface is internal to LibreQoS. An upgrade can change it.
- After each change of the pair, LQoSync writes this token with `requested_by = "lqosync3"`. It writes a temporary file, then renames it.
- LQoSync compares `imported.compatibility_network_json` and `imported.shaped_devices` with the files that it wrote.
  The status shows `waiting for the LibreQoS import (<n> s)` until they agree, then `imported by LibreQoS`.
  After 40 minutes without agreement, the cycle reports the error `LibreQoS did not import the new files`.

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
| Rate, plan | `commit_down_mbps`, `commit_up_mbps`, `ceil_down_mbps`, `ceil_up_mbps` | All four, if the object is present (`missing field commit_down_mbps`) |
| Circuit | `id`, `parent`, `display_name`, `plan`, `devices`, `match` | `id`, `parent`, `plan` |
| Device | `id`, `name`, `mac`, `ipv4` (list), `ipv6` (list), `comment` | `id` |
| `match` (circuit) | `prefixes`, `vni`, `vlan`, `label` | `prefixes` in `ip` mode |

### Validation results

| Input | Result |
|-------|--------|
| `schema_version = 1` | Rejected: `schema_version 1 is not supported (this reader accepts 2)` |
| Unknown field | Rejected |
| Commit rate above the ceiling rate | Rejected: `commit must not exceed ceil` |
| Node `rate` with only the ceiling values | Rejected: `missing field commit_down_mbps` |
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

The repeated check on 2026-11-02 used five nodes and 26 circuits from the operating v3 shaper.
Both installed-binding compiler calls passed. Compiled circuit fields matched, except device comments.
However, the native compiled canonical output lost node kinds and rates. Its compatibility tree emitted 1 Mbps node limits.
The native import retained the supplied policy before compilation. The pair compiled output retained it after compilation.
This difference blocks a production source migration until node policy preservation is proven.
The check did not exercise the live native watcher or change the active source.
Artifacts are under `/home/pi/.local/state/libreqos-native-check-s12invb1`.

## Open items

- [x] Confirm the watched native input path: `<lqos_directory>/libreqos_network.json`, resolved through `legacy_runtime_file_path`.
- [x] Find the file that shows that `lqosd` accepted a new generation. For `shaped_devices`: `state/topology/topology_import.json`. `topology_runtime_status.json` shows `ready` and `source_generation`.
- [ ] Find if a write of the pair starts an import before the 30-minute job. Examine the first live LQoSync change.
- [ ] Test the watcher and the last-good behavior on a v3 test shaper.
- [ ] Find if `lqosd` fills the node type and rate in `topology_canonical_state.json` for `libreqos_network`. The offline compiler leaves them empty.
