---
type: design
created: 2026-11-23
status: proposed-implementation
---

# Router Source Configuration and read-only preview

## Owner intent and source baseline

The router is the entry point for source policies. Each policy selects what LQoSync reads and how records become shaping nodes.
Use one workflow for PPP/PPPoE, DHCP/IPoE, Hotspot, and WAN.
Keep the supplied Topology Studio composition, dynamic themes, and configuration workflow behind the information icon.

The owner supplied these requirements in this conversation. The new ChatGPT links did not expose their message content.
The earlier ZIP remains the source reference. This document adds the detailed behavior requested by the owner.

Local `HEAD` and remote `refs/heads/lqosync3` both resolved to `2b54a9a74554c51a189c6761f59273833d823f12` during this review.
The GitHub connector also returned the branch reader source. These checks do not establish the deployed binary or production state.

## Visual references

- [Interactive design viewer](prototypes/topology-studio-v2/index.html)
- [Saved Sankey canvas](prototypes/topology-studio-v2/01-sankey-canvas.svg)
- [Router configuration SVG](prototypes/topology-studio-v2/02-router-source-configuration.svg)
- [Read-only preview SVG](prototypes/topology-studio-v2/03-read-only-preview.svg)

The viewer switches between the canvas, router configuration, and preview.
Use the source icons for PPP, DHCP/IPoE, Hotspot, WAN, or Access to inspect the policy panels.
Light, Dark, and System change the SVG tokens. The information button opens the workflow only on request.
The viewer uses synthetic data. It has no backend, router connection, credential store, or real save operation.
The SVGs remain editable vector sources. Standalone System mode follows the browser preference.

## Owner requirement: icon tools inside the canvas

The owner accepts the topology cards in the supplied v2 reference.
Keep their geometry, content, and Sankey placement.
Use icon-only action controls with accessible names and tooltips on hover or keyboard focus.
Do not place a toolbar, action button, scene selector, theme selector, or export action outside the infinite canvas.

Place the main tool dock inside the canvas viewport. Place zoom/fit and appearance tools in separate compact docks inside that viewport.
Keep these docks fixed within the viewport while the topology world pans or zooms.
Open router settings, policy actions, help, and detail panels within the canvas bounds.
Clamp panels to the available viewport and scroll their content when required.
The workflow appears only through the information icon.

The corrected viewer removes its external header, scene tabs, source dropdown, and footer controls.
Source icons within the router panel select PPP, DHCP/IPoE, Hotspot, WAN, and access configuration.
Theme icons inside the canvas select Light, Dark, or System. The export icon downloads the selected SVG.
Text remains available for field labels, node data, and explanations. Action controls use symbols.

## Canvas and configuration entry points

| Control | Behavior |
| --- | --- |
| Canvas settings cog | Opens router inventory, Add Router, saved configuration, and source policy entry points |
| Router card cog | Opens configuration for that router, including access, sources, export-tree exclusion, and discovery state |
| Source group cog | Opens group plan mapping, aggregate capacity, directional factors, minimums, allocation, and parent rules |
| Preview | Evaluates the unsaved draft against a selected admissible snapshot and overlays the candidate on the same canvas |
| Load saved topology | Displays the current saved configuration as a topology, with its saved revision and current apply status |
| Information icon | Opens the configuration workflow and contextual explanations |

The main canvas has no permanent workflow strip.
A desktop configuration modal can use a spacious side panel so the canvas remains visible.
A narrow screen uses a full-height panel. Retain keyboard focus, drafts, and the viewport.

## Add Router and connection tests

The form includes name, host, port, transport, TLS verification, and credential references.
An explicit **Test draft connection** action performs one read-only connection and tests the selected required tables.
It neither saves registration nor enables periodic collection.
A test result belongs to the exact draft connection and credential revision. A changed draft invalidates the result.
Show transport, TLS, authentication, identity, and table-permission results separately.

Saving registration keeps the router disabled until the operator explicitly enables it.
After registration, its source configuration remains available from the inventory and router card.

The current application tests registered enabled routers. It does not implement this new draft-test contract.
Keep the existing guarantee that a registered disabled router receives no background reads or writes.
Do not invoke the old test route against a disabled router as an implicit exception.
Implement the requested draft test as a separate explicit operation with bounded time and access checks.

