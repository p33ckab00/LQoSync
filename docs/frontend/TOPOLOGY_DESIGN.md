# Topology Manager web design

Status: proposal for implementation. This task supplies documents and static mockups only.
Date: 2026-11-21, Asia/Shanghai. Inspected source: `76563ee`, branch `lqosync3`.
The deployed state was not inspected. Source availability does not prove production activation.

Read [the connection map](TOPOLOGY_CONNECTIONS.md), [the delivery plan](TOPOLOGY_IMPLEMENTATION_PLAN.md), and [the mockup guide](evidence/topology-manager/README.md).
The [approved TUI layout](../design/tui/TOPOLOGY_MANAGER_LAYOUT.md) supplies the workflow baseline.
The [domain design](../design/TOPOLOGY_MANAGER_IMPLEMENTATION.md) supplies the policy and ownership rules.

## Purpose and boundary

Port the Topology Manager workflow to the Rust web frontend.
Preserve the service as the owner of identity, policy, collection, placement, and publication.
Keep the current TUI available. A browser or web service outage must not stop synchronization.

The design preserves the current industrial console, IBM Plex Sans, semantic colors, and compact operator controls.
It uses one Topology destination after Routers. It does not introduce a second Network destination.
The four views are Builder, Assignments, Policies, and Changes.

The browser edits policy through authenticated gateway and broker operations.
It does not open RouterOS connections, write the registry, or publish LibreQoS files.
It does not run a second Runner.

## Current capability and web scope

`Core` below means the inspected Rust implementation exists.
`Adapter` means a typed and bounded web operation is still required.
`Future` means the inspected service has no complete operation for that control.
Every control also needs the reliability gates in the delivery plan.

| ID | Operator feature | Evidence and boundary | Web delivery |
|---|---|---|---|
| TM-01 | Inspect sources and tree instances | Core: `api_tree`, cached preview, catalog IDs | Adapter, then Builder |
| TM-02 | Refresh observations | Core: `topology.preview.refresh` reads routers | Explicit POST job, never a page-open GET |
| TM-03 | Discover WAN interfaces | Core: `wan.discover` reads configured routers | Explicit discovery job with per-router results |
| TM-04 | Create a WAN node | Core: `wan.create`, endpoint reference and fixed rates | Candidate selection, capacity form, impact review |
| TM-05 | Create intermediate or subscriber-group nodes | Core: `node.create` | Parent picker, name, node type, capacity form |
| TM-06 | Attach a source instance | Core: `instance.add` | Select one parent explicitly and repeat for another instance |
| TM-07 | Move, rename, resize, or remove a node | Core: `node.move`, `node.update`, `node.remove` | Selected-item actions and service proposal |
| TM-08 | Promote or demote a node | Parent changes can use `node.move` | Show the exact new parent and no invented node-conversion API |
| TM-09 | Add a newly discovered source | Core: `topology.sources.candidates`, `source.add` | Fresh candidate, identity review, explicit parent |
| TM-10 | Inspect assignments and identity | Core: preview subscriber identities, endpoints, node, plan, and pin | Paged table and selected-subscriber panel |
| TM-11 | Pin or unpin a subscriber | Core: `subscriber.pin` edit | Online identity for a new pin and stable subscriber ID for an offline unpin |
| TM-12 | Rebalance one source or all sources | Core: `rebalance` edit | Explicit scope and impact review and preserve strict pins |
| TM-13 | Create, edit, delete, or select a local plan | Core: `plan.*`, `subscriber.plan` | Revision-bound forms and no RouterOS profile writes |
| TM-14 | Edit global subscriber speed rules | Core: `speed.defaults` | Four rules, units, resolved examples, legacy-rule warning |
| TM-15 | Edit a source group factor | Core: `group.factor` | Source-level down/up factors and show instance overrides separately |
| TM-16 | Select or clear a fallback group | Core: `fallback.set`, `fallback.clear` | Explicit targets and default rates and preserve refusal while in use |
| TM-17 | Inspect capacity, rounding, and conflicts | Core: aggregate evaluation and preview evidence | Separate calculated/exported values and direct conflict links |
| TM-18 | Inspect publication and operation history | Core: write/apply evidence and no complete topology web history API | New bounded evidence projection and durable web operation records |
| TM-19 | Configure recurring speed schedules | Future: no schedule dispatcher operation | Explain that schedules are unavailable and no working Save control |
| TM-20 | Enroll, replace adoption, activate, or deactivate | Controlled CLI and runtime-owner boundaries | Read prerequisites and writer state and execution stays outside the initial web scope |
| TM-21 | Write WAN routing or DPI application pins | Separate future router-writer work | Link to router capabilities and topology placement does not change routing |
| TM-22 | Edit profile-specific speed overrides, backstops, or formulas | Some domain records exist and no narrow edit in the inspected dispatcher | Read current values and add explicit core operations before web edits |

