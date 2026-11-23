---
type: review
created: 2026-11-23
status: proposed
---

# Topology Studio: design review and implementation guide

## Result and scope

Keep the proposed configuration-driven canvas. Extend the current Leptos UI and topology engine in small steps.
The strongest additions are contextual help, source-rule explanations, WAN evidence, and clear preview states.
Do not replace the current compiler, identity registry, edit receipts, or publisher.

This review covers the supplied DOCX and all five figures. It compares them with local source at `c89d5d7` and the current worktree.
Local source includes uncommitted inventory changes. This review does not check a deployed service.
The visual and theme requirements below record the owner direction. Other improvements remain proposals. Accepted project decisions keep their authority.

## Detailed router-source design

The [Router Source Configuration design](ROUTER_SOURCE_CONFIGURATION.md) records the latest owner requirements and detailed preview contract.
It includes a selectable [SVG design viewer](prototypes/topology-studio-v2/index.html), source examples, credentials, WAN pins, and the load action.
The later clarification defines **Load saved topology** as a read-only display of current saved configuration.
This action is distinct from layout restoration and historical policy restoration.
The requested test before save needs an explicit draft-test operation. The current disabled-router contract remains separate.

## Source package

- [Original ZIP](images/LQoSync_Complete_Design_and_SVGs.zip)
- [Extracted document text](references/topology-studio/SOURCE.md)
- [Original DOCX](references/topology-studio/LQoSync_Topology_Manager_Complete_Design.docx)
- [Topology Studio SVG](references/topology-studio/lqosync_topology_studio.svg)
- [Configuration workflow SVG](references/topology-studio/lqosync_configuration_workflow.svg)
- [Discovery pipeline SVG](references/topology-studio/lqosync_discovery_pipeline.svg)
- [Layer model SVG](references/topology-studio/lqosync_sankey_layers.svg)
- [Information guide SVG](references/topology-studio/lqosync_information_guide.svg)
- [Checksums](references/topology-studio/SHA256SUMS)

The SVG and DOCX copies keep the exact archive bytes. The Markdown extraction preserves text and table cells.
The DOCX keeps embedded figures. The SVGs remain editable design references.

## Owner requirement: reference design with dynamic themes

Keep the composition of `lqosync_topology_studio.svg` as the visual target.
Preserve the canvas grid, rounded cards, curved flow bands, floating tools, and spacious configuration panel.
Keep the WAN, router, source-group, and subscriber hierarchy readable.
All action controls use icons and stay inside the infinite canvas viewport.
Keep the accepted topology cards. Remove external viewer controls. Open configuration and help panels within the canvas bounds.
Adapt the composition for narrow screens without changing its visual identity.
Move the mockup configuration workflow strip into the information panel. Show it only after the operator selects the information icon.
Reserve the main canvas for topology cards and flow relationships.

Use the current application theme control. Support **Light**, **Dark**, and **System** under [THEMING.md](THEMING.md).
System follows browser preference changes while the page stays open. Manual selections keep their current persistence behavior.
Theme changes apply immediately without a reload, topology mutation, or backend job.

Map canvas colors to semantic CSS tokens. Do not copy the SVG hex colors into application components as fixed colors.
Cover the canvas, grid, cards, flow gradients, labels, borders, focus rings, toolbar, drawers, and native controls.
Use the supplied dark palette as a reference. Give the light palette equivalent contrast and the same status meanings.
Preserve draft values, selected cards, layout positions, zoom, and open panels when the theme changes.

Verify both palettes, both System preferences, and preference changes while a configuration panel is open.
Check flow visibility, keyboard focus, status labels, and text contrast in each mode.
The source SVG remains an unchanged reference artifact. The application provides the dynamic theme behavior.

## Extracted product plan

1. Register a router and select its subscriber sources.
2. Configure filters and rules for groups, identity, speed, and parents.

3. Discover WAN candidates and show the evidence for each candidate.

4. Evaluate proposed rules against a cached snapshot.

5. Show the candidate topology and its validation results. Include changes from the current state.

6. Save valid policy through the current service operation.

