# lqosync3 core design

This document adds the design decisions that `docs/ARCHITECTURE.md` and `docs/CONFIGURATION.md` do not give.
Read these documents first. If two documents do not agree, this document is correct for the core.

- `docs/ARCHITECTURE.md`: parts, modes, router state, performance rules.
- `docs/CONFIGURATION.md`: TOML keys, defaults, speed rules, router state, subscriber presence.
- `brain/30-knowledge/updatecsv-py-dissection.md`: the behavior of `updatecsv.py`.
- ADR-0001, ADR-0003, ADR-0004 (items 1, 2, 5), ADR-0005, ADR-0006, ADR-0007 in `brain/20-decisions/`.
- `updatecsv.py` is a guide for the domain rules only (ADR-0007). Do not copy its output format or its defects.
- `brain/30-knowledge/v3-node-handling.md`: the LibreQoS v3 node rules, with evidence.

## 1. Delivery in five plans

| Plan | Result | Network access |
|------|--------|----------------|
| 1. Core | Library: config, speed rules, state, cycle assembly with the removal guard, topology with the v3 node rules, pair render. Scenario tests. | None |
| 2. Runtime | Collector (`mikrotik-rs`, TLS, `.proplist`, timeouts, parallel routers, router states), apply (lock, hash, atomic write with `fsync`, backups, `shadow`, reload), `status.json`, signals, Telegram, CLI `check`, `once`, `run`, `diff`, systemd unit. | Routers |
| 3. LibreQoS v3 | Target detector (sets `v3`), `no_parent` check for `target = "auto"`, `compile_mode` warning, `v3_pair`, `v3_network`, v3 status check. | None |
| 4. TUI | `lqosync3 tui` (ratatui 0.30, crossterm 0.29): monitor screens, actions (sync now, reload), and a config editor with `check` before save. | Local only |
| 5. Install | `install.sh`, `import-json`, release build. | GitHub |

Each plan ends with software that passes its tests.

## 2. The core is a pure function

One cycle in the core is one function call. The core does no file access, no network access, and does not read the clock.

```text
assemble(config, state, static_rows, router_results, v3, force, now) -> CycleOutput
CycleOutput = { rows, nodes, state, report }
```

- `v3` is `true` if the target is `v3_pair` or `v3_network`. Plan 3 sets it from the target detector. It selects the default node types (section 6).
- `force` is `true` for a manual "sync now" from the TUI or the CLI. It releases a guard hold (section 4).
- `router_results` holds one `RouterResult` for each configured router, in config order.
- `RouterResult` is `Online(RouterSnapshot)`, `Failed`, `Unreachable`, or `Offline`. Plan 2 sets the state. Plan 1 receives it.
- `RouterSnapshot` holds the raw RouterOS rows of six tables: `ppp_secret`, `ppp_active`, `ppp_profile`, `hotspot_active`, `dhcp_server`, `dhcp_lease`. Each row is a map from field name to string, in the RouterOS reply order.
- `static_rows` are the rows of the current `ShapedDevices.csv` with Comment equal to `runtime.static_comment` (case is ignored). The operator can add these rows while the tool operates.

## 3. State file

Path: `<runtime.state_dir>/state.json`. Plan 2 writes it with a temporary file and a rename.

```json
{
  "version": 1,
  "circuits": {
    "<Circuit Name>": {
      "router": "Router-1",
      "source": "ppp",
      "last_seen": 1793427468,
      "row": { "Circuit ID": "...", "Circuit Name": "...", "...": "15 CSV columns as strings" }
    }
  },
  "nodes": {
    "<node name>": { "id": "lq3-1f0c9a7e5b2d4c86", "router": "Router-1", "parent": null, "type": "plan", "down": 157.5, "up": 157.5 }
  },
  "routers": { "Router-1": { "unreachable_since": null } }
}
```

- `source` is `ppp`, `hotspot`, `dhcp`, or `unknown`.
- `router` is `null` for a row from the seed. A node with `router = null` is a root node for all routers.
- `nodes` holds only the nodes that the tool made. Use it to keep the nodes of a failed router and to remove only generated nodes when `preserve_network_config = true`.
- Seed: if `state.json` does not exist, read the rows of the current `ShapedDevices.csv`. Each row that is not static becomes a circuit with `router = null`, `last_seen = now`. `source` comes from the row: Comment `PPP` gives `ppp`, a name that starts with `HS-` gives `hotspot`, a name that starts with `DHCP-` gives `dhcp`, else `unknown`. The seed keeps the Circuit ID and the Device ID.

## 4. Cycle assembly rules

The core applies these rules in this order.

