# Dynamic topology design

Status: approved and implemented. See the implementation plan for verification and execution rulings.
Source baseline: `ac4c922`, branch `lqosyncv3`.

## Intent and confirmed requirements

The operator controls which nodes exist and how those nodes connect.
DHCP, PPP, and Hotspot identify inventory sources. They do not require service-specific layers in the exported tree.
The operator can select available sources, add nodes, remove nodes, promote branches, and demote roots.
Automatic assignment uses only the destinations that the operator selects.
The user confirmed that promotion can create an independent top-level root with its own capacity and a retained WAN/core association.

Supported arrangements include one root with direct subscribers, multiple roots with direct subscribers, arbitrary nested groups, and mixed flat/grouped branches.
A selected subscriber can have its own root when the operator creates and promotes that node.
Mixed service types can share any destination. A service-specific node label does not reserve its destinations.

## Baseline behavior and reference

The reference is `/opt/libreqos/src/updatecsv.py`, read without execution or modification.
Its header and network helpers describe grouped roots, flat roots, unparented CSV rows, promoted groups, and promoted clients.
Use these arrangements as examples. Do not copy its execution, cleanup, rate defaults, or RouterOS behavior.

At the source baseline, v3 already supports independent selections and generic groups.
At that baseline, `model::validate` requires every root to be an explicit WAN and prohibits a WAN below another node.
`assignment::Topology` derives the routing WAN from the topmost exported ancestor.

Assignment targets cannot belong to another source. Pools must keep their original source node as a target.
Source-created branches still receive a service kind. Pool identity still depends on the destination node ID.
These constraints prevent independent roots, shared destinations, and fully independent source policy.

See `v3/src/{model,assignment,selection,sources_web,assignment_web,noc_web,web,core_plan,sessions}.rs`.
The current Graphify map predates these modules' latest behavior. Confirm relationships in the current source.

## Approach

Use an editable tree with independent source and WAN assignments, as requested by the user.
Keep the Rust backend, Maud console, local TOML state, and existing validation/publication boundaries.

Alternative: fixed global topology modes copied from the Python script.
This is smaller but cannot represent mixed structures or operator-selected combinations without more exceptions.
Alternative: keep WAN-only roots and relax service checks.
This handles flat WAN trees but cannot support the independent roots that the user confirmed.

## Separate identities

Maintain three identities: source selection, exported node, and routing WAN.
Each source selection retains a stable ID and its router/service/selector/value identity.
Each exported node retains a stable ID, display name, optional parent, and capacity policy.
Each WAN retains a stable ID, router/interface mapping, directional capacity, and core list/table association.

An assignment policy identifies its selected sources and eligible destination pairs.
Each pair contains an exported node ID and a WAN ID.
The same node can accept several sources or appear with several WANs in a policy.
This allows one flat global root to contain clients from several WANs without inventing a routing WAN from the root's position.

Policies can combine selected services or apply to one selection. A source selection belongs to one policy at a time.
Individual selectors keep precedence over group selectors. Exclusions keep their current effect.

Every normalized subscriber keeps its existing stable UUID.
Its planned assignment records source/policy identity, exported parent, and routing WAN separately.
Core plans consume the recorded WAN. CSV/JSON exports consume the recorded exported parent.
Both views must reference the same captured assignment result.

Builder choice: exact Rust type names, TOML table names, and private schema version encoding.
The implementation plan must define those interfaces before code changes.

## Node operations

Add a custom node under a selected parent, or create it as an independent root.
Allow optional node creation from selected inventory. Default to a generic group and let the operator choose its name.
An attached source does not create another node.
Do not require PPP, DHCP, Hotspot, profile, or server layers.

Promote moves a branch one level upward. Promotion from a direct child of a root makes it an independent root.
An explicit top-level destination can promote a deeper branch directly to a root.
Demote chooses a parent from the existing valid nodes.
Both actions preserve the branch's descendants, node IDs, selections, pins, and WAN assignments.

