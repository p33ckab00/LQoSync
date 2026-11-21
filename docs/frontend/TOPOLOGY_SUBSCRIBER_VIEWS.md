# PPPoE and IPoE views in Topology delivery

Status: proposed contracts and static references. Production routes do not exist yet.
Date: 2026-11-21, Asia/Shanghai. Source checkpoint: `cd05d23`.

Deliver these views with the [Topology Manager plan](TOPOLOGY_IMPLEMENTATION_PLAN.md).
The read-only cut includes Builder, Assignments, Policies, PPPoE, and IPoE/DHCP.
The mutation cut uses the same identity, policy, review, and receipt services.

## Navigation and layout

Keep Topology after Routers in the application sidebar.
Add PPPoE and IPoE / DHCP as source-specific destinations.
Keep Builder, Assignments, Policies, and Changes inside Topology.
Use ordinary navigation links with `aria-current="page"` for these destinations.
Do not apply ARIA tab roles to page navigation.

| Proposed route | Content | Shared owner |
|---|---|---|
| `/pppoe?view=users` | PPPoE identity records and session evidence | Subscriber projection |
| `/pppoe?view=plans` | Local plans with PPPoE membership counts | Topology Policies |
| `/ipoe?view=clients` | DHCP client records and lease evidence | Subscriber projection |
| `/ipoe?view=plans` | Local plans with DHCP membership counts | Topology Policies |
| `/topology?tab=assignments&subscriber={id}` | Selected stable subscriber and all verified identities | Topology Assignments |

Router and source filters use stable IDs. Reject invalid route parameters.
Keep account names, MAC addresses, and IP addresses out of URLs.
IPoE / DHCP means the current DHCP-backed source capability.
This design does not claim support for every IPoE discovery or admission method.

Preserve the current console, local IBM Plex Sans, semantic colors, and 44px action targets.
Show the common evidence strip above the source views.
Follow it with view links, source context, filters, a semantic table, and selected-subscriber details.
Keep refresh as a named icon button. A view must not poll a router when it opens.

Show Source settings as a link to the selected Router source section in production.
The static reference shows the destination as text because that Router fixture is outside this package.

At narrow widths, stack the context and filters.
Keep table headers, a stable row header, and a named keyboard-scroll region.
Do not hide identity, state, or evidence to fit the viewport.
Keep page navigation on one locally scrollable row.

## Records and state

| View | Required columns | State labels |
|---|---|---|
| PPPoE Users | Subscriber, scoped account, session state, endpoint, local plan, placement and pin | Online, Offline, Disabled, Stale, Unknown |
| IPoE Clients | Subscriber, scoped client, lease state, endpoint, local plan, placement and pin | Bound, Unbound, Disabled, Stale, Unknown |
| Local plans | Stable plan, down/up Mbps, filtered membership, global unique membership, review action | Shared plan, no separate source catalog |

Show source collection state separately from subscriber state.
An unreachable router or incomplete collection must not turn every subscriber Offline or Unbound.
A disabled collector and a disabled account are different facts.
Label a row's last observation time, completeness, and freshness where necessary.

Only fresh active sessions or bound leases supply current endpoints.
Label retained addresses Past. Do not use them for a new pin.
Show empty, loading, stale, unavailable, denied, and failed states with a reason.
Do not represent missing collection as zero healthy clients.

## Connections and identity integrity

The current observation model is `src/collection.rs`.
The TUI presence rules are in `src/tui/presence.rs`.
The topology identity and policy models are in `src/topology_manager/model.rs`.
The active assignment projection is in `src/topology_manager/preview.rs`.
These sources establish the boundary. They are not a complete browser contract.

`collection::Subscriber` has key, source, name, circuit, IPv4, MAC, and state fields.
The report also carries router observation time, source flags, process ID, and cycle sequence.
It does not supply every possible PPP session or DHCP lease field.
Do not invent duration, server, remote profile, or termination-reason data.
Add service evidence before the UI shows such fields.

Use the same bounded core snapshot for observations, stable identity links, plans, and assignments.
Keep policy revision and observation revision separate.
Validate freshness against the active service process and collection cycle.
Do not expose the raw collection cache through the gateway.
The current active preview does not include every offline account or unbound lease.
T00 must add a bounded safe observation projection and separate saved subscriber records.

A verified identity contains router ID, source ID, source kind, and account reference.
A subscriber can have several verified identities, including PPPoE and DHCP.
Both source views must resolve that subscriber to the same stable ID.
Do not join records by displayed name, current IP, or MAC alone.

