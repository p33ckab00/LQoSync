---
type: spec
created: 2026-11-01
status: for owner approval (builder check done)
related: ["[[ADR-0009-dhcp-discovery-and-exclusions]]", "[[2026-10-31-lqosync3-tui-design]]", "[[2026-10-31-lqosync3-v3-design]]"]
---

# lqosync3 DHCP client speed overrides and the DHCP view of the TUI (Plan 4c)

## 1. Intent

Most clients of one DHCP server have the same speed: the speed of the server comment. Some clients need a different speed. Their leases are dynamic, so they have no lease comment, and the tool gives them the server speed.

The operator wants to see the DHCP servers and their leases in the TUI, select one client, and set its speed. The setting goes into the TOML file, not into the router. The output must work with LibreQoS 2.2 (`v2_pair`) and with LibreQoS 3.x (`v3_pair`, `v3_network`).

## 2. Goals, non-goals, anti-goals

Goals:
- A speed override for one DHCP client, found by its MAC address, for each router.
- The Settings tab lists the DHCP servers of each router that have a lease, and the leases of each server, with the current speed and its source.
- Enter on a lease sets its speed. The usual procedure applies: preview (`v`), apply (`A`), confirm (`y`), verify.

Non-goals:
- A change on the MikroTik router (a lease comment, a static lease). The tool reads the router only.
- An override for PPPoE or Hotspot clients.
- An override by host name or by IP address.

Anti-goals:
- An override that changes the speed of a client other than the one with that MAC address.
- A new circuit on the production shaper that the operator did not ask for.
- Code in the TUI that calculates a rate or a group in a different way from the cycle.

## 3. Approaches

1. **TOML override by MAC address, in the cycle core** (selected by the owner). The cycle uses the override before it calculates the rate and the node group. The preview and the removal guard then show and protect the change.
2. Write a comment on the lease in MikroTik. Rejected: the tool reads the router only (TUI spec non-goal), and a dynamic lease cannot keep a comment.
3. Change the rate in the CSV after the cycle. Rejected: a copy of cycle code (anti-goal), and the `per_plan` node of the client stays wrong.

## 4. Config

New list for each router, with a `router_defaults` default of `[]`:

```toml
[[routers.dhcp.clients]]
mac = "AA:BB:CC:DD:EE:FF"   # required; the key of the client
download_mbps = 50          # required; the max rate
upload_mbps = 20            # required; the max rate
name = "Juan"               # optional; a note for the operator, shown in the TUI
```

- The loader accepts a MAC with `:` or `-` and any case, and keeps it as `AA:BB:CC:DD:EE:FF`. Another form is an error that names the entry.
- Two entries with the same MAC on one router are an error.
- `download_mbps` and `upload_mbps` must be finite and at least `bandwidth_defaults.max_floor_mbps` (LibreQoS rule: max >= 0.2).
- The min rate comes from `bandwidth_defaults.min_rate_percentage` and `safe_min_mbps`, as for each other circuit. The operator sets only the max rate.
- The override is for the router, not for one server. If the client moves to another server of the same router, the override stays.

New key `routers.dhcp.shape_unassigned` (default `false`, in `[router_defaults.dhcp]`):
- A lease with `server = all` or no server name (usually a static lease) is not shaped today. With `false`, this stays, and the report gets one warning line for each router with the number of such leases.
- With `true`, such a lease is shaped in a group named `unassigned` (node name from `nodes.dhcp_site` with `{server}` = `unassigned`). Its rate: the lease comment if it gives a rate, else the settings rate of the router (`dhcp_default_per_client_mbps`). An override applies to it too. The node rate is the number of leases × the settings rate (section 5).
- The live router (FSE-Distro) has no lease with `server = all` (checked by the owner on 2026-11-01). The RouterOS field `active-server` is not used: nobody checked it.

## 5. Cycle (`sources.rs`)

- The order of the rate sources of a lease: (1) the override of its MAC, (2) the speed sources of the server, as today.
- The override replaces only the rate. Each skip rule stays (owner decision): a disabled server, an excluded server, a server that is not discovered with `discover = false`, an excluded IP, a lease that is not bound with `require_bound = true`, a lease with no server and `shape_unassigned = false`. A skipped lease with an override stays skipped.
- With an override, the group is the group of the server mode: `per_site` keeps the server node; `per_plan` uses the plan node of the override rate.
- The rate of a `per_site` server node is the number of its circuits × the server rate (the first server-level source that gives a rate: `server_comment`, then `settings`), with the factors as today (owner decision). An override does not change the node rate. Today the node uses the rate of the first circuit by name; without overrides all circuits of a `per_site` server have the server rate, so the output is the same.
- An override for a MAC that has no lease in this cycle has no effect, and gives no warning in the report (the TUI shows it as "no active lease").
- The circuit name, the device name, the IPv4, and the comment do not change.

To keep one rule for the cycle and the TUI, the DHCP part of `sources.rs` gives a list of leases with their result (`server`, `mac`, `ip`, `host`, rate or skip reason, rate source: `override`, `server_comment`, `lease_comment`, `settings`, or `unassigned`). The cycle and the TUI both use this list.

## 6. TUI (Settings tab)

- In each router, under `dhcp`: one group for each server that has one or more leases in `snapshot.json`, plus the servers of `[[routers.dhcp.servers]]`. The title shows the server name and the number of leases. A lease without a server is in a group `no server` (with the text "not shaped" if `shape_unassigned = false`).
- A server group shows the server keys (mode, speed sources, rates) as today. An edit of a key of a server that has no `[[routers.dhcp.servers]]` entry makes the entry first.
- Below the keys: the leases of the server. Each line: host name (else MAC), IP, MAC, the current max rates and their source, and `override` if the client has one.
- A skipped lease shows in grey with its reason, for example `not shaped: excluded IP` (owner decision). `Enter` on it does not make an entry: the message gives the reason.
- `Enter` on a shaped lease: a form with the download and upload max rates (and the optional name). `Enter` saves into `[[routers.dhcp.clients]]`. `r` on a lease with an override removes the entry.
- The overrides without an active lease are listed in the router `dhcp` group with "no active lease", so the operator can remove them.
- The current rate and source come from the shared lease list of section 5, run on `snapshot.json` with the draft config. No other rate code.

## 7. Output (LibreQoS 2.2 and 3.x)

The override changes only the rates of the circuit and, in `per_plan`, its node. The writers do not change: `v2_pair` (14 columns), `v3_pair` (15 columns), and `v3_network` get the new rates in the same format.

## 8. Tests

- Config: a MAC in each form loads as `AA:BB:CC:DD:EE:FF`; a bad MAC and a duplicate MAC are errors; a rate below `max_floor_mbps` is an error.
- Cycle: an override changes only the client with that MAC, in `per_site` and in `per_plan` (the plan node changes); the `per_site` node rate does not change, also when the override client has the first name; a server-comment client keeps its rate; an override for a missing MAC has no effect; an override on a skipped lease (excluded IP, not bound) gives no circuit; `shape_unassigned = false` gives no circuit and one warning; `true` gives circuits in the `unassigned` group.
- Output: the same scenario gives the new rate in `v2_pair`, `v3_pair`, and `v3_network`.
- TUI: the server groups come from the leases; Enter on a lease makes a client entry; `r` removes it; an override without a lease shows "no active lease"; the preview shows one `~ circuit` line for the client.
- `tests/scenarios.rs` passes without a change to its expected files (no override in the scenarios, so no output change).

## 9. Left to the builder

- The exact names of the shared lease list type and function.
- The layout of the lease lines in the keys panel.