Reject self-parenting, descendant-parenting, excessive depth, and capacity violations before saving.
A hierarchy edit cannot silently change the assigned WAN.

Remove an unused node directly.
For a referenced node, show its children, selections, eligible destinations, pins, and affected candidate/committed clients as aggregate counts.
Require an explicit replacement node or an explicit exclusion/removal choice for the affected sources.

Splice direct children into a chosen valid parent, or move them to independent roots with explicit capacities.
Do not delete descendants or customer selections implicitly.
Rewrite affected destination pairs and pins only after the operator selects valid replacements.
Do not change committed output during the settings edit. Recompile only from a fresh valid managed collection.

Reject removal if a reference remains unresolved. Keep the prior settings and generation on failure.

For a flat arrangement, remove intermediate branches and choose the remaining root as the replacement destination.
The result must have direct CSV parents and no unused generated service layers in JSON.
Discovery and later collection must not recreate removed nodes.

## Assignment behavior

Allow manual policies and automatic policies. Each policy has an explicit default destination pair and a nonempty eligible set.
The default must belong to the set. No source node must remain eligible merely because it was the original destination.
Validate node/WAN references and unique destination pairs. A target need not have a matching service label or exclusive source owner.

No configured policy may refer to an excluded or removed selection.

Keep the current `online_clients` default and optional `planned_load` metric.
Count mode compares WAN counts first, destination counts second, and stable destination identities last.
Count all eligible continuous clients and manual assignments before placing new sessions.
Several selected sources that share a node or WAN contribute to the same counts.
Manual pins specify an eligible destination pair and override automatic placement.

Continuous sessions keep a compatible destination pair. New sessions and observed reconnects can rebalance.
If the operator removes an active destination, show the affected count and require an explicit replacement before accepting that edit.
Operator-directed replacement can move an active assignment. Automatic reconnect rules must not disguise that change.

Failed, stale, partial, or ambiguous reads cannot establish a disconnect.
Keep DHCP bound-lease semantics and the documented limits of polling.

## Capacity and output

Validate each actual exported ancestor's directional capacity.
Also validate aggregate assigned minimums and configured floors against each routing WAN's directional capacity.
These are separate constraints when several independent roots share one WAN or one root spans several WANs.
Use both constraint sets for effective minimum allocation. Do not multiply a WAN budget by its number of exported roots.
Preserve source-owned maximum rates and the configured minimum policy.

Independent roots can use explicit capacities or computed aggregates with positive factors.
Reject nonfinite values, invalid IDs, missing parents, duplicate subscriber identities/IPs, and unresolved WAN mappings.

Managed subscribers require an exported parent and a valid routing WAN.
Offline fixtures can compile without live WAN/core evidence, as they do today.
Blank-parent CSV rows from the Python `no_parent` mode are outside this change.
Do not add automatic per-client root generation or a new global mode selector.

## Console

Keep the compact NOC layout.
Show discovered sources, the editable tree, and the selected-node inspector.
Provide selected-source and selected-destination controls with names and stable IDs.
Expose Add, Promote, Move/Demote, Remove, and Replace destinations in the relevant inspector.

Keep manual and automatic assignment controls beside their selected destination set.
Label exported roots separately from routing WANs. Show effective WAN associations without implying that position determines routing.
Show candidate, committed, and applied status separately. Keep current form tokens and validation errors.

### Configuration modals and cog controls

Use modal forms for configuration edits throughout the console.
This includes nodes, routers, source selections, WAN mappings, assignment policies, pins, speeds, core mappings, and application settings.
Place a cog button beside each configurable item or section. Label it with the item's name for assistive technology.
Provide a visible tooltip and keyboard focus indicator. A cog opens the relevant configuration form.
Keep explicit action labels for Add, Promote, Move/Demote, and Remove.
Those actions can open the same modal with the appropriate operation selected.