## Access and operation scope

Support separate read/write credential references or one shared reference, as the owner requested.
A shared account with full RouterOS permission is allowed. The selected application operations still determine what LQoSync can invoke.
A read policy cannot invoke writes merely because its credential permits them.

Keep secrets in the service credential store. Return opaque references, masked state, and test status to the browser.
An access profile identifies the credential, transport requirements, and supported operation scope.
Policies reference access profiles instead of copying passwords.

A write policy must name a supported operation, target scope, and explicit enabled state.
A blank write policy does nothing. Do not expose arbitrary scripts or RouterOS command text.
Reuse the existing authorized WAN-list writer for supported routing operations.
Do not add profile edits, subscriber changes, or other router writes without their own implementation and checks.
Preview never invokes a write policy, including a policy that is already enabled.

## One source-policy workflow

Each router can have multiple named policies. Each policy contains:

1. Source kind, enabled state, priority, and read access reference.
2. Required tables and supported joins.
3. Match fields, typed operators, values, and AND/OR groups.
4. Node behavior: group by a selected field, use a fixed group, or attach to an existing compatible node.
5. Stable identity scope, duplicate rules, and the current absence/grace policy.
6. Subscriber speed rules and conflict behavior.
7. Parent mapping and group capacity rules.
8. Optional supported write operations, separate from discovery.

Use a guided query builder with known fields and table permissions.
A readable query summary can show `status = bound AND server = dhcp-vlan3001`.
The query is a typed local rule over supported observations, not arbitrary RouterOS code.
Show unavailable fields as unavailable. Do not silently replace a missing field with an empty string.

## Source-specific reads and group rules

| Source | Read and join | Example match | Example generated node |
| --- | --- | --- | --- |
| PPP/PPPoE | `/ppp/active` → `/ppp/secret` by account name → `/ppp/profile` by profile name | Active PPPoE members with profile name `PLAN-15M` | One stable group for that profile |
| DHCP/IPoE | `/ip/dhcp-server/lease` → `/ip/dhcp-server` by server name | Bound leases on `dhcp-vlan3001` | One stable group for that DHCP server |
| Hotspot | `/ip/hotspot/active` → local user → user profile, plus Hotspot server | Eligible active sessions on `hotspot-guest` | One group for that server, unless another supported grouping is selected |
| WAN | `/interface` + `/ip/route`, with optional supported client evidence | An active default route resolves to an interface in the selected table | Candidate WAN source, then saved pin and configured WAN node |