An unmatched or ambiguous observation remains visible with its link state.
It has no plan or pin action until the service supplies a verified subscriber link.
An observation ID identifies that read record only. It is not a policy target.

A DHCP lease does not grant permission to shape traffic.
This delivery adds no DHCP admission, PPP account creation, RouterOS profile write, or routing writer.
Router source collection settings remain in Routers.
Topology owns local plans, speed rules, source factors, fallback, and placement.
A topology pin does not write a routing rule.

## Shared actions and review

Open the same subscriber detail and action forms from all three subscriber destinations.
Use a stable subscriber ID and the action's explicit verified identity scope.
The service must define whether each action targets one identity or the shared subscriber policy.
Show that scope before confirmation. Do not let a route filter silently narrow a shared policy edit.

Use the same expected revision, principal-bound proposal, and durable request receipt.
Keep lost replies in Outcome unknown until the core confirms the result.

The source Local plans view filters membership. It does not create a separate plan catalog.
Show both filtered counts and all affected unique subscribers.
A shared plan edit must review other source kinds and aggregate capacity.
Count a subscriber with two identities once in global unique totals.
Identity counts and subscriber counts must have explicit labels.

Keep observed RouterOS profiles separate from local plans.
Do not label a local plan edit as RouterOS profile synchronization.

After a confirmed change, invalidate the affected shared snapshot and query views.
Preserve filters, selection, and table scroll when the record still exists.
A receipt and publication result must agree across PPPoE, IPoE, Assignments, and Changes.
Keep saved policy, file/import outcome, and native verification separate.

## Delivery and proof

| Task | Included subscriber work |
|---|---|
| T00 | Safe observation projection, verified identity links, unmatched records, source health, unique counts |
| T01 | Bounded subscriber and plan membership reads with common snapshot cursors |
| T03 | PPPoE Users, IPoE Clients, both Local plans views, and shared subscriber details |
| T05 | Common jobs, review ownership, and outcome recovery |
| T08 | Shared plan and pin forms, explicit scope, cross-source impact, and snapshot invalidation |
| T10 | Consistent operation and publication evidence from every entry route |
| T11 | Source-view browser, identity, recovery, and scale proof |

Required fixtures and assertions:


- One verified subscriber has PPPoE and DHCP identities. All routes use the same policy record.

- Two routers have the same account name. Their scoped identities remain distinct.

- A new client uses a previous IP. The new observation does not inherit the old subscriber policy.

- A MAC changes or repeats. No unverified identity merge occurs.

- A router fails, restarts, or reports an old cycle. Its rows become Stale or Unknown.

- An offline account has a saved pin. No current endpoint appears. An allowed unpin uses its stable ID.

- An unlinked bound lease remains read-only. No shaping admission action appears.

- A plan has PPPoE and DHCP members. Review covers all unique affected subscribers.

- A selected subscriber disappears during review. Confirmation refuses stale scope.

- A change starts in PPPoE and finishes in IPoE. Both show the same durable receipt.

- Filters and sorting run before pagination. Cursors reject a changed snapshot.

- Keyboard, screen reader, 200 percent zoom, themes, and 320px layout checks pass before release.

## Visual references

Read [the complete screenshot guide](evidence/topology-manager/README.md).
The subscriber scenes use an independent synthetic fixture.
There are three PPPoE-linked subscribers and two DHCP-linked subscribers.
One subscriber has both identities. The shared plan has four unique members.
Each source table also shows observation-only records.

These counts do not replace the original Builder fixture's counts.

[PPPoE Users](evidence/topology-manager/pppoe-dark.png) and [IPoE Clients](evidence/topology-manager/ipoe-dark.png) show the primary layouts.
[PPPoE Local plans](evidence/topology-manager/pppoe-plans-dark.png) and [IPoE Local plans](evidence/topology-manager/ipoe-plans-dark.png) show shared plan ownership.

The references apply [Vercel frontend-design](https://github.com/vercel-labs/open-agents/blob/main/.agents/skills/frontend-design/SKILL.md) and [Google Modern Web Guidance](https://github.com/GoogleChrome/modern-web-guidance).
Google's `responsive-table` and `tabs` guides inform semantic tables, local scroll, and page navigation.
The mockups add no JavaScript tab framework or new production dependency.
STE review uses short sentences and exact technical names. It does not certify ASD-STE100 compliance.
