# Dynamic topology and session assignments

The operator controls the exported tree. Discovery does not create required service layers.
Sources, exported nodes, and routing WANs have separate identities.
One root can accept direct subscribers from several services or WANs.
Several independent roots can share one routing WAN.

## Configuration dialogs

Use the cog beside an item to open its configuration dialog.
Current values and status remain visible on the page.
Add, Promote, Move/Demote, and Remove retain explicit labels inside their dialogs.

Save submits the existing guarded form. Invalid saves keep the dialog and its entered values open.
Cancel, the close button, and Escape close the dialog without saving.
Unsaved values remain in that dialog until you leave or reload the page.
The dialog contains keyboard focus and scroll. Its backdrop blocks the page and its navigation.
The same-origin script uses native dialogs. Without script support, the guarded forms remain available on the page.

## Convert existing settings

Keep a private backup of the application data directory before conversion.
The automatic settings backup does not back up candidate state or the generation pointer.

1. Read fresh router inventory if committed or candidate assignment evidence already exists.
2. Open Topology.
3. Open the cog for **Enable independent topology**.
4. Save the conversion.

The conversion keeps node IDs, source choices, subscriber UUIDs, WAN IDs, eligible targets, modes, metrics, and manual pins.
It creates independent source policies from the old node bindings and pools.
Existing service branches remain until the operator changes or removes them.
The first explicit save creates private `state/settings-before-dynamic.toml` before it replaces legacy settings.
A GET does not migrate settings or create a backup.

Ambiguous legacy source or WAN ownership blocks conversion. It does not reset assignments or drop pins.
A pre-existing migration backup must match the legacy settings. Preserve a matching private backup before retrying a changed legacy configuration.
The old configuration path remains readable until an explicit conversion.

Before the first valid dynamic publication, source deletion or regrouping can still affect legacy committed evidence.
The server rejects edits that would prevent adoption of that evidence. Publish a valid dynamic generation first, then retry.
Explicit node replacements can use retained replacement history without rewriting the old committed files.

## Build a flat or grouped tree

1. Configure routing WANs on WANs with their router/interface and directional capacities.
2. Add exported nodes in Topology.
3. Choose **Independent root** for a top-level node, or select an existing parent for a branch.
4. Select sources on Sources and attach them to a policy.
5. Select eligible exported node/WAN pairs on Assignments.

A source can use an existing destination directly. Creating an extra branch is optional.
New source-created branches use a generic group label.
PPP profiles/accounts, DHCP servers/MAC identities, and Hotspot profiles/accounts remain available as selectors.
Individual selectors override group selectors. Exclusions skip matching records without deleting router data.

Promote moves a branch one level upward. A direct child of a root becomes an independent root.
Move/Demote selects a different parent, including an independent root position.
These edits preserve node IDs, descendants, sources, pins, and routing WAN choices.
Cycles, unknown parents, invalid capacities, and stale settings revisions block the edit.

For a flat tree, select the root as the destination and remove unwanted intermediate nodes.
A referenced node requires an explicit replacement destination.
Dependent children require an explicit new parent or independent-root choice.
The edit rewrites eligible pairs and pins, and records the authorized transition for existing candidate assignments.
It keeps the assigned WAN. Discovery does not recreate removed nodes.
The previous committed generation remains intact until a fresh valid publication succeeds.

Root capacities can be explicit or computed from subscriber aggregates and positive factors.
Computed children remain capped by their immediate exported parent.
Effective minimums must fit every exported ancestor and each routing WAN's shared budget.
Independent roots do not multiply the capacity of a shared WAN.
A routing WAN definition does not create an extra exported queue automatically.
Source maximum rates remain unchanged.

## Configure sources and eligible pairs

Sources can share a policy and its eligible destinations.
Use the source cog to change its policy or exclusion while preserving its selection ID.
Regrouping requires the new policy to preserve existing destinations.
Use explicit destination replacements on Assignments before changing those destinations.
Clear old policy pins before excluding or regrouping a source selection.

Each policy has a name, a default pair, a nonempty set of eligible pairs, a mode, and a metric.
A pair identifies an exported node and a routing WAN.
The same node can appear with different WANs. Mixed services can share the same pair.
No service-kind match or exclusive source ownership is required.
The former source node need not remain in the eligible set.

Manual mode uses the default pair for new assignments. Compatible previous assignments remain stable.
Manual pins override automatic choices.
Online-count mode chooses the least populated WAN, then destination, then stable target identity.
All current retained and pinned clients contribute before new clients receive destinations.
The optional `planned_load` metric compares projected plan rates with WAN and explicit destination capacities.
It keeps valid historical assignments for inactive clients without counting them as current load.

## Session evidence

Continuous sessions keep compatible assignments.
New sessions and observed reconnects can receive a different eligible pair.
Healthy absence, changed session handles, or reset uptime can establish a reconnect.
Failed, stale, partial, ambiguous, and older reads cannot establish one.
The first count observation preserves compatible existing assignments.

PPP and Hotspot count mode require valid session handles.
DHCP count mode uses bound leases. This does not prove physical device connectivity.
Polling cannot establish a reconnect without observable evidence.
Distinct reads can occur in the same second. Read tickets and snapshot identity establish their order.

## Candidate, committed, and applied state

Private `state/online-assignments.toml` stores candidate assignments and session evidence.
Healthy empty scans and later core/capacity failures can update that evidence without replacing committed output.
Assignments and Validation label candidate and committed views separately.
Core plans consume the planned WAN. CSV and JSON consume the planned exported parent from the same result.
A local candidate is not applied routing or shaping.

Corrupt, oversized, incomplete, or concurrently changed state fails closed.
Candidate files retain the 64 MiB and 250,000-record limits. Use one process per data directory.
Protected manual core entries still block publication. These controls do not adopt or delete them.
No router write, LibreQoS apply, package change, or service restart occurs through these controls.

For rollback, stop the new console and restore its matching settings and candidate-state backups before using an older binary.
Never rewrite a committed generation to repair candidate state.

## Large configuration views

Assignments shows the first 200 matching policies and destinations per policy.
Use the policy and destination filters to find other entries. Source summaries show the first 20 members and the total count.
Removal dialogs show separate committed and stored-candidate subscriber counts. These counts include inactive assignment records.
Missing evidence shows `unavailable`; a missing legacy WAN pair is not treated as zero.
A pin control requires a resolved node/WAN pair. Refresh selected inventory if the committed legacy snapshot cannot supply that pair.

Manual browser checks use the synthetic fixture and an existing Playwright installation.
See [browser checks](tests/browser/README.md). These checks save synthetic settings.