Shared plans stay shared. PPPoE and IPoE profile links open a filtered Policies view.
Router source settings remain in the Router screen. Topology owns placement, capacities, factors, fallback, and local speed policy.
Do not expose `registry.get` or unrestricted `policy.commit` to the browser to bypass an absent narrow operation.

## Layout and navigation

Use `/topology?tab=builder` as the default entry.
Support `assignments`, `policies`, and `changes` in the `tab` parameter.
A selected node uses its stable catalog ID. A subscriber link uses its stable subscriber ID.
Keep account names, IP addresses, credentials, and proposal tokens out of URLs.

Keep the common application header and its Light / Dark / System selector.
Add a local heading, short purpose text, and compact action toolbar.
Use icon buttons for Refresh and Discover, with accessible names and hover/focus labels.
Keep labels on consequential actions such as Review change and Confirm change.

Keep this evidence strip above all four views:

| Item | Meaning |
|---|---|
| Observation time | Time of the source snapshot, with age and stale state |
| Saved revision | Current registry policy revision |
| Writer and runtime mode | `legacy`, `topology`, or `unknown`, with the separate shadow/apply mode |
| Publication | File outcome and import phase for a specific write ID |
| Native verification | Per-identity acceptance evidence, including waiting or failed identities |

Do not merge these fields into one green Connected badge.
A saved policy can differ from the last applied policy. Show both facts.
An old Applied receipt remains historical when a newer revision exists.
An absent receipt is Unknown, not Applied.

### Builder

At 1440px, use a 230px application sidebar and 24px workspace padding.
Arrange Inventory, Topology tree, and Details in a grid with 16px gaps.
Target widths are 260px, a flexible tree of at least 320px, and 300px details.
At smaller workspace widths, move Details below the tree before any panel becomes unusable.

Inventory shows sources, WAN candidates, source kind, router, freshness, member count, and instance count.
A source and an instance are different records. One source can have several instance IDs.
A selected source offers Add instance and an explicit parent picker.
Absent or disabled sources keep their identity and a reason. They do not appear as zero healthy members.

The tree shows node name, type, calculated/exported capacity, subtree members, and child count.
Details shows direct members. The adapter must name direct and subtree counts separately.

The root label is synthetic. Do not create a registry node merely to display the root.

Selection uses stable IDs. Refresh preserves expansion, selection, focus, and scroll.
Use a semantic nested list with disclosure buttons unless the implementation supplies a complete ARIA tree keyboard model.
Drag-and-drop is optional. Every move must work through a keyboard-accessible parent picker.

Details shows stable ID, source reference, parent, capacity mode, limits, factor origin, and resolved rates.
Expose only capacity modes accepted for the selected node type.
A WAN needs explicit capacity. A root cannot inherit from an absent parent.
Do not display capacity as measured traffic or as proof of routing affinity.
Show source-level factors and instance overrides with different labels.

Move and removal reviews list changed destinations, affected subscribers, new assignments, cleared records, warnings, and conflicts.
A truncated sample must say how many records it omits. Counts come from the service, not from visible rows.

### Assignments

Use a semantic table with Name, Source, WAN/instance, Plan, Pin, and Observation columns.
Use a stable row header and a keyboard-scrollable table region.
Filters cover router, source kind, source, WAN, instance, pin, and conflict.
Search and sort operate across the result set before pagination, not only on the visible page.

The subscriber panel separates stable identity from the current IP and MAC.
Show all identities when one subscriber has several source references.
An offline subscriber has no current endpoint. Do not reuse an old endpoint as current evidence.

Pin, unpin, plan selection, and scoped rebalance use explicit action forms.
A pin concerns topology placement. It is not a RouterOS routing rule.
A rebalance summary explains its scope, strict pins, capacity ratio, and affected counts.
Do not add bulk writes until the service supplies a bounded atomic contract for them.

### Policies

Use sections for Subscriber Speed Policy, Local plans, Source factors, and Fallback group.
Each section owns its forms and results. Do not copy these editors to global Settings.
Read current profile overrides and formulas, but keep unsupported edits unavailable.

