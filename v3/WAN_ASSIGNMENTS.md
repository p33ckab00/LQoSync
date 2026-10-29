# WAN mapping and subscriber assignments

The WANs and Assignments pages configure intended subscriber parents for local output.
Core routing and address-list synchronization require a future production adapter.
These assignments do not change the actual path of subscriber traffic yet.
The [core list planner](CORE_PLANNER.md) now previews owned per-host list changes and audits supported routing structures.

## Map WAN interfaces

1. Configure the core or edge router on the Routers page.
2. Select **Read inventory**.
3. Create an explicit WAN root in Topology.

4. Open **WANs**.
5. Select the router interface for that WAN root.
6. Enter download and upload capacities in Mbps.
7. Save the WAN mapping.

Interface names, route data, and interface lists do not prove which interface carries a subscriber's traffic.
The operator must select the actual WAN interface.
The interface can be a physical interface, VLAN, PPP client, or another interface from RouterOS inventory.
One router/interface pair can map to one WAN root only.
Two interfaces on the same router can map to separate WAN roots.

Mapped WAN routers join scheduled collection. This includes routers with the core role and no subscriber sources.
Collection retains the four-router concurrency limit and pause controls.
Inventory choices expire after five minutes. Failed reads and incomplete or duplicate interface metadata block new mapping choices.

For normalized publication, every mapped WAN requires fresh interface inventory.
RouterOS must report `running=true` and `disabled=false` for the selected interface.
An unavailable interface blocks the next generation. The last valid generation remains selected.
The app does not migrate subscribers or infer failover during an outage.
Unmapped WAN roots remain compatible with source-only operation, but cannot participate in assignment pools or pins.

## Configure source pools

1. Create the source subscriber branch under a WAN root in Topology.
2. Bind its router/profile/server on the Sources page.
3. Create additional subscriber branches under other mapped WAN roots.

4. Leave these branches without their own source bindings.
5. Open **Assignments**.
6. Add compatible destinations to the source pool.
7. Select the assignment mode.

A source pool must keep its original source node.
More destinations require the same subscriber kind: PPPoE, DHCP, or Hotspot.
A destination cannot belong to another selected subscriber source.
Source collection remains attached to the original node. The destination changes only the exported subscriber parent.
The discovered network map remains separate from the exported tree.

The default mode keeps new subscribers on their source node, unless a pin selects another eligible destination.
Optional balance mode distributes new or unassigned subscribers by planned load relative to capacity.
It does not move subscribers with valid previous assignments.

## Assignment order and capacity

The app applies an explicit pin first, then a valid previous assignment.
It calculates retained, pinned, and default subscriber loads before it distributes new balanced subscribers.
New balanced subscribers follow sorted subscriber IDs. Equal scores use sorted destination IDs.
The same inputs produce the same result, regardless of source table order.

Each candidate score uses its projected download and upload plan totals divided by WAN capacities.
The larger directional ratio controls the score.
An explicit destination capacity adds another ratio. The larger WAN or destination ratio controls the score.
The app selects the destination with the smallest score.

This calculation uses subscriber plan rates, not measured traffic.
Plan totals can exceed WAN capacity. The score is not an admission limit.
The topology compiler caps computed aggregate capacities at their immediate parents.
Subscriber speeds, minimum rates, IDs, and IPv4 addresses remain unchanged.

## Subscriber controls and persistence

Search subscribers by ID, name, or IPv4 address. Each page contains at most 100 records.
Select a destination and choose **Pin subscriber** to move the intended parent automatically.
If you clear a pin, an eligible previous assignment stays in place.
This does not force a subscriber back to the source or redistribute subscribers.

Clear stored pins before you remove their destinations or source bindings.
The stored-pin view includes inactive subscribers and supports search by ID.
The current source-pool view displays the first 200 bound sources.

Inactive subscribers keep their previous assignments across nonempty scans and restarts.
Inactive records do not add plan load.
If you remove a source binding or eligible destination, the next valid publication removes its inactive assignment records.
Subscribers whose old destination is no longer eligible follow the current mode when they return.
A source change with a stored pin blocks publication until you correct or clear the pin.

Settings use the application's `settings.toml` under `assignment.pools` and `assignment.pins`.
Runtime ownership uses `assignments.toml` inside each immutable local generation.
Every active subscriber must have a matching assignment record and target parent.
Inactive records can coexist with the active snapshot.

Missing, corrupt, or inconsistent committed assignment data blocks publication. There is no silent assignment reset.
All four generation files exist durably before `state/current.toml` changes.
The UI reads the snapshot and assignments from the same pointer decision.
Older normalized generations derive their initial ownership from their existing subscriber parents.

Save triggers automatic local normalization and publication when automatic publication is enabled and all health gates pass.
Settings or generation changes invalidate stale edits. New reads invalidate stale calculation results.
Local publication needs no separate **Apply** or **Verify** step.
No LibreQoS file, package, service, or router configuration changes occur.

## Performance check

Run the synthetic assignment benchmark from `v3/`:

```bash
cargo test --locked --test assignments assign_50k_subscribers_across_eight_wans -- --ignored --nocapture
```

The benchmark checks 50,000 subscribers across eight equal-capacity WANs.
It verifies equal distribution, retained assignments, unchanged rates, and successful CSV compilation.
The initial debug measurement was 525 ms for new assignments and 229 ms for retained assignments.
These measurements exclude network collection and disk publication. They are not production latency guarantees.