7. Compile and publish through the current authoritative writer.

8. Show source freshness and publication evidence on the canvas.

9. Provide optional help for the selected field or entity.

The source proposes six phases: guide, router configuration, policy preview, WAN evidence, capacity metrics, and saved-view integration.
The dependency order below moves correctness and preview contracts before new policy controls.

## Current source and remaining gaps

| Area | Evidence in current source | Extension still needed |
| --- | --- | --- |
| Canvas | `crates/web-ui/src/topology_canvas.rs` has modal tools, automatic edits, and layout-only saves | Contextual guide and explicit view restoration semantics |
| Flow lines | The canvas uses subscriber-count widths from service flows | A visible count legend, then optional rate or telemetry modes |
| Router form | `crates/web-ui/src/router_form.rs` saves new routers disabled and tests current enabled routers | A deliberate decision before any test-before-enable workflow |
| Inventory | Uncommitted `topology_inventory.rs` and `topology_inventory_model.rs` add source states | Reuse this work after its owner completes it |
| Contracts | `crates/web-contract/src/topology.rs` has snapshot IDs, cached preview, revisions, and edit requests | Typed additions for policy evaluation and provenance where required |
| WAN discovery | `src/topology_manager/wan.rs` reads interfaces and active IPv4 default routes | Route-read errors, route-table evidence, persistent candidate decisions |
| Source rules | `source_policy.rs`, `source_rate.rs`, and `rates.rs` already own relevant logic | Bounded filter/group rules that reuse these owners |
| Publication | Existing integration documents define one owner, exact receipts, and recovery gates | Evidence for each new policy path and output change |

These findings establish source presence, not complete runtime behavior.
The graph supplied leads. Direct file reads supplied the findings above.
The handoff contains older gaps. Recheck each gap against current source before you assign implementation work.

## Corrections to the supplied design

### 1. Keep preview optional for ordinary edits

[ADR-0014](../../brain/20-decisions/ADR-0014-canvas-auto-apply-and-writer-retirement.md) requires automatic valid edits.
The supplied text preserves this decision, but Figure 4 ends with "Approve or revise".
Use "Inspect result or edit draft" in a revised figure. Do not add a mandatory approval step for ordinary edits.
Keep first activation and ownership transfer under their current release gates.

An unsaved rule draft can remain local while the operator requests a read-only preview.
A valid accepted edit still follows the current automatic apply path.

### 2. Separate view restoration from policy restoration

Use **Restore canvas view** for positions, zoom, collapsed groups, and visible layers.
This operation must not change policy, ownership, output files, or RouterOS.

Use **Restore policy revision** only for a separately implemented policy operation.
It must validate against current sources and submit a new revision through the current edit path.
An old saved view must never replay an old policy as a side effect.

### 3. Preserve the disabled-router contract

The current form says that a disabled router receives no connection. This rule also blocks a test.
The source proposes a connection test before explicit enable. These workflows conflict.

For the first slice, preserve the current registration and test workflow.
If pre-enable tests become a requirement, define an explicit one-shot test exception before implementation.
It must not enable periodic discovery or change the current disable guarantee silently.
Do not add a second registration wizard in the topology canvas.

### 4. Distinguish failed route reads from empty routes

In `wan.rs`, `session.print(path, fields).await.unwrap_or_default()` discards the route-read error.
A successful interface read can conceal a failed route read.

Return route status separately from route rows. Keep the last valid evidence with its collection time.
A failed read must show **Unknown** or **Stale**, rather than assert that no default route exists.
Do not use failed or partial evidence to remove a WAN pin or subscriber assignment.

### 5. Keep the three topology meanings visible

Show physical/logical inventory, effective shaping parents, and routing assignments as distinct layers.
A canvas connector does not prove that MikroTik routing follows that connector.
The [current routing design](../superpowers/specs/2026-11-23-canvas-flows-and-routing-balance-design.md) already separates failover routing from shaping placement.
Extend that design. Do not create another allocator inside the policy evaluator or browser.

### 6. Correct visual ambiguities

