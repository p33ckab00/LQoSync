# Core address-list planner

The **Core lists** page configures local per-subscriber address-list plans.
Save or collection changes trigger the planner automatically when automatic publication is enabled.
The app does not send router writes or change LibreQoS.
The plan is a preview, not a RouterOS script or a sequence ready for execution.

## Configure the planner

1. Configure the WAN router with the `core` or `both` role.
2. Map its actual interface to a WAN root on **WANs**.
3. Select **Read inventory** on **Routers**.
4. Open **Core lists**.

5. Select the mapped core router for each WAN.
6. Enter an address-list name.
7. Select the existing custom FIB table for that WAN.
8. Save the core mapping.

The core router must match the router that owns the mapped WAN interface.
Different core and edge routers require more path information before this planner can validate their relationship.
One WAN can have one binding. List names and custom tables must be unique per core router.
Every WAN with active selected subscribers needs a binding after the planner is enabled.
You can save bindings one at a time. Publication waits until the configuration and inventory pass all checks.
The app does not create routing tables or firewall rules.


The form includes a revision of settings and generation state.
A stale form cannot overwrite a newer policy or generation.
The usual host and form-token checks also apply.

## Route checks

The initial route audit supports RouterOS 7 cores.
This is separate from LibreQoS version detection, which remains compatible with the existing supported versions.
Core and `both` inventory reads include address lists, full mangle/filter/rule fields, and routing tables.
Only allowlisted `print` operations are available.

Each configured list needs exactly one enabled direct rule in `prerouting`:

- `action=mark-routing`
- `src-address-list` equals the configured positive list name.
- `new-routing-mark` equals the configured custom table.
- `passthrough=false`

The audit supports an optional `dst-address-type=!local` condition.
Unrestricted source/destination addresses can use `0.0.0.0/0`.
Other packet matchers and complex prerouting actions require manual review.
The audit blocks unknown fields.
Comments, counters, log fields, and normal status fields do not select packets.

Each table must exist once, have FIB enabled, and contain one active default route.
Its resolved `immediate-gw` must identify the mapped WAN interface.
Discard defaults, ECMP, multiple active defaults, and wrong interfaces block the plan.
Enabled routing rules, FastTrack, jumps, PCC/connection-mark conditions, and unsupported matchers also block the plan.
The orchestration status shows the first failure.
The [Core review page](CORE_REVIEW.md) shows multiple findings through the same route analyzer.
RouterOS can return the enabled FIB switch as `fib=""`. The analyzer accepts this specific representation.
Other empty boolean flags remain unknown and block the plan.
See the official [routing table reference](https://manual.mikrotik.com/docs/cli-reference/routing/table/).

These are conservative structural checks. They do not prove the actual traffic path.
The audit does not simulate NAT, VRFs, destination-specific routes, connection state, or all firewall behavior.
Representative isolated router tests remain necessary before production execution.
See the official [policy-routing guide](https://help.mikrotik.com/docs/spaces/ROS/pages/59965508/Policy%2BRouting)
and [mangle reference](https://manual.mikrotik.com/docs/cli-reference/ip/firewall/mangle/).

## Desired members and ownership

The desired members use the active normalized subscriber snapshot and its assigned WAN parents.
Each subscriber contributes one individual IPv4 `/32` member to the configured WAN list.
The planner preserves subscriber speeds, minimum rates, identities, and IPs.
Inactive assignment records do not create desired members.

The first core mapping creates a persistent UUID in `settings.toml` under `core_lists.owner`.
Do not change this UUID to adopt another application's rows.
Bindings use `core_lists.bindings`.
Each desired member uses this exact comment:

```text
lqosync3:APPLICATION_UUID:SUBSCRIBER_UUID:WAN_ID
```

The complete marker, current list, and WAN scope must agree before the planner treats a row as owned.
Owned members must be static, enabled, permanent, individual hosts with unique identities and row IDs.
Malformed markers, broad owned addresses, duplicate ownership, and dynamic or expiring owned members require review.

Manual and other-owner rows remain protected.
An exact host, CIDR, or IP range that overlaps any desired subscriber blocks the plan.
DNS members inside configured lists remain unresolved and block the plan.
Rows outside the configured lists remain untouched, even if they carry this application's marker.
If you change a list binding, this does not authorize cleanup of its retired list.
The [RouterOS address-list reference](https://help.mikrotik.com/docs/spaces/ROS/pages/130220135/Address-lists)
describes host, range, DNS, dynamic, and timeout behavior.

## Preview and failures

The page shows the last committed generation, add/keep/remove totals, protected counts, and inventory ages.
Search action, subscriber ID, IPv4 address, WAN, core, or list name.
Each page contains at most 100 entries. Page links preserve the search text.
Configured policy can differ from the last committed plan after a failed collection or validation.
The page labels the plan as historical and local.

- **Add:** the desired owned host is absent.
- **Keep:** the exact owned host and marker already exist.
- **Remove:** a static owned host in a currently configured list is no longer desired.

An IP change or WAN move can produce an add and a removal for the same subscriber.
These rows do not define a safe execution order or a router transaction.
A future writer must reread ownership, coordinate route/shaping changes, and check actual results.

Failed, partial, stale, empty, or inconsistent inputs keep the entire last valid generation.
The planner records source and core router timestamps. Every input must be no older than five minutes.
The publisher rechecks these timestamps after artifact writes, immediately before it switches the generation pointer.
Settings, read epochs, errors, inventory identity, and generation guards reject superseded work.
Wholly empty subscriber scans cannot authorize mass removal.

## Files and compatibility

With core bindings enabled, each managed generation contains:

```text
staging/GENERATION/
  subscribers.toml
  assignments.toml
  ShapedDevices.csv
  network.json
  core-plan.json
```

The publisher writes all five files durably before `state/current.toml` changes.
`core-plan.json` is a compact derived artifact. Policy and assignment state continue to use application-local TOML.
The loader checks plan membership, IPs, parent nodes, ownership, bindings, and inventory provenance against that generation.

A declared missing, corrupt, or inconsistent plan blocks managed reads and publication.
Older generations without a declared core plan remain readable.
Removing all core bindings creates a normal assignment generation without a core plan after a fresh valid collection.
It does not authorize router cleanup.

Inventory timestamps form part of the plan. A fresh collection can create a generation even when host membership is unchanged.
Generation retention and multi-process writer coordination remain pending. Run one process per application data directory.

## Performance check

Run from `v3/`:

```bash
cargo test --locked --test core_plan plan_50k_subscribers_and_60k_existing_members -- --ignored --nocapture
```

The synthetic workload uses 50,000 subscribers and 60,000 existing list members.
It checks 25,000 adds, 25,000 keeps, 10,000 owned removals, and 25,000 protected members.
The planner indexes desired hosts and compares ranges without a full subscriber scan for each list row.
It audits routes once per core and sorts output for deterministic results.
The test verifies reversed input order, snapshot integrity, and unchanged subscriber data.

Measurements exclude live collection and disk publication. Debug results are not production latency guarantees.
