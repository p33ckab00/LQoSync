---
type: spec
created: 2026-11-19
status: draft
related: ["[[topology-manager-status]]", "[[2026-11-18-topology-subscriber-plans]]", "[[lqosync-owned-profiles-and-wan-policy]]"]
---

# WAN discovery design (Phase 2b-3a)

## Intent

The owner wants to build the production tree in LQoSync: two WAN nodes, with one instance of each subscriber group below each WAN, and LQoSync balancing subscribers between the WANs.
The two WANs are `pppoe-out` interfaces on a MikroTik core router. Today no catalog source or code reads them.
This slice adds the core router as a read-only WAN reader and shows its interfaces as WAN candidates in the Builder Inventory.
The operator confirms a candidate as a WAN node in Phase 2b-3b. This slice makes no registry change.

## Owner decisions (2026-11-19)

- Production has two WANs. Each is a `pppoe-out` interface on the MikroTik core.
- The core routes by policy-based routing with per-IP address-lists. An owner script fills the lists today.
- Subscriber groups are not tied to one WAN. LQoSync decides the WAN of each subscriber by load balancing.
- Writing the core address-list (routing) is a later feature. Finish the Topology Manager first. All router access stays read-only.
- Roles are per router, in any combination: PPP, DHCP, Hotspot, WAN. Any router can have the WAN role. A core with only the WAN role reads only `/interface`.
- The router add form with role selection belongs to Phase 2c (Settings). In this slice the owner adds the core router to `config.toml` by hand.
- Approach A (below).
- The legacy cycle skips a router with all subscriber protocols disabled. The owner reviews this again when the Topology Manager is complete.

## Slices

| Slice | Content |
|---|---|
| 2b-3a (this design) | Core router as a WAN reader, `wan.discover`, WAN candidates in the Builder Inventory |
| 2b-3b | Builder edits: WAN node from a candidate with manual capacity, intermediate nodes, a source instance below several parents, move and remove with affected members, capacity in Details |
| 2b-3c | Pin and scoped rebalance through the API and the TUI. The balancer and the rebalance engine exist in `rebalance.rs` |

## Goals

- The operator reads the interfaces of each configured router on demand and sees the likely WANs first.
- A candidate that a catalog WAN source already uses shows that use. A catalog WAN source whose interface is absent shows as missing.
- The running production service and its configuration continue to operate without change.

## Non-goals

- No registry or catalog change. No WAN node creation (2b-3b).
- No router write. No address-list or routing change.
- No WAN read in each preview refresh or cycle.
- No router add from the TUI (2c).

## Anti-goals

- A change that makes the current production binary refuse its configuration.
- A discovery result that LQoSync treats as proof of an Internet WAN. A candidate is a signal. The operator confirms it.

## Approaches

| Approach | Summary | Trade-off |
|---|---|---|
| A. WAN role in the registry, discovery on demand (chosen) | The core is a `[[routers]]` entry with PPP, DHCP, and Hotspot disabled. `wan.discover` reads `/interface` from configured routers. 2b-3b puts the WAN role on the catalog router when the operator confirms a candidate | No new configuration key, so the old binary still reads the file. A renamed or removed interface shows only at the next discovery |
| B. WAN flag in the configuration, read in each cycle | `[routers.wan] enabled = true`, like PPP and DHCP. The coordinator reads `/interface` at each preview refresh | Fresh candidates and WAN state. `RouterCfg` has `deny_unknown_fields`, so the current production binary refuses the file until the new binary is deployed. More router reads |

The owner chose A because it has no risk for the running service and the WAN selection occurs once.
An approach where the operator types the interface name was dropped, because the owner asked for discovery.

## Model

- The core router is one `[[routers]]` entry: name, address, username, `password_file`, and `enabled = false` for `pppoe`, `dhcp`, and `hotspot`. No new key.
- The router ID is `policy_draft::router_id(name)`, the same ID that `read_catalog` gives. Discovery matches catalog routers to configured routers by name, as the coordinator does (designer call).
- The read is one `/interface` print with the `Table::Interfaces` fields (`.id,name,type,disabled,running,comment`), with the existing session and time-limit code. It does not use the double-read stability check of `ReadGuard::read_current`. On a PPP router, dynamic `pppoe-in` rows change between two reads, so that check would fail often. A candidate is a signal that the operator confirms, so one read is sufficient (designer call).
- A candidate identity is the router ID and the interface name. The name is the future `endpoint_ref` of a WAN source. The RouterOS `.id` is a row locator and is not stored.

## Operation (new)

