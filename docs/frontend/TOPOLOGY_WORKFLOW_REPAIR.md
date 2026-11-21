# Topology workflow and automatic sync

Date: 2026-11-21 (Asia/Manila).

## Design and workflow

The Builder follows the documented Inventory / Topology tree / Details reference.
The hierarchy uses nested lists, branch controls, a synthetic Network root, and readable parent/source labels.
Search reveals matching nodes and their ancestors. Clear search to restore the previous branch state.
Select a node to add a child, edit, move, or review removal.
Attach an existing source to add another instance beneath an explicit parent.

The service still reviews each policy edit before confirmation.

Inventory starts collapsed on narrow screens. Its Show control keeps source actions accessible.
The Details panel moves below the tree before the desktop columns become too narrow.
The existing theme, navigation, policy rates, publication evidence, and shared plan catalog remain available.

PPPoE and IPoE/DHCP use a source table with selected subscriber details beside it on wide screens.
Search includes account, subscriber ID, IP, and MAC. Filters include router, state, local plan, and verified link.
Clear filters restores the full source view. Reload refreshes both the summary and the record list.

Current endpoints require a fresh snapshot. Retained endpoints have a Past or stale label.
Late responses cannot replace newer filter results.
Background refresh preserves selected records and open policy drafts.

Plan changes still use the shared review/confirmation flow. A DHCP observation alone grants no shaping permission.

## Router reads and WAN discovery

Subscriber collection and WAN interface discovery are separate facts.
A WAN-only router can have no PPPoE or DHCP collector. That does not remove it from WAN discovery.
Discover WAN interfaces reads every enabled router and shows each result separately.
An empty result shows the router and zero interfaces. A failed router keeps its own failure text.

Each new discovery request reads the routers again. It does not reuse an old interface list as a fresh answer.

Opening a route and five-second background refresh read service caches only.
They never open a router connection or run another compiler.
Refresh observations and Discover remain explicit router-read actions.

## Automatic orchestration

The previous regular service cycle never called the existing topology sync path.
The repaired cycle dispatches to the topology compiler when durable runtime ownership selects topology.
It collects the configured sources, evaluates the saved hierarchy/rates, and publishes the admitted pair.
The pair contains `network.json` and `ShapedDevices.csv`.
Confirmed hierarchy changes take effect on the next successful scheduled cycle.
Stable subscriber identities remain stable when placement or current endpoints change.

One Runner owns collection, registry changes, compilation, and publication.
The topology path reuses its collected observations for PPPoE, IPoE, Router, and Topology caches.
It keeps native/bridge diagnostics and correct circuit counts without a legacy State.
A failed batch retains the previous shaping pair and source timestamps, with failed/stale evidence.
It cannot silently fall back to the legacy writer.

## Explicit activation

Enrollment prepares ownership and admission records. It does not activate topology publication.
The new activation command provides the missing explicit transition.
It requires the instance lease, `apply` mode, `v3_pair`, an enrolled policy, and fresh authorized observations.
It also enforces compiler, static-source, source-filter, generation, and revision checks.

Install the repaired core and web artifacts before this operator procedure.
Use the configuration path and service unit of the actual installation.
The following example uses `SERVICE_CONFIG` as a placeholder. Replace it with the real configuration path.

1. Stop the synchronization service so the activation command can get its lease.
2. Preview activation:

   ```sh
   sudo lqosync3 topology activate --config SERVICE_CONFIG
   ```

   Without `--yes`, the command compiles in scratch space. It changes no live pair, registry, or runtime ownership.

3. Review the result. Resolve every admission or source error before confirmation.
4. Confirm activation:

   ```sh
   sudo lqosync3 topology activate --yes --config SERVICE_CONFIG
   ```

   Confirmation publishes through the existing sole writer and durably selects topology ownership.

5. Start the synchronization service.
6. Inspect Writer, saved revision, files/import, and native verification independently in the web UI.

Activation does not stop or start services itself. It refuses while another process holds the instance lease.
The browser cannot bypass activation or publication checks.
This source repair does not activate or deploy changes to the running network.

## Verification and references

See [the repair plan](plans/2026-11-21-topology-workflow-repair.md).
See [browser and core evidence](evidence/topology-workflow/README.md).
The original [Topology design](TOPOLOGY_DESIGN.md) and [subscriber view contract](TOPOLOGY_SUBSCRIBER_VIEWS.md) remain the policy references.

[Vercel frontend-design](https://github.com/vercel-labs/open-agents/blob/main/.agents/skills/frontend-design/SKILL.md) guided controlled density and hierarchy.
[Vercel interface rules](https://github.com/vercel-labs/web-interface-guidelines/blob/main/command.md) guided controls, focus, and request/state handling.
[Google Modern Web Guidance](https://github.com/GoogleChrome/modern-web-guidance) supplied `css-layout`, `responsive-table`, and `accordion` guidance.
The layout keeps DOM order aligned with visual order. Source tables keep semantic headers and keyboard-scroll regions.
Disclosure controls use ordinary nested lists instead of an incomplete ARIA tree.
