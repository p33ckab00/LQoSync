# Flexible topology and reconnect assignment

The operator controls the exported hierarchy. Discovered records do not become queues automatically.
WAN roots stay explicit. Custom groups can contain branches and selected subscribers from different services.
Every exported node remains a real shaping node with directional capacity checks.

## Select and arrange sources

1. Read fresh router inventory.
2. Create WAN roots and map their interfaces on WANs.
3. Add named groups or service branches in Topology. Use Parent to change their hierarchy.
4. Open **Source selection and automatic grouping**, or open Sources.
5. Select a profile, server, account, or DHCP MAC identity.
6. Enter an existing destination node ID. Alternatively, enter a parent ID to create a branch automatically.
7. Save the independent selection.

Search filters the discovered choices. The controls show at most 200 choices and selections.
An exact individual selection overrides the corresponding group selection.
Select **Exclude matching records** to omit a record from normalization.
An exclusion does not delete a RouterOS account, lease, profile, or rule.
An excluded record can disappear from complete inventory without invalidating the selection.

Legacy source bindings on service nodes remain supported. Do not add a duplicate independent group binding for the same source.
Independent selections have stable IDs. Subscriber IDs do not change when their destination or category changes.
The current implementation supports PPP profiles/accounts, DHCP servers/MAC identities, and Hotspot profiles/accounts.
Generic `group`, `site`, and WAN nodes can receive independent sources.

Remove a selection to stop its inclusion. The topology node and RouterOS records remain intact.
Remove an unused node through its inspector. Child, subscriber, pool, pin, and selection validation still apply.
Invalid edits keep the prior settings or output generation, as applicable.

## Choose manual or automatic assignment

On Assignments, configure eligible destinations for each bound source node.
Destinations can use the same service kind or a generic group. They must not belong to another source.
Map every participating WAN interface first.

Manual mode keeps the original source destination unless a pin selects another eligible target.
Automatic mode offers two metrics:

- `online_clients`: choose the WAN with the smallest count of observed clients or bound leases.
- `planned_load`: choose by projected plan rates relative to capacity. This mode keeps historical assignments.

Online count is the default metric for new pools and saved pools that omit `metric`.
Use `metric = "planned_load"` to keep the earlier weighted policy explicitly.
Continuous online sessions keep their compatible destination. Pins take priority over automatic choices.
Only new sessions and reconnects become eligible for count redistribution.
Equal WAN counts use branch counts and stable destination IDs for deterministic results.

The first session scan adopts compatible committed assignments. It does not force a migration of continuous clients.
Later evidence can release an assignment after healthy absence, a changed session handle, or reset uptime.
PPP and Hotspot count mode require a valid session handle from the active table.
DHCP count mode uses a bound lease and its active identity. This does not prove physical device connectivity.
A dynamic DHCP lease can disappear from a complete healthy table and later return.

Failed, stale, partial, ambiguous, and older observations do not establish a disconnect.
Two successful reads in the same second can differ. Read tickets and snapshot identity establish ordering before persistence.
A reconnect without observable absence, a new handle, or reset uptime cannot be detected by polling.

## Candidate and committed state

Session evidence and proposed assignments use private `state/online-assignments.toml`.
The file contains normalized subscriber IDs and selected destinations. It does not contain passwords or raw inventory.
The application validates stored evidence and rejects corrupt, oversized, or concurrently changed candidate files.
Candidate files have a 64 MiB limit and at most 250,000 assignment or session records.
One application process must own each data directory.

Healthy empty scans update absence evidence while preserving the last valid output pair.
Core ownership or capacity failures can block publication after candidate evidence advances.
The committed generation and its assignment format remain unchanged.
Session timestamps do not create new generations when the actual output and assignments are unchanged.

Assignments shows fresh candidate clients when available. Candidate clients can receive manual pins before publication.
If live input is unavailable, the page labels its committed fallback. Candidate-only pin saves require fresh evidence.
Topology labels saved count-mode observations as candidate data. Validation uses the same planner without saving its preview.
The core plan and shaping files consume the same assigned parents.

No RouterOS write, production shaping, package change, or service restart occurs through these controls.
Protected manual core entries still block publication. This feature does not adopt or remove them.

## Compatibility and recovery

Earlier settings without `selections` remain readable. Legacy node bindings keep their meaning.
Earlier committed assignment files remain readable. Candidate observations are separate from the durable generation pointer.
Keep private settings backups before operating a newer binary against live configuration.
An older binary cannot read the new selection or metric fields. Restore its matching settings backup before rollback.
Never edit the committed generation to repair candidate state. Investigate a corrupt candidate file before collecting again.
