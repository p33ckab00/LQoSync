# Subscriber normalization and source selection

The app reads selected router sources and publishes local generations automatically.
It does not export to LibreQoS, apply shaping, write router configuration, or change packages and services.

## Set the sources

1. Configure a subscriber router with a read/API account and a password environment reference.
2. Select **Read inventory** on the Routers page.
3. Create subscriber shaping nodes under WAN roots in Topology.
4. Open **Sources**. Select a router and source for each subscriber node.
5. Save the selection. A fresh, complete, valid inventory publishes automatically.

The available sources are exact PPP profiles, DHCP servers, and Hotspot user profiles.
One source can bind to one shaping node. Duplicate bindings are rejected.
WAN and site nodes remain topology settings. They do not select subscriber tables.
Use **Speed rules** to inspect rate provenance and change minimum policy.

A source save, successful inventory read, or policy save triggers local normalization when sources are bound.
Scheduled rounds read only routers with selected sources, with at most four concurrent reads.
The configured interval starts after the previous round ends.
Pausing automatic publication also pauses scheduled collection and normalized publication.
Already running reads stop or finish before their deadlines. Queued routers do not start after a pause is observed.
On-demand inventory reads remain available while paused.
After a restart, active source bindings resume collection on the next scheduled round.
Historical inventory files are diagnostic data; the app does not trust them as fresh startup data.

## Identity and source integrity

PPP and Hotspot IDs use the router, service, and exact account name.
DHCP IDs use the router, service, and canonical MAC address.
UUID v5 creates a stable identifier from these values. It is an identity key, not a security mechanism.
IP addresses, hostnames, profile names, server selection, and shaping node IDs do not form the identifier.
Names remain exact, including spaces and case.
The same account on a different router has a different ID.

Bound DHCP leases use `active-server`, `active-address`, and `active-mac-address` when available.
A concrete configured server/MAC can supply a fallback.
A wildcard `server=all` without an active server remains unresolved.
See the [MikroTik lease reference](https://manual.mikrotik.com/docs/cli-reference/ip/dhcp-server/lease/) for configured and active fields.
A missing, duplicate, or unknown profile/server mapping blocks publication.
A known valid unselected source is skipped. An unknown mapping is never assumed to be unselected.

For selected DHCP and Hotspot sources, exact same-router IPv4 and MAC overlap uses the Hotspot account.
The DHCP record is omitted. Hotspot rates and node assignment remain authoritative for that overlap.
Other duplicate IPs or subscriber IDs block publication.
The app does not infer device identity from an IP alone.

## Health gates

Every selected router needs a successful latest read and an inventory no older than five minutes.
A future timestamp, collection deadline, failed required table, or ambiguous metadata blocks publication.
Unrelated unselected service failures do not override healthy selected service tables.
Unresolved selected speeds, invalid addresses, and identity collisions block publication.
Multiple IPs on one subscriber identity are not yet supported and remain blocked.
IPv6 and effective external RADIUS policies require additional collection support.
A wholly empty collection is blocked to prevent accidental removal of every subscriber.

Failed or partial scans retain the last valid subscriber snapshot and output generation.
Changes to settings, read epochs, or inventory invalidate calculation results before publication.
Older concurrent reads cannot replace newer inventory results.
A computed branch with no subscribers is omitted from a nonempty generation.
It remains configured and returns automatically when subscribers return.
A wholly empty snapshot still blocks publication.

## Publication and storage

A normalized generation contains:

```text
staging/GENERATION/
  subscribers.toml
  ShapedDevices.csv
  network.json
```

The app writes all three files durably before it changes `state/current.toml`.
One atomic pointer selects the authoritative generation.
The loader reads the snapshot from that generation.
A missing or corrupt committed snapshot produces an error. It does not fall back to an older mirror silently.
Identical snapshots and outputs reuse the current generation.

`state/subscribers.toml` is a compatibility mirror after a managed publication.
A mirror write failure produces a visible warning. The committed generation remains usable.
Do not edit the mirror to change a managed snapshot.
Offline data directories without source bindings can still use the legacy snapshot input.
Clearing all bindings retains a managed generation instead of republishing it as offline input.

Settings, inventory, snapshots, and output files all remain under the application data directory.
Generation retention and multi-process writer coordination are not implemented. Run one application process per data directory.

## Performance and checks

The normalizer uses indexed source/user joins and cached profile resolution.
Topology aggregation visits each subscriber's ancestors once, instead of scanning subscribers for every node.
The topology depth limit is 256 nodes.
Heavy normalization, compilation, and snapshot reads run in blocking workers outside the application lock.
The generation commit holds a serialized application guard while it writes through a blocking worker.
UI previews limit subscriber rows to 200 and CSV text to 20 data lines.

The synthetic benchmark uses 50,000 subscribers and 100 selected profiles.
It measures normalization and compilation, excluding live network time and disk publication.
Run it with:

```bash
cargo test --locked --test normalization normalize_and_compile_50k_subscribers -- --ignored --nocapture
```

Development checks use synthetic inventory and local mock RouterOS servers.
They do not require production credentials or a LibreQoS installation change.
