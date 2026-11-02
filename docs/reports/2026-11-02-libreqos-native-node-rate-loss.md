# LibreQoS 3.0.7: the native topology compiler loses node kinds and rates

| Item | Value |
|------|-------|
| Date | 2026-11-02 |
| LibreQoS | `libreqos 3.0.7.202610310038`, upstream commit `2d922cb6d6ceb14b554d12b88d3fcbb902af9919` |
| Binding | `/opt/libreqos/src/liblqos_python.so`, SHA-256 prefix `cf714dd835ba696e` |
| Affected | `[topology] source = "libreqos_network"` with `compile_mode = "full"` or `"full2"` |
| Not affected | `source = "shaped_devices"` (the production source) |
| Result | Defect confirmed. Each node gets 1 Mbps in the compiled compatibility tree. |
| Evidence | [`repro.py`](2026-11-02-libreqos-native-node-rate-loss/repro.py), synthetic data, exit code 1 |

## Summary

LQoSync writes `libreqos_network.json` with `schema_version = 2` and a `rate` for each node.
The LibreQoS reader imports these node kinds and rates correctly into `native_canonical`.
The `full` and `full2` compile step then builds the canonical nodes again from data that has no kinds and no rates.
The export of the compatibility tree uses a 1 Mbps fallback for each node without a rate.

The runtime takes its node limits from this compiled state.
An earlier isolated runtime test found 1 Mbps ceilings for all groups and all leaves in the final dataplane transaction.
See [[2026-11-02-native-runtime-policy-validation]].

The defect is in LibreQoS. The LQoSync output is correct.

## Reproduction

The script compiles one synthetic topology with the installed binding in each mode.
It compiles the same nodes and circuit as a `shaped_devices` pair (the control) and as `libreqos_network.json`.

- `SITE-A`: type `site`, 200/100 Mbps
- `PLAN-A`: type `node`, 50/25 Mbps, parent `SITE-A`
- `REPROC1`: one circuit below `PLAN-A`, plan 5/2.5 commit, 50/25 ceiling

Run the script as a normal user:

```bash
python3 -I docs/reports/2026-11-02-libreqos-native-node-rate-loss/repro.py
```

Each case has its own scratch folder, `lqos.conf`, input folder, and state folder.
The binding reads the case config through `LQOS_CONFIG`.
Each compile runs under `strace`, and each `connect()` call fails with `EACCES`.
If the trace shows a write outside the scratch folder, the script stops with exit code 2.
The live input files, the live state, and the services do not change.

### Result

| Case | Compile | `SITE-A` (Mbps) | `PLAN-A` (Mbps) |
|------|---------|-----------------|-----------------|
| `shaped_devices/full` (control) | ok | 200/100 | 50/25 |
| `libreqos_network/full` | ok | **1/1** | **1/1** |
| `shaped_devices/full2` (control) | ok | 200/100 | 50/25 |
| `libreqos_network/full2` | ok | **1/1** | **1/1** |

The values come from `compatibility_network_json` in `state/topology/topology_canonical_state.json`.
Both compile calls return `True`. LibreQoS does not report an error.

### Files of the `libreqos_network/full` case

The import has the correct node policy (`state/topology/topology_import.json`, `imported.native_canonical`):

```json
[{"node_id": "repro-site-a", "node_kind": "site", "rate_input": {"intrinsic_download_mbps": 200, "intrinsic_upload_mbps": 100, "source": "unknown"}},
 {"node_id": "repro-node-a", "node_kind": "node", "rate_input": {"intrinsic_download_mbps": 50, "intrinsic_upload_mbps": 25, "source": "unknown"}}]
```

The import has an empty compatibility tree: `"compatibility_network_json": {}`.

The compiled canonical state has no kinds and no rates (`state/topology/topology_canonical_state.json`):

```json
[{"node_id": "repro-node-a", "node_kind": "", "rate_input": {"source": "unknown"}},
 {"node_id": "repro-site-a", "node_kind": "", "rate_input": {"source": "unknown"}}]
```

The compiled compatibility tree has 1 Mbps nodes. `PLAN-A` also changes from `node` to `Site`:

```json
{"SITE-A": {"id": "repro-site-a", "type": "Site", "downloadBandwidthMbps": 1, "uploadBandwidthMbps": 1,
  "children": {"PLAN-A": {"id": "repro-node-a", "type": "Site", "downloadBandwidthMbps": 1, "uploadBandwidthMbps": 1, "children": {}}}}}
```

The circuit plan stays correct in `state/shaping/topology_compiled_shaping.json` (5/2.5 commit, 50/25 ceiling).
The runtime limits the circuit to the 1 Mbps parent.

## Cause

The source references are for upstream commit `2d922cb6`, the source of the installed 3.0.7 package.

1. `lqos_topology_compile/src/libreqos_network.rs`, `into_imported_bundle`:
   the reader puts the node kinds and rates into `imported.native_canonical`.
   For schema 2, `imported.compatibility_network_json` is empty.
