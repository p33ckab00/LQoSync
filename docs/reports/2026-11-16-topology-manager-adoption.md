# LibreQoS Topology Manager adoption research

Status: research and proposed design. No runtime implementation or deployment.
Reviewed: 2026-11-16, Asia/Shanghai. Agent: Codex `/root`.

Later owner decisions are in the [discussion record](../design/TOPOLOGY_MANAGER_DISCUSSION.md).
The [product review](2026-11-16-topology-product-review.md) supersedes this report's initial implementation recommendations where they differ.
Keep this report as source research, not as a newer approval record.

## Purpose and result

The owner wants LQoSync3 to adopt the LibreQoS Topology Manager approach.
The goal includes topology ownership in LQoSync, with LibreQoS enforcement.

The current source pair supports topology export. It does not enable native branch moves.
The manual importer explicitly creates fixed nodes with no allowed parent candidates.
The public native JSON format also lacks editor metadata.
Thus, a source switch alone does not solve native editor integration.

Two designs are possible:

1. Give LQoSync its own topology model and editor. Publish through the current source pair.
2. Extend the LibreQoS ingress contract. Supply editor metadata and reuse its native editor and override resolver.

The first design is the smaller initial change for LQoSync-owned topology.
The second design needs a separate compatibility investigation and an ingress extension.
These recommendations do not constitute an approved implementation design.

## Evidence scope

- Development checkout: `/home/pi/LQoSync3.0`, branch `lqosync3`, HEAD `0ad8b8d`.
- Separate checkout: `/opt/LQoSync3.0`, branch `lqosyncv3`, HEAD `0e12561`.
- Installed control and dataplane packages: `3.0.7.202611141944`.
- Public source reference: `9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d`.
- GitHub reported that public commit date as `2026-11-08T17:26:51Z`.
- Read the project graph, current handoff, relevant decisions, and native-source investigation notes.
- Read the installed UI source map and current topology state without subscriber data export.

The public source is a contract reference. It does not prove exact installed binary behavior.
Old v2 documentation is background only. V3 has a different dataplane.
The development checkout had unrelated changes before this work. Those changes remain intact.

## Native architecture

The implementation separates imported facts, saved operator intent, effective topology, and runtime observations.
The UI compares Canonical, Saved override, and Live state.
Discovery does not need to replace saved operator intent on every import.

The canonical node record includes:

- Stable node ID, display name, kind, and optional coordinates.
- Current parent and current attachment references.
- `can_move` and `allowed_parents`.
- Attachment options under each legal parent.
- Rate input metadata.

An attachment has its own identity, capacity, role, and rate source.
Probe health is a separate observation. It must not replace configured policy.
LQoSync can adopt these distinctions. Its wire format can differ.

Sources: [canonical model][canonical], [editor model][editor], [effective resolver][effective].

## Native commands and persistence

The installed UI uses authenticated WebSocket requests through its current request wrapper:

- `GetTopologyManagerState`.
- `SetTopologyManagerOverride` and `ClearTopologyManagerOverride`.
- `SetTopologyManagerProbePolicy`.
- `SetTopologyManagerManualAttachmentGroup` and its clear command.
- `SetTopologyManagerAttachmentRateOverride` and its clear command.

These are internal UI protocol names. They are not verified public REST endpoint paths.
Any external adapter must prove authentication, framing, permissions, errors, and version compatibility.

The public handler requires an admin for writes.
It checks the child edit policy, legal parent, and attachment preference IDs.
It builds candidate effective artifacts before persistence.
If publication fails, it attempts to restore the previous overrides.
This code does not prove dataplane acknowledgement or complete end-to-end rollback.

The current override implementation stores topology intent in the `topology` section of `lqos_overrides.json`.
It preserves sibling sections through the current store and lock path.
The former `topology_overrides.json` is a migration input, not a new integration interface.

Sources: installed `topology_manager.js.map`, [native handler][handler], [override store][overrides].

## Why the current source cannot unlock native moves

Live config selects `shaped_devices` with `compile_mode = "full"`.
All configured built-in integrations are disabled.
The canonical source is `legacy/manual`. Its four site records have no legal parents and cannot move.
The attachment health snapshot has no entries. The runtime status reports `ready: true`.
Readiness alone does not prove that the bridge accepted the desired world.

The public manual producer derives canonical nodes from `network.json`.
Its legacy conversion assigns `can_move: false` and an empty `allowed_parents` list.
A flag in the source JSON does not change that conversion.
The editor loader prefers the populated canonical model over legacy parent candidates.
A separate editor-state file is thus not a proven solution.

The strict schema-2 `NetworkNode` accepts identity, kind, parent, name, coordinates, and rate.
It has no fields for allowed parents or explicit attachments.
The mapper uses canonical defaults for the omitted editor fields.
This means the native JSON source is not a complete native-editor transport.

