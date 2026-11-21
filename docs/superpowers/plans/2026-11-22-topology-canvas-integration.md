# Topology canvas integration implementation plan

> For agentic workers: use superpowers:executing-plans to implement this plan task by task. Delegation is not required.

**Goal:** Connect the canvas to the topology service and prove its publisher before retiring the legacy generation path.

**Architecture:** Keep the current Rust/Leptos frontend and service-owned catalog, resolver, compiler, and publisher.
Add automatic edits and a recoverable publication protocol where evidence shows a gap.
Transfer writer ownership only after shadow and integrity checks pass.

**Tech stack:** Rust, Leptos, Wasm, the existing HTTP gateway, service IPC, and LibreQoS v3 pair ingress.

**Spec:** [Canvas design](../../frontend/TOPOLOGY_CANVAS_DESIGN.md) and [integration map](../../frontend/TOPOLOGY_CANVAS_INTEGRATION.md).

Status: in progress. Implementation baseline: `2aa4fb0` (2026-11-22, Claude Code). Baseline manifest: [evidence](../../frontend/evidence/topology-canvas-integration/README.md).

## Global constraints

- Ordinary valid edits auto-save and auto-apply. Preview is optional and cannot gate publication.
- Keep one live producer, exclusive service ownership, and actor-bound durable receipts.
- Keep deployed state and previous artifacts unchanged during development and shadow checks.
- Preserve unrelated Routers edits. Use focused paths when committing, if publication is requested.
- Keep source and node identities stable. Preserve protected static records and current endpoint freshness rules.
- Keep full Google Maps decimal text and numeric coordinate export without intentional rounding.
- Do not represent a shared physical bandwidth limit as independent duplicated limits.
- Do not add MikroTik routing writes without a separate scope decision.
- No task is complete from a mock, passing serialization, or a successful write alone.

## Review focus

- Lost replies and duplicate requests must not repeat a policy edit or publish an older revision. Task 2 owns the tests.
- A crash between file replacements must not admit a mixed pair. Task 5 owns the tests.
- Source outages and protected static records must not become accidental removals. Tasks 3 and 6 own the tests.
- Old clients and new registry fields must not corrupt rollout or rollback. Tasks 1, 2, and 7 own the tests.
- A successful previous native ACK must not label a newer candidate Applied. Task 5 owns the tests.

## Dependency order

Task 0 precedes all changes. Task 1 precedes Tasks 2 and 3.
Task 4 consumes Tasks 1–3. Task 5 can harden publication before Task 4 completes.
Task 6 requires Tasks 2–5. Task 7 requires Task 6. Task 8 requires Task 7 evidence.

### Task 0: Freeze the baseline and acceptance manifest

**Files:** create `docs/frontend/evidence/topology-canvas-integration/README.md`; update this plan with the implementation baseline.
**Interface:** input is the current build, ownership, source configuration, and existing policy. Output is the evidence manifest.

- [x] Record source/worktree state, target/source mode, compiler identity, current writer, and deployed binary separately.
- [x] Define supported configurations, backup paths, semantic comparison fields, required live events, and observation window.
- [x] Identify protected static data and existing external writers before any transfer.
- [x] Check the manifest against actual read-only installation evidence. Do not copy fixture names as live facts.

### Task 1: Extend the model and typed canvas contract

**Files:** `src/topology_manager/{model,catalog,layout,edits,store}.rs`, their tests, `crates/web-contract/src/topology.rs`, `src/topology_manager/web_view.rs`.
Create `crates/web-ui/src/topology_canvas.rs` for the focused canvas state/render owner when implementation begins.
**Interface:** add typed node presentation, geographic type, coordinate text, and edge intent keyed by existing stable IDs.
Layout storage has an independent revision and cannot trigger policy publication.

- [ ] Write failing tests for old documents, new field round trips, incompatible versions, unknown IDs, cycles, and atomic coordinate-pair edits.
- [ ] Extend the current schema with explicit version migration and backup recovery. Retain legacy router-coordinate fallback.
- [ ] Keep Site/AP export type separate from `Wan`, `Intermediate`, and `SubscriberGroup` policy roles.
- [ ] Add bounded canvas cache/contract fields. Keep preview and observations read-only.
- [ ] Run the model, registry recovery, and contract tests. Confirm old policies preserve IDs and data.