2. `lqos_topology_compile/src/compile_mode.rs:645`, `compile_full_like` (`full` and `full2`):
   - Line 651: `sanitized_editor_state` makes an editor projection of `native_canonical`.
     `TopologyEditorNode` (`lqos_config/src/topology_editor_state.rs:210-248`) has no node kind and no rate field.
   - Line 654: `from_editor_and_network(&editor, &compatibility_network_json, ...)` builds the canonical nodes again.
     The compatibility tree is empty, so it finds no node snapshot.
   - The function does not read `imported.native_canonical` after this step. The correct kinds and rates are not used.
3. `lqos_config/src/topology_canonical_state.rs:1514`, `from_editor_and_network`:
   without a snapshot, `node_kind` is empty (`unwrap_or_default`).
   `canonical_rate_input_from_network_snapshot` (line 775) gives no rates and `source = unknown`.
4. `compile_mode.rs:668-675`: the compatibility tree is empty, so the compiler exports `canonical.insight_topology_network_json()`.
5. `topology_canonical_state.rs:730-737`, `build_insight_logical_entry_json`:
   a node without a rate takes the highest child rate, else `unwrap_or(1)`.
   The leaf node gets 1 Mbps. Its parent site then also gets 1 Mbps.
   `normalized_insight_node_kind` (line 553) changes an empty kind to `Site`.

The upstream `main` branch has the same `compile_full_like` code on 2026-11-02.
A search of the upstream issues found no report of this defect.

## Proposed test for upstream

[`native_full_keeps_node_policy.rs`](2026-11-02-libreqos-native-node-rate-loss/native_full_keeps_node_policy.rs) is an integration test for the `lqos_topology_compile` crate.
It parses the same synthetic schema 2 input and calls `compile_topology` with `Full` and `Full2`.
It asserts that the import keeps the policy, and that the compiled canonical state and the compatibility tree keep it too.

> [!note] Not run
> The test was not compiled. The public upstream workspace refers to `rust/libreqos_dpdk`, which is not in the public source.
> A build of the crate outside the workspace needs third-party crates from the network. That step was not authorized in this task.
> The Python reproduction runs the same logic in the installed binary.

## Proposed correction for upstream

Not compiled and not tested. See [proposed-fix.md](2026-11-02-libreqos-native-node-rate-loss/proposed-fix.md):
the change, the reasons for its place and form, its cost, and the rejected options.

## Other compile modes

`ap_site`, `ap_only` and `flat` also give a different result for the native source.
The synthetic topology has no AP. In these modes, the compiler puts a circuit without an AP above it at the root, and it removes the nodes.
This is the documented projection, so this report does not classify it as a defect.
The control case is not a valid comparison for these modes: the pair path writes `source = "legacy/manual"` and keeps the `network.json` tree in each mode.
A test with AP nodes is necessary to examine these modes.

## Consequences for LQoSync

- Keep `[topology] source = "shaped_devices"` on the production shaper.
- With `target = "auto"`, a change of the source to `libreqos_network` selects `v3_network`.
  Then the shaper limits each node to 1 Mbps, and each subscriber below it.
- Run `repro.py` again after each LibreQoS upgrade. Exit code 0 means that the defect is gone.

## Upstream issue draft

**Title:** `libreqos_network` (schema 2) with `compile_mode = full`/`full2` drops node kinds and rates, and nodes export at 1 Mbps

**Body:**

LibreQoS 3.0.7 (`2d922cb6`). With `[topology] source = "libreqos_network"` and `compile_mode = "full"` or `"full2"`, every topology node in the compiled canonical state has an empty `node_kind` and no `rate_input`.
`compatibility_network_json` then exports each node with `downloadBandwidthMbps = 1` and `uploadBandwidthMbps = 1`, and type `Site`.
The runtime applies 1 Mbps group ceilings, which also limit every circuit below them.
The same nodes as a `shaped_devices` pair compile correctly.

Steps:
1. Use a `libreqos_network.json` with one `site` (200/100) and one child `node` (50/25), each with a `rate`.
2. Compile it with `write_compiled_topology_from_network_json_payload("libreqos_network", "full", text)`.
3. Read `state/topology/topology_canonical_state.json`.

Expected: kinds `site` and `node`, rates 200/100 and 50/25.
Actual: empty kinds, `rate_input = {"source": "unknown"}`, 1/1 Mbps in `compatibility_network_json`.

Cause: `compile_full_like` rebuilds canonical nodes with `from_editor_and_network(editor, imported.compatibility_network_json)`.
For schema 2, the compatibility tree is empty and `TopologyEditorNode` has no kind or rate, so the policy in `imported.native_canonical` is lost.
`build_insight_logical_entry_json` then falls back to `unwrap_or(1)`.
A reproduction script, a regression test, and a proposed change are attached.