1. Start with the rows from `state.circuits`. Add each static row. A static row replaces a state row with the same Circuit Name.
2. Make the IP index from the IPv4 field of all rows.
3. For each router in config order with `Online`: process PPPoE, then Hotspot, then DHCP servers in config order. Process PPPoE users in Circuit Name order. Process Hotspot and DHCP rows in RouterOS reply order.
4. For each collected circuit:
   - A new Circuit Name with an IPv4 that is in the IP index for a different name is a duplicate. Skip it, add a report warning, and do not count it in a group total.
   - A new Circuit Name gets a new Circuit ID and Device ID: `id_length` characters from `A-Z0-9`, from `/dev/urandom`. Make a new value if the ID is already in use.
   - Set the CSV fields from section 5. Set `router`, `source`, and `last_seen = now`. A collected circuit replaces a static row with the same name.
   - If two routers give the same Circuit Name, the later router in config order wins.
5. Removal, only if `runtime.remove_inactive_entries = true`. Remove a row that was not collected in this cycle if one of these conditions is true, and `now - last_seen >= subscriber_grace`:
   - Its router is `Online`.
   - Its router is not in the config.
   - Its router is `null`, and all routers are `Online`.
   - Its router is `Offline`, and `offline_action = "remove"`. For this case, ignore `subscriber_grace`.
   A static row is never removed. A row of a `Failed` or `Unreachable` router stays.
6. Removal guard, if `safety.enabled`. Let `removed` be the rows that rule 5 removes, and `base` the count of rows that are not static before rule 5. If `removed > safety.max_removed_count` or `removed × 100 > base × safety.max_removed_percent`, hold:
   - Do not remove the rows in this cycle. Keep all other changes.
   - Store `guard = { fingerprint, since }` in the state. `fingerprint` is FNV-1a 64 of the sorted removed names, joined with `\n`.
   - Add a report alert.
   - Release the hold if the next cycle has the same fingerprint, or if `force` is `true`. Then remove the rows and clear `guard`.
7. Validate the rows (section 7).
8. Make the nodes (section 6).

## 5. Circuit rules for each source

The rules follow the domain rules of `updatecsv.py`, with the changes in `docs/CONFIGURATION.md`, "Differences from `updatecsv.py`". The templates come from `[circuits]`.

| Source | Rows | Circuit Name | Device Name | MAC | IPv4 | Comment |
|--------|------|--------------|-------------|-----|------|---------|
| PPPoE | `ppp_active` names that are also in `ppp_secret`, secret `disabled != "true"` and `inactive != "true"` | `pppoe` with `{user}` | user | active `caller-id`, else secret `caller-id` | active `address` | `PPP` |
| Hotspot | `hotspot_active` | `hotspot` with `{key}` = MAC without colons, upper case (if `include_mac` and a MAC exists), else the user. Skip a row with no key. | user, else the key | `mac-address` | `address` | `HS` |
| DHCP `per_site` | `dhcp_lease` with `server` = server name, a `mac-address`, and `active-address` or `address`. With `require_bound`, also `status = "bound"`. | `dhcp` with `{key}` = `host-name`, else MAC without colons | the key | `mac-address` | `active-address`, else `address` | `host-name`, else `DHCP` |
| DHCP `per_plan` | Same | `dhcp` with `{key}` = the text before `|` in the lease comment, else `host-name`, else MAC | the key | Same | Same | the client name, else `DHCP` |

IPv6 and Anchor Node ID are always empty. Parent Node ID is the `id` of the parent node (section 6), or empty. The fields compare as strings. The rate fields use `fmt_rate` (section 8).

Rates of one circuit:

```text
max = max(clamp(base), max_floor_mbps)       clamp(x) = max(round3(x), safe_min_mbps)
min = clamp(base × min_rate_percentage)
if min > max: min = max
```

LibreQoS 2.2 and v3 reject a max below 0.2 Mbps and a min below 0.1 Mbps. Thus `max_floor_mbps` defaults to 0.2, and the loader rejects `safe_min_mbps` below 0.1.

Hotspot uses the router `download_limit_mbps` and `upload_limit_mbps` (fallback `hotspot_default_per_client_mbps`). DHCP uses the first source in `speed_sources` that gives a rate. If no source gives a rate, skip the lease.

## 6. Node rules

The nodes follow the six network modes of `updatecsv.py`, with the v3 changes of ADR-0005. The templates and types come from `[nodes]`.