[The MikroTik manual](https://manual.mikrotik.com/docs/authentication-authorization-accounting/ppp-aaa/) distinguishes active sessions, local accounts, and shared profile settings.
Therefore, **active users grouped by profile** is a join. The UI must show that join explicitly.
Do not assume that an active record supplies its profile or a useful comment.
Use a supported active comment when present, or the configured account comment source. Label the actual field in provenance.
RADIUS-only users need an explicit supported authority or fallback. Do not invent a local secret or profile.

DHCP dynamic clients can use LQoSync metadata, a server-to-plan mapping, or client-specific overrides.
The workflow does not require a lease comment or conversion to a static lease.
A lease IP change must follow the current identity rules, rather than create a circuit from the new IP alone.

VLAN, pool, or network grouping needs the required evidence and join first.
Do not offer these fields as working controls when the current reader does not supply them.

## Subscriber speed and node capacity

The owner can map a profile name explicitly to a local speed plan.
A name such as `PLAN-15M` is a selector, not proof that the rate is 15 Mbps.
Enable parsing only with an explicit supported format. Validate its result and units.

The policy editor exposes an ordered precedence list. One proposed preset is:

1. Explicit LQoSync subscriber override.
2. Configured subscriber comment rule with a valid plan token.
3. Exact profile or DHCP-server mapping to a LQoSync speed plan.
4. Supported RouterOS profile `rate-limit`.
5. An explicitly configured fallback plan.

This preset is a proposal. Preserve existing precedence until a deliberate policy migration replaces it.
Show which rule won, the unused alternatives, and any conflicting rates.
The policy defines whether a conflict blocks the candidate or uses its declared precedence.
An unresolved mandatory rate blocks export. A selected fallback produces a visible warning.

The [RouterOS PPP manual](https://manual.mikrotik.com/docs/authentication-authorization-accounting/ppp-aaa/) defines the first profile rate as client upload and the second as download.
Normalize that direction before display or export. Keep exact units and the existing sustained-rate parser.

Configure aggregate capacity separately:

- Use either a fixed node capacity or an explicit sum-of-plans calculation.
- Calculate download and upload independently, with their own factors and minimums.
- Keep current ancestor limits, WAN backstops, and shared capacity rules.
- Split a shared fixed capacity without creating another copy of the total budget.

For the symmetric sample, `21 × 15 Mbps × 0.50 = 157.5 Mbps` is a group target.
It is neither a per-circuit limit nor a throughput measurement.

## WAN evidence and pins

Show the interface, route table, route activity, resolved gateway/interface, interface state, and collection time.
An active default route is evidence. It does not prove that every port or every default-route interface is a physical Internet uplink.
An unresolved recursive route stays unresolved. A failed read stays failed or stale.

Offer **Pin as WAN**, **Ignore candidate**, and deliberate manual selection.
Keep the pin by stable router/interface reference and retain its settings during a failed read.
A WAN source pin classifies an interface. A subscriber WAN pin or routing assignment is a separate operation.
Neither a source pin nor a preview rewrites MikroTik routes.

The current `wan.rs` discards route-read errors with `unwrap_or_default()`.
Replace that loss of evidence before new automatic WAN decisions rely on route absence.

## Detailed read-only discovery and validation

### 1. Prepare a draft

Keep draft policy separate from saved configuration.
Validate rule syntax, supported fields, access scope, and limits before any discovery request.
The draft records the base saved policy revision. It cannot select another account as its actor.

### 2. Select or collect evidence

Use an existing fresh snapshot when it contains all required tables.
Offer an explicit bounded refresh through the service when required. Do not refetch on each keystroke.
Record router, table, read result, collection time, completeness, and snapshot ID.
Use one bounded plan per router with explicit concurrency, row, byte, and time limits.
An exceeded limit marks the result partial. A partial result must not authorize removals.

Separate command times form an observation window, not a simultaneous RouterOS transaction.
Check cross-table joins and report records that changed during collection.

### 3. Freeze inputs and evaluate

Bind the candidate to exact snapshot IDs, base revision, and draft digest.
Normalize records, resolve identity references, evaluate filters, form groups, resolve rates, and choose effective parents.
Use current compiler rules for capacities and export eligibility.

Preview must not allocate durable IDs, advance removal grace, claim output ownership, or save routing assignments.
Use temporary references for proposed entities. Keep source data and credentials private and bounded.
A candidate cache is permitted. A registry commit or live output write is not.

### 4. Validate the complete candidate

| Severity | Examples | Result |
| --- | --- | --- |
| Blocker | Invalid query field, missing mandatory join, cycle, invalid parent, unresolved mandatory rate, duplicate effective ownership, unsupported export shape | Candidate cannot publish through the normal writer |
| Blocker | IP ownership collision that the selected exporter cannot represent, invalid total/shared capacity | Keep the affected candidate invalid and show the records |
| Warning | Selected fallback plan, stale optional evidence, uncertain WAN candidate, configured but inactive account | Show the explanation and the affected entities |
| Information | Inherited value, unchanged group, local DHCP metadata, tree-excluded router | Explain the result without a new approval gate |

Failed or partial mandatory subscriber evidence blocks that evaluation scope.
Do not treat an empty or failed join as proof that subscribers disappeared.
A complete successful read can affect absence handling only through the existing live reconciliation rules, never through preview.

### 5. Display the candidate on the same canvas

Show **Saved topology** and **Read-only candidate** as distinct display modes.
Keep positions and selection stable where identities match.
Use dashed borders and add/change/remove markers for the diff.
Use flow width only for the selected metric, not for validation severity.

A details table includes source record, matched policy, stable identity reference, group, effective parent, directional rate, winning speed source, and issue codes.
Include the router, endpoint, snapshot, and field provenance.
Show additions, removals, changed parents, changed rates, and unchanged counts.
Paginate subscriber details. Do not create thousands of visible cards on initial display.

### 6. Edit, save, or return to saved topology

The operator can revise the draft or save valid configuration. Preview remains optional for ordinary edits.
A save uses the current authenticated edit operation, expected revision, and retry receipt.
The service checks current evidence and validation again. A stale candidate is not a publish authorization.

**Load saved topology** displays the current saved policy as the effective canvas view.
It does not save the draft, activate ownership, publish files, or claim dataplane acceptance.
Preserve the draft separately. Do not silently discard it when the operator switches views.
A historical policy restore is a separate validated mutation, not this load action.

After a valid accepted edit, normal automatic apply continues under the current service ownership rules.
Show saved, compiled, written, imported, and dataplane-accepted states separately.
A failed apply cannot display the candidate as live merely because its policy saved successfully.

## Sankey semantics inside the infinite canvas

The topology graph remains editable cards with curved flow bands.
Keep one visible card for each logical router or source across multiple WAN instances.
The backend supplies merge IDs, effective membership, and aggregate counts.
Each exported circuit retains one effective shaping parent, even when its logical source has several WAN parents.

Subscriber count is the first metric. Use a common count scale and a separate invisible hit target for pointer access.
The reference sample conserves `26 + 15 = 21 + 12 + 8 = 41` circuits.
Color identifies the upstream WAN in the sample. Labels remain available without color.
The current 1.5–10 px flow scaling is an existing implementation detail. Proportional bands in this reference are a proposed visual refinement.

Add provisioned rate and measured traffic only as distinct supported metrics with units and freshness.
Keep shaping hierarchy, routing assignments, and physical inventory distinguishable.
A connector alone does not prove that MikroTik traffic follows that link.

Router cards appear from configured sources and current eligible observations, with the current grace rules.
A registered router with no eligible members remains reachable in inventory.
**Exclude router from export tree** keeps the canvas card and source reads. Export promotes children under the existing rules.
This setting is separate from disabling the router and excluding a source from shaping.

## Implementation map and acceptance

| Slice | Reuse | Required proof |
| --- | --- | --- |
| Router/source modal | Current router inventory, forms, canvas dialogs, theme tokens | Context, focus, draft retention, all three themes, no permanent workflow strip |
| Draft connection test | Router transport and authentication service | Explicit one-shot read, bounded scope, no save/enable, current disabled-router behavior preserved |
| Policy schema/editor | `source_policy.rs`, `source_rate.rs`, `rates.rs`, typed contracts and web receipts | Bounded deterministic rules, explicit precedence, no alternate identity registry |
| Snapshot evaluator | `reader.rs`, `normalize.rs`, observation and preview | Per-table status, missing joins, stable identity, no durable preview side effects |
| WAN candidates | `wan.rs`, inventory, current WAN/routing modules | Failed versus empty reads, route tables, pin retention, no inferred port role |
| Candidate overlay/load | Current canvas, service web view and compiler | Candidate/saved/applied separation, correct diff, read-only load, preserved draft |
| Live integration | Existing service publisher and ownership | Exact output-pair evidence, runtime acceptance, restart recovery, migration and rollback checks |

Test PPP joins with missing secrets/profiles and RADIUS-only sessions.
Test DHCP address changes, dynamic metadata, and explicit exclusions.
Test overlapping policy matches, invalid speeds, stale revisions, request retries, failed tables, and partial reads.
Verify that preview leaves registry bytes, grace state, routing state, and live output hashes unchanged.
Use a 5,000-subscriber fixture with bounded summaries and paginated details.
Run the current workspace and browser gates when application implementation changes.
This design task does not implement those service contracts or claim their tests passed.

## Design verification

The [evidence bundle](evidence/topology-studio-v2/README.md) records the local browser checks and screenshots.
Three SVG files passed XML parsing. Local document links and whitespace checks passed.
The corrected viewer keeps controls and dialogs inside the canvas at 1600, 1440, 768, and 390 CSS pixels.
Pan, zoom, fit, themes, panel selection, help focus, and preview/load checks passed.
Card geometry and text match the previous v2 canvas.
The heuristic prose checker reported style findings. This document does not claim certified compliance.
The checks cover this design viewer. No application implementation, deployment, or live router test occurred.