Show Download minimum, Upload minimum, Download maximum, and Upload maximum.
Each rule shows its mode, entered value, inheritance where present, and resolved Mbps.
Minimum percentages accept 0.1–100. Maximum percentages accept 1–1000, with at most one decimal.
Fixed Mbps rules accept whole numbers from 1 to 1,000,000.
The service remains authoritative for profile, subscriber, and aggregate validation.

`speed.defaults` resets profile speed overrides to inheritance.
The review must disclose replacement of legacy rounded rules before confirmation.
Local plans have base download/upload rates. They do not have independent minimum controls in this scope.
A source group factor accepts 0.001–1 with at most six decimal places.
Keep factor input as an exact decimal string through the web boundary.

Speed Schedules has an unavailable state and a short reason.
A future design must specify weekdays, time windows, overlap, DST, restart behavior, and target ceilings.
The approved future schedule timezone default is `Asia/Manila`, distinct from the documentation timezone.

### Changes

Show current conflicts, proposed changes, web operation records, and publication evidence in separate sections.
Do not present the current TUI Changes view as a complete historical audit log.
That view currently lists revision, rounded exports, and conflicts.

A conflict links to its node, source, policy, or subscriber when the service identifies a target.
Show unknown subjects as safe text. Never convert an error string into an unsafe link.
Show the actor, base revision, saved revision, operation state, and timestamps for web operations.
Local CLI/TUI changes need a core audit source before the UI can claim complete actor history.

Changes is optional inspection. It is not a second mandatory Apply screen.
An edit-specific review confirms one operation. A valid saved edit enters the service's current publication path automatically.
A legacy writer can save policy without publication of that policy. Show the activation boundary explicitly.

## Forms and responsive behavior

Use a right-side details panel for inspection and a native modal for bounded edit/review tasks.
A complex long form can use a dedicated local panel rather than a cramped modal.
Open production modals with `showModal()`. Provide a title, Cancel, Close, safe initial focus, and focus return.
Preserve field values after validation, timeout, conflict, and theme changes.
Do not silently rebase or resubmit a draft after a revision conflict.

At 1024px, use Inventory plus Tree with Details below or in a local panel.
At 390px and 320px, show one workspace panel with Inventory / Tree / Details selectors.
Keep the evidence strip readable with wrapped fields. Keep the four view links horizontally scrollable.
Use one semantic table with a horizontal scroll region before a stacked layout.
If a later stacked layout hides headers, preserve and test each cell/header relationship explicitly.

Minimum readable text is 13px. Use 44px touch controls and a visible focus ring.
Support 200 percent zoom, reduced motion, forced colors, and long labels.
A tooltip must support focus, remain hoverable, and close with Escape.
Do not put required error or availability information only in a tooltip.

## State and quality contract

| ID | Required proof before release |
|---|---|
| TQ-01 | GET, route entry, polling, theme changes, and browser Back cause no router I/O or mutation |
| TQ-02 | Every row, node, preview, and action belongs to the declared revision and observation snapshot |
| TQ-03 | Stale, partial, empty, unavailable, forbidden, disabled, and not-enrolled states remain distinct |
| TQ-04 | Repeated clicks, reconnects, reloads, and request retries cannot duplicate a mutation |
| TQ-05 | An expired, replaced, foreign-session, or stale proposal cannot commit |
| TQ-06 | A lost response produces an explicit unknown result until authoritative evidence resolves it |
| TQ-07 | Disk-full, process crash, and restart tests preserve policy ownership and the last good pair |
| TQ-08 | Source, identity, capacity, hierarchy, static-ownership, and pin validation remain service-owned |
| TQ-09 | Saved, written, imported, waiting, applied, failed, and superseded states never substitute for each other |
| TQ-10 | Sessions, CSRF, Origin, authorization, broker UID, and operation allowlists protect every boundary |
| TQ-11 | No credentials, registry files, arbitrary paths, raw service operation names, or unsafe errors reach the browser |
| TQ-12 | Keyboard, focus retention, native dialog behavior, contrast, zoom, themes, and narrow layouts pass browser checks |
| TQ-13 | Pagination, sorting, limits, partial responses, and large-data tests cannot omit an actionable conflict silently |
| TQ-14 | Web shutdown does not stop the core loop and failed topology writes never select a second writer |
| TQ-15 | Core, contract, broker, gateway, browser, and publication checks supply release evidence against one source revision |

These are acceptance requirements. The mockups do not prove runtime reliability or accessibility compliance.
See [the connection map](TOPOLOGY_CONNECTIONS.md) for required service extensions and failure behavior.