| Operation | Args | Result | Rules |
|---|---|---|---|
| `wan.discover` | none | `{collected_at, routers: [{router_id, name, state, error, candidates: [...], missing: [...]}], unconfigured: [...]}` | Reads every configured router at the same time with one time limit of 30 s. No enrollment or registry is necessary. Changes no state except the cached answer |

- `state` is `ok` or `failed`. `error` is the read error text for `failed` (for example a login failure), else `null`. A failed router does not fail the operation.
- `candidates` has one entry for each interface: `{name, type, running, disabled, comment, suggested, source_id, used_by}`.
- `suggested` is `true` when `type` is `pppoe-out`.
- Interfaces whose `type` ends with `-in` are server-side dynamic tunnels (for example PPP subscriber sessions). Discovery leaves them out.
- Order: suggested first, then by name.
- `source_id` is the catalog WAN source with this router and this interface name as `endpoint_ref`, else `null`. `used_by` is the sorted list of catalog node IDs that use that source. It is empty when no node uses the source or no source exists. A source can have several instances, so `used_by` is a list (designer call).
- `missing` lists `{source_id, endpoint_ref}` for catalog WAN sources on this router that the read did not find. Only an `ok` router has a `missing` list.
- `unconfigured` lists `{source_id, router_id, endpoint_ref}` for catalog WAN sources whose catalog router has no `[[routers]]` entry with the same name (designer call).
- With no registry catalog, `source_id` is `null`, and `used_by`, `missing`, and `unconfigured` are empty.
- The runner keeps the last answer in memory. Phase 2b-3b uses it to check that a confirmed candidate was discovered.

## Compatibility with the running service

- Today the legacy cycle reads every `[[routers]]` entry. For a router with all protocols disabled, it connects, logs in, and reads no table. When that router fails or is unreachable, the cycle is not "all online", so it holds row removals for the other routers, and it adds router errors, offline status, alerts, and notifications at each cycle.
- Owner decision: the legacy cycle skips a router with PPP, DHCP, and Hotspot all disabled. It does not connect to it, and the router has no status, alert, or notification in the legacy cycle. Only `wan.discover` reads it. The owner reviews this again when the Topology Manager is complete.
- Tests must prove that a skipped router has no connect, no effect on "all online", and no status, alert, or notification. A test with a skipped router that is unreachable must show that removals for the other routers continue.
- Network mode `root_promotion` (production) adds no node for a router without sources. The compatibility test pins this mode. The `grouped_router_root` and `flat_router_root` modes add a root node for each configured router and are out of scope.
- The topology read for proposal drops routers without roles (`required.retain`). The enrolled catalog does not contain the core, so preparation does not read it.
- A test must prove that one more router with all protocols disabled changes no legacy cycle output, no proposal read, and no prepared preview.
- This is the only legacy code change in this slice.
- The production service runs an older build. The owner adds the core entry together with the deployment of the tested build, in the Phase 4 runbook. The TUI cannot reach `wan.discover` before that deployment, because the current production service has no API socket.

## TUI

- The Builder Inventory panel gets a "WAN candidates" section below the subscriber sources.
- A key in the Builder starts `wan.discover`. While it runs, the section shows "reading routers". The other views keep working.
- Each router shows its name and state. A failed router shows its error. Each candidate shows name, type, running state, a mark for `suggested`, and `used_by`. Missing WAN sources show as "missing".
- Before the first discovery, the section shows the key to start it.
- The section fits 80 columns.

## Known limit for later slices

Until the routing feature exists, the owner script decides the real WAN of a subscriber.
The WAN that LQoSync selects can differ from the real path. Before Phase 4 activation this has no effect.
After activation, the load of each WAN node can differ from the real load until the routing feature writes the address-lists.

## Verification

- Service tests: a router with `pppoe-out` and `ether` interfaces gives the `pppoe-out` first with `suggested`. `-in` interfaces are left out. One failed router and one good router give one `failed` and one `ok` entry. A catalog WAN source gives `used_by`, and its absent interface gives `missing`. The operation is dispatched.
- Compatibility tests: one more router with all protocols disabled changes no legacy cycle output, no proposal read, and no prepared preview. The legacy cycle does not connect to it, and its failure does not hold removals.
- TUI tests: the key sends `wan.discover`; the section shows routers, candidates, marks, errors, and missing entries; the section fits 80 columns.
- Full suite, format, Clippy with warnings denied, and whitespace checks.

## Left to the builder

- The exact key, within the global key rules.
- Line format of the section.
- Where the read lives (coordinator or runner), if it reuses the existing session and time-limit code.