Keep the selected-item inspector compact. Show current values and a cog instead of a permanent edit form.
Each modal shows its target name, title, form, Save/Cancel controls, and a close button.
Opening or closing the modal must not save configuration.
Show validation errors beside their fields and in an accessible summary inside the modal.
Preserve submitted values after a failed save. Keep the modal open so the operator can correct them.
Retain the current server validation, form tokens, ownership checks, and publication gates.

Use the native modal dialog and its top layer where supported. No modal library is required.
The page, sticky header, sidebar, and mobile navigation must remain below the modal backdrop.
The dialog and its controls must remain above the backdrop, outside ancestor clipping and stacking contexts.
Menus and help popovers from the dialog must remain visible above its content and within its interaction boundary.
Render modal-specific errors or notifications inside the active dialog. They must not appear behind its backdrop.
Close an open mobile navigation overlay before opening a dialog.

Keep only one configuration dialog open at a time. Show removal/replacement steps within that dialog instead of stacked dialogs.
Set keyboard focus inside the dialog, contain Tab navigation, and make the background inert.
Allow Escape and the close button to cancel. Restore focus to the invoking control, or a valid nearby control after removal.
Do not discard unsaved values silently when an operator dismisses the form.
Prevent background scroll while the modal is open. Restore the prior scroll state when it closes.
Use a scrollable dialog body that fits the viewport. Keep its title, close control, and action buttons reachable on mobile.
Preserve the current Content Security Policy and use the existing same-origin script for modal behavior.

## Compatibility and failure handling

Read existing settings and committed generations without manual file edits.
Translate legacy service-node bindings into independent source identities without changing subscriber UUIDs.
Translate each legacy WAN root into a WAN identity plus its existing exported root.
Translate old node pools and pins into policies and destination pairs with the same eligible choices and assignment mode.
Preserve initially shared legacy policies when several selectors use the same original source node.

Do not save a migration merely because a GET page loads.
Create a private matching settings backup before the first explicit save of the new schema.

Reuse existing candidate/committed evidence when a destination pair has one unambiguous interpretation.
If legacy evidence is ambiguous, reject the migration and identify the unresolved mapping. Do not infer reconnects or discard pins.
Follow explicit replacement history when adopting legacy destinations. Before the first dynamic publication, reject source edits that would invalidate unresolved legacy evidence.
Keep all-or-nothing settings validation, captured input guards, and generation publication.

Keep candidate observations separate from committed output, per ADR 0001.
No edit bypasses protected manual RouterOS entries or core publication gates.
No router write, LibreQoS apply, dependency update, or live service restart forms part of this implementation.

## Acceptance and review

Prove flat single-root, flat two-root, arbitrary nested, mixed-service, promoted-root, and demoted-root output with synthetic fixtures.
Prove shared destinations and one root with several WANs preserve independent core assignments.
Prove two exported roots that share one WAN cannot exceed its combined minimum budget.
Prove selection, hierarchy, and policy edits preserve subscriber UUIDs and unrelated settings.
Prove node removal rewrites only explicit replacements and rejects unresolved children, pins, or policy references.

Prove legacy settings, pools, pins, generations, and candidate evidence preserve their meaning.
Prove reconnect placement, continuous-session retention, manual overrides, healthy absence, and failed-read behavior.
Run the required Rust checks, targeted browser form checks, and representative 50k-client workloads.
Review connected code for correctness and performance before any commit/push.

Check cog buttons and modal forms on every configuration page at desktop and mobile widths.
Check opening, saving, validation failure, cancellation, unsaved values, Escape, Tab navigation, and focus restoration.
Check modal visibility with the sticky header, mobile sidebar, scrollable tables, menus, and notifications present.
Check small viewports and 200 percent zoom. No backdrop may cover a modal control or permit interaction with the background.

## Approval boundary

This document describes the proposed change. It does not change application behavior or live settings.
After user review, write the implementation plan, implement the approved behavior, verify it, and capture the final result.