The layer figure says "three separate layers" but lists five numbered items.
Keep three layers. Describe candidate preview as a display mode and publication as a service action.

The studio image combines source and capacity flow. Give each metric its own name, unit, and legend.
Use subscriber counts first, because the current service already provides them.
Measured Mbps needs attributable telemetry and a collection time. Provisioned Mbps is a separate metric.

Keep one visible card for each logical node across multiple WAN parents.
The service owns card IDs and merged counts. The browser must not infer export identity from labels.

## Proposed implementation contracts

Extend current types when they already carry the required data. The names below describe responsibilities, not mandatory new structs.

| Responsibility | Minimum contract |
| --- | --- |
| Source policy | Stable policy ID, schema version, router/source scope, enabled state, bounded typed rules, deterministic precedence |
| Discovery evidence | Snapshot ID, per-table result, collection time, completeness, source scope, redacted records |
| Candidate result | Base policy revision, snapshot IDs, policy digest, proposed nodes and circuits, provenance, validation report |
| WAN evidence | Router/interface identity, route table, matching route, observation time, status, saved pin or ignore choice |
| Validation issue | Stable rule code, severity, entity ID, explanation, corrective action |
| Canvas view | Independent layout revision, positions, zoom, collapsed groups, selected layer and metric |
| Apply status | Existing operation identity, saved revision, output receipt, import result, dataplane evidence |

Keep snapshot IDs distinct from collection times and policy revisions.
Record completeness per table, not only per router.
A partial join must identify the missing input table.
Treat a snapshot bundle as an observation window. Separate RouterOS reads do not prove a simultaneous device snapshot.

Preview must not allocate durable IDs, advance removal grace, change the registry, save routing assignments, or claim the output writer.
Use temporary preview references for proposed entities until the current service commits durable identities.
A preview request can create a bounded private cache. It cannot trigger RouterOS writes or live output writes.

On policy commit, check the expected revision and current eligibility again.
If source data changed, recompute validation against the current admissible snapshot.
Do not publish the preview artifact as though it were a current compiler result.

## Source-rule design improvements

Start with typed presets for PPP profile, DHCP server, and the current Hotspot grouping.
Limit fields, operators, group keys, rule count, and result size.
Do not expose arbitrary RouterOS command paths, scripts, or writes through a text query box.
Defer regex and unrestricted joins until concrete rules require them and execution limits exist.

Use deterministic rule priority. Reject equal-priority ownership conflicts unless an explicit tie rule resolves them.
Keep stable identity independent of an IP address, display name, group label, or WAN assignment.
Reuse current identity and grace rules. A new evaluator must not create a second subscriber registry.

Show the winning speed rule and its inputs for each circuit.
Reuse the current speed precedence before any deliberate migration. Resolve upload and download independently.
A profile plan rate does not automatically define an aggregate node ceiling.

Keep automatic capacity factors, fixed capacity, minimums, WAN backstops, and shared budgets distinct.
Explain the sample `21 × 15 Mbps × 0.5 = 157.5 Mbps` as a planning calculation.
It is not observed traffic or a universal capacity guarantee.

Use distinct labels for these operations:

- **Disable router:** stop connections under the current router contract.
- **Exclude router from export tree:** retain source collection and promote children under the current export rules.
- **Exclude source from shaping:** remove that source from effective shaping under the current exclusion rules.

Do not rename source exclusion to **Do not read** unless collection actually stops at that scope.
Explicit exclusion can authorize a policy change. Failed discovery cannot authorize absence-based removal.

## Optional operator guide

The information icon opens help without a mutation request.
The configuration workflow stays hidden until the operator clicks or activates the information icon with the keyboard.
Open the workflow in the help panel. Do not show a permanent workflow strip on the main canvas.
The default panel shows the configuration steps. **About this setting** provides optional context for the selected entity.
Closing the panel returns to the same canvas state. The panel follows the current application theme.