### Task 2: Add one automatic edit operation

**Files:** `crates/web-contract/src/topology.rs`, `crates/web-gateway/src/{http,broker,topology_jobs}.rs`, `src/api/{dispatch,protocol}.rs`, `src/runner.rs`, `src/topology_manager/{edits,web_receipts}.rs`, and their tests.
**Interface:** proposed `POST /api/v1/topology/edits` and service command `topology.edit.apply` consume `{key, expected_revision, edit}` plus the server-derived actor.
They return durable operation/receipt identity, saved revision, and a pending or failed apply result. Names are new interfaces, not current routes.

- [ ] Write failing tests for unauthorized scope, same-key replay, changed payload, lost reply, stale revision, and late completion.
- [ ] Compose existing edit validation and durable receipt handling into one service-owned automatic mutation.
- [ ] Trigger reconciliation after a saved policy edit without an operator confirm step.
- [ ] Preserve compatibility routes during rollout. Old clients cannot bypass the same validation and ownership rules.
- [ ] Run API, gateway, and receipt tests. Prove retries query the existing receipt after an unknown outcome.

### Task 3: Connect discovery, graph semantics, and coordinates

**Files:** `src/topology_manager/{wan,reader,coordinator,source_policy,normalize,identity,catalog,resolver,aggregate,rebalance,publication}.rs`, their tests, and typed web contracts.
**Interface:** selected discovery yields typed resources; typed edge edits resolve to existing placement instances and one effective subscriber parent.

- [ ] Write failing tests for selective discovery, overlapping PPP selectors, incomplete evidence, and source outages.
- [ ] Reuse collection paths and distinguish PPP profile, PPPoE server, DHCP server, and current subscriber membership.
- [ ] Validate shared WAN eligibility and hierarchy separately. Reject unsupported shared-bottleneck constraints before publication.
- [ ] Export explicit Site/AP type and full numeric coordinates. Test long decimal paste values through saved policy and JSON output.
- [ ] Preserve pin eligibility and pinned demand. Add count mode tests and implementation.
- [ ] Specify WAN quality probe freshness and pin failure behavior before quality mode implementation. Keep unsupported controls unavailable.
- [ ] Run discovery, identity, resolver, rebalance, aggregate, and publication tests with protected static cases.

### Task 4: Port the frontend workflow to the canvas

**Files:** `crates/web-ui/src/{topology,topology_model,topology_forms}.rs`, new `topology_canvas.rs`, scoped styles in `crates/web-ui/style.css`, and `tools/frontend/topology-canvas-check.cjs`.
**Interface:** the canvas consumes typed resources and status, submits Task 2 operations, and persists Task 1 layout.
Use the current Rust component and theme conventions, with Vercel frontend guidance.

- [ ] Add browser cases for empty start, existing policy, source attachment, connectors, modal edits, and automatic apply status.
- [ ] Implement pan/zoom, stable selection, node positioning, and keyboard alternatives without replacing the Rust stack.
- [ ] Implement canvas FABs and accessible Inventory/settings/allocation/location modals.
- [ ] Remove mandatory review/approve controls from the new workflow. Preserve optional preview as inspection only.
- [ ] Keep incomplete fields as drafts. Serialize valid edits and debounce unsent changes without overwriting accepted operations.
- [ ] Test light/dark/system themes, mobile fields, full coordinate paste, Escape/focus return, reconnect, and stale responses.
- [ ] Build Wasm and run native UI/contract and browser checks. Mock evidence alone does not complete this task.

### Task 5: Prove or repair coordinated publication

**Files:** `src/write_receipt.rs`, `src/files.rs`, `src/runner.rs`, `src/runner/topology_cycle.rs`, `src/topology_manager/{publication,compiler,runtime}.rs`, `src/apply_evidence/{monitor,verify}.rs`, and focused recovery tests.
**Interface:** keep `Tracker::write(outputs, meta, now, hub)` as the sole publication boundary unless a proven protocol change requires a typed replacement.
The resulting receipt binds the complete pair to policy/configuration/observation and runtime evidence.