An earlier isolated test also found lost node policy on the native-source compilation path.
That result applies to the tested build. It does not establish the current build result.
Retest node types, minimums, ceilings, and runtime output before any migration.

Sources: [manual producer][bundle], [legacy conversion][canonical], [native JSON reader][native],
`brain/10-sessions/2026-11-02-native-topology-source-check.md`, and the follow-up runtime note.

## Proposed ownership for LQoSync

| Record | Proposed owner | Rule |
|---|---|---|
| Discovered subscriber and router facts | LQoSync collectors | Keep source identity and freshness |
| Approved sites and parent relationships | LQoSync topology store | Discovery proposes changes |
| Local profiles and subscriber assignments | LQoSync policy | Follow the current owner direction |
| Branch edits and attachment preferences | One designated editor | Do not run two competing writers |
| Source pair | Existing LQoSync writer | Export one resolved revision |
| Imported and compiled artifacts | LibreQoS | Read for verification |
| Applied shaping state | LibreQoS bridge | Verify separately from file acceptance |
| Packet routes and WAN assignment | RouterOS or a future routing adapter | A shaping parent does not select a route |

For the first design, LQoSync owns the edit store and resolves its own edits before export.
It should not also write native LibreQoS topology overrides for those same relationships.
For the second design, LibreQoS owns the override resolver after the adapter supplies canonical facts.
The ownership choice must precede implementation.

## Existing code to reuse

| LQoSync component | Current capability | Proposed extension |
|---|---|---|
| `src/topology.rs` | Stable IDs, group parents, rates, nested tree | Resolve approved topology before export |
| `src/model.rs` | Circuit rows and node state | Reference stable topology identities |
| `src/writer/pair.rs` | Current CSV and JSON output | Retain the current writer |
| `src/writer/v3network.rs` | Schema-2 output | Keep migration gated by policy-preservation proof |
| `src/validate.rs` | Source validation | Add explicit graph and edit validation |
| `src/runner.rs` and apply evidence modules | Publication and observations | Tie edit status to the published revision and native evidence |

Existing node IDs are deterministic. Some grouping keys contain names.
A future rename must preserve an approved node ID rather than recompute it from the new display name.
The current topology is generated from discovery groups. It is not yet a persistent operator-owned topology catalog.

## Proposed first implementation boundary

1. Import the current source tree as an initial catalog without changing its IDs or limits.
2. Keep discovered facts, approved topology, draft edits, and observed runtime state separate.
3. Permit node rename, directional capacity edits, and validated branch reparenting.
4. Show affected descendants and circuits before an explicit save.
5. Export through the current `v3_pair` path and retain its current checks.
6. Show saved, published, imported, and native-applied states separately.
7. Preserve the last approved revision for rollback through the same writer.

Later work can add attachments, preferred ordering, probe health, and native-editor integration.
Native manual groups currently require two to eight attachments and valid, distinct probe IPs for each attachment.
Do not promise native probe parity from a basic parent editor.

This boundary excludes automatic WAN routing, a new dataplane, and a direct write into LibreQoS generated state.
It does not authorize a live source switch or service restart.

## Acceptance checks before activation

- An unchanged initial import preserves node IDs, subscriber mappings, and directional limits.
- Reject duplicate IDs, missing parents, self-parenting, and cycles.
- Keep valid root-level sites. Do not require a single stored root when the native model permits a forest.
- Reject deletion of an occupied node unless the operator supplies a valid reassignment.
- A branch move retains all descendant circuits and their plan rates.
- A rename preserves identity and circuit parent binding.
- Discovery refreshes preserve approved relationships and expose conflicts.
- A stale draft cannot overwrite a newer approved revision.
- A failed export or native rejection never appears as applied.
- Rollback restores the prior revision through the normal publication and verification path.
- Tests use isolated files. Live validation needs a bounded change and native readback.

The current native convergence investigation remains separate.
Until that issue is resolved, `ready: true` must not substitute for applied-state proof.

## Next work

Choose whether the product needs its own topology editor or the native LibreQoS editor specifically.
For its own editor, specify the persistent catalog and revision flow first.
For the native editor, prototype an isolated ingress extension before any UI work.
No supported external editor-metadata ingress was established in this research.

[canonical]: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqos_config/src/topology_canonical_state.rs
[editor]: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqos_config/src/topology_editor_state.rs
[effective]: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqos_topology/src/effective_state.rs
[handler]: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqosd/src/node_manager/local_api/topology_manager.rs
[overrides]: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqos_overrides/src/topology_overrides.rs
[bundle]: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqos_topology_compile/src/bundle.rs
[native]: https://github.com/LibreQoE/LibreQoS/blob/9ebe29bc9d49013aff9b0544de3c6f6fc6ba2f1d/src/rust/lqos_topology_compile/src/libreqos_network.rs