| Topic | Question the guide answers |
| --- | --- |
| Router | Why is it disabled, and when does LQoSync connect? |
| Source rule | Which records match, and which rule wins? |
| Identity | Why does the circuit keep its ID after an IP or group change? |
| Speed | Which input supplies each directional rate? |
| WAN | Which route suggests this interface, and what does a pin change? |
| Capacity | Is this a subscriber plan, aggregate target, fixed limit, or minimum? |
| Preview | Which snapshot and policy revision produced this candidate? |
| Apply | Is the edit saved, written, imported, or accepted by the dataplane? |
| Validation | What caused this issue, and which setting corrects it? |

Return keyboard focus to the control that opened help when help closes.
Keep the draft, selected node, viewport, and scroll position.
On narrow screens, use a full-height drawer. Provide a list equivalent for graph relationships.
Use explicit SVG icons instead of symbols that need unavailable fonts from the mockup.
Keep Light, Dark, reduced-motion, and keyboard behavior consistent with the current UI.

## Implementation order and proof

| Phase | Scope and likely owners | Required result before the next phase |
| --- | --- | --- |
| 0. Reconcile baseline | Current inventory owner, canvas, contract, current routing plan | Record current commit and worktree. Resolve duplicate scope. Identify the actual current schema and test state. |
| 1. Optional help | `topology_canvas.rs`, small help-content module, `style.css`, browser check | Workflow appears only through the information icon. Help closes with focus return, preserves drafts, and sends no mutation request. |
| 2. WAN evidence | `wan.rs`, reader table fields, web view and contract, inventory | Distinguish empty, failed, partial, and stale reads. Cover route tables and retain saved pins through a read failure. |
| 3. Preview evaluator | Existing source policy/rate modules, observation and preview, web receipts | Evaluate typed drafts without durable side effects. Report scope, revisions, provenance, and deterministic conflicts. |
| 4. Policy editor | Existing router/source forms, service commands, contract, canvas modal | Preview remains optional. Valid edits reuse revision checks and retry receipts. Drafts survive validation errors. |
| 5. View and metrics | Existing layout path, canvas model, web view, telemetry only if available | Restore view without policy changes. Count legend and aggregation are correct. Unknown telemetry stays unknown. |
| 6. Release evidence | Existing compiler, publisher, activation and recovery checks | Source outage, restart, migration, matching output pair, exact apply evidence, and rollback checks pass. |

Do not add read/write access-profile infrastructure to Phase 1.
Keep current server-side credentials. Add scoped write profiles only when a specific supported write operation needs them.
Do not combine this roadmap with an unrestricted RouterOS automation engine.

For any schema change, add a migration and test it against the current schema.
Record the compatible binary and backup procedure. Do not copy an old schema number from the supplied design.

## Focused acceptance scenarios

1. A route permission failure leaves the last WAN evidence visible with a stale/error state.

2. A failed PPP table read cannot make its current circuits disappear.

3. An explicitly excluded source changes shaping only through its validated policy operation.

4. Repeated preview produces the same result and leaves durable state and live outputs unchanged.

5. A stale policy revision fails cleanly. A retry of the same accepted operation does not duplicate the edit.

6. A DHCP address change or group move preserves circuit identity under current registry rules.

7. Two WAN parents produce one visible source card and no duplicate effective circuit ownership.

8. A fixed multi-WAN capacity preserves the total budget under current capacity rules.

9. A restored view changes no policy or output hashes.

10. An unknown or stale metric displays no invented throughput.

11. A 5,000-subscriber fixture keeps the summary bounded and details paginated or collapsed.

12. A publication failure retains correct ownership and exposes the exact failed operation.

Run targeted Rust tests for each backend change and the affected browser checks for each UI change.
Before release, run the required workspace and frontend gates in `tools/frontend/check-all.sh` with the documented fixture environment.
These are future implementation checks. This review does not claim that they passed.

## Verification of this review

This review covered the complete DOCX text, tables, and all five embedded figures.
Archive copies have checksum evidence. Local Markdown links and extracted XML passed separate checks.
No application code or production service changed for this review.
No deployment, RouterOS operation, live publisher test, or browser application test occurred.

The heuristic prose checker reported style findings, mainly technical terms and noun groups. It does not establish certified compliance.