- [ ] Write failure tests at intent save, first replacement, second replacement, receipt save, import request, ACK, and process restart.
- [ ] Prove the actual importer excludes mixed pairs. If not, coordinate a recoverable publisher/reader protocol before live use.
- [ ] Implement recovery before new publication and import. A journal must have bounded, authenticated ownership and digest checks.
- [ ] Cover saved-but-unpublished policy and partially replaced files separately from runtime last-good state.
- [ ] Prove stale ACK, late old receipts, digest drift, and unchanged output cannot falsely mark a new revision Applied.
- [ ] Prove lock loss, unknown ownership, and compiler drift block writes and never enter legacy fallback.
- [ ] Run receipt, compiler, publication, ownership, apply-evidence, and recovery checks. Capture every failure-point result.

### Task 6: Shadow and semantic acceptance

**Files:** new focused `tests/topology_canvas_integration.rs`; evidence under `docs/frontend/evidence/topology-canvas-integration/`; reuse existing fixtures and test helpers.
**Interface:** both candidate generators consume the same frozen authorized observations. Only scratch outputs are permitted.

- [ ] Test PPP, DHCP, mixed sources, multiple devices, reconnects, IPv6, exclusions, and protected static rows.
- [ ] Compare circuit/device identities, endpoints, resolved parents, rates, counts, and shared capacities.
- [ ] Record expected differences for deliberate new policies. Reject unexplained missing or extra records.
- [ ] Exercise real service commands through the gateway, canvas edits, compilation, and the controlled publisher fixture.
- [ ] Confirm the topology path never invokes legacy generation and GET/preview never writes or discovers.
- [ ] Run the focused integration suites and the required workspace checks. Store exact commands and results.

### Task 7: Controlled ownership transfer and rollback proof

**Files:** `docs/operations/TOPOLOGY_LIVE_TEST.md`, `docs/frontend/DEPLOYMENT.md`, and the new evidence manifest. Modify migration/runtime code only if the proven protocol requires it.
**Interface:** admitted complete topology candidate plus exclusive lease yields topology ownership and matching runtime acceptance.

- [ ] Update the live runbook for new schema, pair recovery, reader coordination, backups, and the controlled rollback procedure.
- [ ] Verify all offline gates and resolve external writers before scheduling the live window.
- [ ] In the authorized live window, quiesce the current producer and transfer ownership without overlapping live writers.
- [ ] Verify output hashes, import generation, native ACK, identities, capacities, coordinates, and freshness against the exact write receipt.
- [ ] Exercise the declared edits, no-op cycles, source outage, restart, and rollback scenarios during the observation window.
- [ ] Record rollback state compatibility. Never implement rollback as deletion of the owner marker.
- [ ] Leave legacy code available until the complete evidence bundle passes. No live window is authorized by this plan alone.

### Task 8: Retire the legacy producer after acceptance

**Files:** legacy dispatch/generation branches in `src/runner.rs`, obsolete configuration/CLI branches found by reference search, related tests and operational docs.
**Interface:** supported runtime configurations dispatch to topology only; useful shared serializers and publication infrastructure remain.

- [ ] Confirm Task 7 evidence and the supported-target migration are complete before preparing deletion.
- [ ] Search all legacy-generation callers and configuration readers. Document what remains for adoption and rollback compatibility.
- [ ] Remove the legacy producer in a separate reviewed patch. Do not remove `src/writer/pair.rs` merely because legacy uses it.
- [ ] Test startup, import, topology edits, source collection, receipt recovery, and unsupported old configuration errors without legacy dispatch.
- [ ] Update deployment and recovery docs. Preserve required rollback artifacts for the declared retention window.

## Verification commands

Use focused filters first, then the required project gates after implementation.
These are planned commands, not results from this mapping task.

```sh
cargo test --locked -p lqosync3 topology_manager::
cargo test --locked -p lqosync3 write_receipt
cargo test --locked -p lqosync3 apply_evidence
cargo test --locked -p web-contract -p web-gateway -p web-ui
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo build --locked -p web-ui --target wasm32-unknown-unknown --release
```

Use deployment-specific browser and live commands from the updated runbook.
Do not run the old live procedure unchanged or interpret its historical result as canvas acceptance.
