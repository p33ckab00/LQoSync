# Proposed fix: keep the native node policy in `compile_full_like`

> [!warning] Reference only
> This change is not compiled and not tested. Do not apply it to the installed LibreQoS.
> It is a proposal for the LibreQoS maintainers. The defect and the evidence are in
> [../2026-11-02-libreqos-native-node-rate-loss.md](../2026-11-02-libreqos-native-node-rate-loss.md).

| Item | Value |
|------|-------|
| Upstream commit | `2d922cb6d6ceb14b554d12b88d3fcbb902af9919` (LibreQoS 3.0.7) |
| File | `src/rust/lqos_topology_compile/src/compile_mode.rs` |
| Function | `compile_full_like` (modes `full` and `full2`) |
| Test | [native_full_keeps_node_policy.rs](native_full_keeps_node_policy.rs) |

## Problem in one sentence

`compile_full_like` builds the canonical nodes again from the editor projection and the compatibility tree.
For a schema 2 input, these two sources have no node kind and no node rate, so the compile loses the correct values in `imported.native_canonical`.

## Change

```diff
--- a/src/rust/lqos_topology_compile/src/compile_mode.rs
+++ b/src/rust/lqos_topology_compile/src/compile_mode.rs
@@ fn compile_full_like(
-    imported: ImportedTopologyBundle,
+    mut imported: ImportedTopologyBundle,
     mode: TopologyCompileMode,
 ) -> Result<CompiledTopologyBundle> {
     let source = compiled_source(&imported.source, mode);
     let ingress_identity = compiled_ingress_identity(&imported, mode);
     let mut editor = sanitized_editor_state(&source, imported.generated_unix, &imported);
     editor.ingress_identity = ingress_identity.clone();
     let mut compatibility_network_json = imported.compatibility_network_json.clone();
     let mut canonical = TopologyCanonicalStateFile::from_editor_and_network(
         &editor,
         &compatibility_network_json,
         TopologyCanonicalIngressKind::NativeIntegration,
     );
+    // The editor projection has no node kind and no rate. Put back the native values
+    // before the compatibility tree is exported from `canonical` (below).
+    if let Some(native) = imported.native_canonical.take() {
+        restore_native_node_policy(&mut canonical.nodes, native.nodes);
+    }
     canonical.source = source.clone();
```

New function in the same file:

```rust
/// Puts back the node kind and the rate input of the native import.
///
/// `from_editor_and_network` builds each node from the editor projection, which has no
/// kind and no rate. For a native source (`libreqos_network`, schema 2), the compatibility
/// tree is empty, so the built node has neither. A value that the build found (for example
/// an attachment rate, `AttachmentMax`) stays.
///
/// `canonical` is sorted by `node_id` (`from_editor_and_network` sorts it), so a binary
/// search finds each node. The native nodes are moved, not copied.
fn restore_native_node_policy(
    canonical: &mut [TopologyCanonicalNode],
    native: Vec<TopologyCanonicalNode>,
) {
    for mut src in native {
        let Ok(i) = canonical.binary_search_by(|n| n.node_id.as_str().cmp(&src.node_id)) else {
            continue;
        };
        let node = &mut canonical[i];
        if node.node_kind.is_empty() {
            node.node_kind = std::mem::take(&mut src.node_kind);
        }
        let rate = &node.rate_input;
        let no_rate = rate.intrinsic_download_mbps.is_none()
            && rate.intrinsic_upload_mbps.is_none()
            && rate.legacy_imported_download_mbps.is_none()
            && rate.legacy_imported_upload_mbps.is_none();
        if no_rate {
            node.rate_input = std::mem::take(&mut src.rate_input);
        }
    }
}
```

## Why this place and this form

The design follows the structure of the 3.0.7 compiler:

- **The defect is in `compile_full_like`, so the change is there.** The reader (`into_imported_bundle`) and `from_editor_and_network` are correct for their inputs.
  A change of `from_editor_and_network` would change its signature and its other callers (`compile_mode.rs:1426`, `topology_canonical_state.rs` tests).
- **The editor build stays the source of the parents and attachments.** `sanitized_editor_state` applies the parent rules to the native nodes. The change keeps that result.
  It adds only the two fields that the editor projection cannot carry.
- **The change does not replace a value that the build found.** An attachment rate or a compatibility snapshot rate stays. So the change has no effect on the `shaped_devices` pair, where the build finds the rates in `network.json`.
- **The change runs before the export of the empty compatibility tree** (`compile_mode.rs:668-675`). The export calls `insight_topology_network_json()`, which reads `canonical`. So the export gets the correct rates and kinds, and the `unwrap_or(1)` fallback does not apply.
- **`take()` is safe.** `compile_and_write_imported` (`ingest.rs:167`) makes `TopologyImportFile` from `&imported` before it calls `compile_topology`.
  `compile_full_like` does not read `imported.native_canonical` after this step.
  `TopologyCanonicalNode` and `TopologyCanonicalRateInput` derive `Default`, so `std::mem::take` is available.
- **The imports are already present.** `compile_mode.rs` already imports `TopologyCanonicalNode`.

## Cost

The change runs once for each compile, not for each packet or each cycle of the dataplane.

| Item | Cost |
|------|------|
| Time | O(M log N): one binary search in `canonical` for each of the M native nodes |
| Memory | No new allocation: no index map, no copy of a string or a rate. The native nodes are moved. |
| Order | The order of `canonical.nodes` does not change. Later code that expects the sort by `node_id` is not affected. |

A `HashMap` index is also correct. It needs one allocation and has no speed advantage at the node counts of a topology (tens to thousands).
For 5 nodes, as on the production shaper, the difference is not measurable.

## Rejected options

- **Use `native_canonical` as the canonical state, without the editor build.** This removes the parent sanitization of `sanitized_editor_state`.
- **Fill the compatibility tree from `native_canonical` before `from_editor_and_network`.** This writes a compatibility tree for schema 2. The comment `lqos-74t.150 #110` shows that the v2 format leaves this tree empty on purpose.
- **Change the `unwrap_or(1)` fallback.** The fallback is not the cause. A node without a rate must still get a value, and the correct value is in `native_canonical`.

## Verification steps for the maintainers

1. Add [native_full_keeps_node_policy.rs](native_full_keeps_node_policy.rs) to `src/rust/lqos_topology_compile/tests/`.
2. Run the test without the change. The result must be a failure in `full` and `full2`.
3. Apply the change.
4. Run the test again. The result must be a pass.
5. Run all tests of `lqos_topology_compile` and `lqos_config`.
6. Run [repro.py](repro.py) with a binding that has the change. The exit code must be 0.