| Node | Exists when | Rate |
|------|-------------|------|
| Router root (`grouped_router_root`, `flat_router_root`) | Always, for each configured router | `root_download_mbps`, `root_upload_mbps`, type `root_type` |
| Global root (`flat_global_root`) | Always | Same |
| `ppp` | PPPoE enabled, `per_plan_node = false`, grouped modes. Also with zero users. | `clamp(min(Σ base_down × pppoe_aggregate_factor, cap_down))` |
| `ppp_plan` | `per_plan_node = true`, a profile with one user or more | Same, with the sum for the profile |
| `hotspot` | One user or more | `clamp(min(count × per_client × factor, cap))` |
| `dhcp_site` | One kept lease or more | `clamp(min(count × server_rate × factor, cap))` |
| `dhcp_plan` | One kept lease or more for the plan | `clamp(min(Σ base × factor, cap))`. `{plan}` = `"<int(down)>M"` if down = up, else `"<int(down)>M-<int(up)>M"`. |
| `client` (`flat_promotion`) | Each circuit that is not static. The circuit Parent Node is its own `client` node. | The CSV max rates of the circuit |

- Grouped modes are `grouped_router_root` (group nodes are children of the router root) and `root_promotion` (group nodes are at the top level).
- `cap` is `download_mbps × multiplier` of `[backstop_caps.<source>]` if `enabled`, else no cap.
- `count` is the number of unique Circuit Names in the group, without duplicate-IP skips. Calculate `count × per_client × factor` from left to right. Add the sums in Circuit Name order.
- Each node object has five keys: `children`, `downloadBandwidthMbps`, `id`, `type`, `uploadBandwidthMbps`.
- Node `id`: `lq3-` and 16 lower-case hex characters of FNV-1a 64 of `<kind>|<router>|<key>`. `kind` is `root`, `ppp`, `ppp_plan`, `hotspot`, `dhcp_site`, or `dhcp_plan`. `router` is the router name, or empty for the global root. `key` is the profile, server, or plan label, or empty. A `client` node uses `lq3-c-<Circuit ID>`. A template change does not change the `id`.
- Node names must be unique at all levels. v3 rejects a duplicate name.
- Node type: the `type` in `[nodes]`, else the default for the target. With `v3 = false`: `ppp`, `ppp_plan`, `dhcp_plan` are `plan`, and `hotspot`, `dhcp_site` are `site`. With `v3 = true`: all group nodes are `site`. `client` is always `client` (ADR-0006).
- With a v3 target and `no_parent`, the cycle stops with an error (ADR-0006).
- The nodes of a `Failed` or `Unreachable` router come from `state.nodes`.
- With `preserve_network_config = true`, start from the current `network.json`. Remove each node in `state.nodes`. Then add the new nodes. An existing root node keeps its other children and gets the new rate and type.

## 7. Validation

The validator stops the cycle with an error for these cases:

- Two rows with the same Circuit ID.
- A row with a Parent Node that is not a node, when the mode makes nodes. `no_parent` gives an empty Parent Node.
- A row with a Parent Node ID that is not the `id` of its Parent Node.
- Two nodes with the same name, at any level of `network.json`.
- A min rate above the max rate.
- A max rate below 0.2 or a min rate below 0.1 (LibreQoS rejects these).

The validator removes a row for a duplicate IPv4 and adds a report warning. It keeps one row in this order: a row collected in this cycle, then a row that existed before, then the row with the lower Circuit Name.

## 8. Output format

- `round3(x)`: round to 3 decimals, half to even, with `format!("{:.3}", x)` and a parse to `f64`.
- `fmt_rate(x)`: `format!("{:.3}", x)`, then remove the trailing zeros and a trailing `.`. Examples: `50.0` gives `50`, `157.5` gives `157.5`, `0.128` gives `0.128`.
- `ShapedDevices.csv`: header, then rows sorted by (Parent Node, Circuit Name, Device Name, IPv4, MAC). Terminator `\n`. Minimal quoting.
- Columns: `v2_pair` writes 14 columns (the 2.2 example: `Parent Node ID` after `Parent Node`). `v3_pair` writes 15 columns (the v3 example: `Parent Node ID`, `Anchor Node ID` after `Parent Node`). The reader accepts 13, 14, or 15 columns by header name.
- `network.json`: `serde_json` with the default sorted map, `PrettyFormatter::with_indent(b"    ")`, and a final `\n`. Rates are JSON numbers rounded with `round3`.
- Static rows and rows of a failed router are written with their stored strings, without a new format.

## 9. Tests

- Scenario tests in `tests/scenarios/<case>/`: `config.toml`, `snapshot.json`, `previous.csv`, and the expected `ShapedDevices.csv` and `network.json`. We write the expected files from this spec. They do not come from `updatecsv.py`.
- The fixtures are synthetic. Do not put live subscriber names, MAC addresses, or IP addresses in the repository.
- `tools/v3sim/check.sh` runs the v3 compiler on a scenario output, if the v3 binding is present (dev only).
- Plan 2 adds `lqosync3 diff`: a semantic compare of the tool output with the live files (circuits, IPs, parents, rates) before the cutover.
