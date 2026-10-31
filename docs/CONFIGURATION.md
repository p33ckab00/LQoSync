# Configuration

The tool reads one TOML file. The default path is `/etc/lqosync3/lqosync3.toml`.
Use `--config <path>` to read a different file.

## Rules

- The loader rejects an unknown key. A spelling error stops the start and shows the key name.
- Do not put a password or a token in the TOML file. Use a `*_file` key or a `*_env` key.
- Each secret file must have mode `0600` and owner `root`. If not, the tool does not start.
- Rates are in Mbps. Intervals and timeouts are in seconds.
- The loader rejects a value out of range. Examples: `min_rate_percentage` above `1.0`, a factor below `0`, a timeout of `0`, `safe_min_mbps` below `0.1`, `max_floor_mbps` below `0.2`.
- Run `lqosync3 check` after each change.

## Policy in TOML, formats in code

All policy and tuning values are in TOML. These items are fixed in the code, because LibreQoS or RouterOS defines them:

- The RouterOS API paths, for example `/ppp/active`.
- The 13 columns of `ShapedDevices.csv` and their order.
- The file names `ShapedDevices.csv`, `network.json`, and `libreqos_network.json`.
- The `libreqos_network.json` schema.

## Differences from `updatecsv.py`

lqosync3 is a new design. `updatecsv.py` is a guide for the domain rules only. Refer to [ADR-0007](../brain/20-decisions/ADR-0007-fresh-design-not-a-port.md).
Before the cutover, `lqosync3 diff` shows each difference against the live files.

| Item | lqosync3 | `updatecsv.py` |
|------|----------|----------------|
| `rate-limit` order and units | RouterOS: `rx/tx`, a number with no unit is bit/s | First value is download, no unit is Mbps |
| DHCP `per_site` speed | Server comment, then settings | Settings only |
| Profile name speed | A number with a unit (`Tier-15M-WAN1` gives 15) | The last number (gives 1) |
| Comment speed | Units `k`, `M`, `G` in a pair. A unit must end the word (`5Main` is not a speed). | `M` only, no word end |
| Router failure | Keep the last good rows | Remove the rows if a different router succeeds |
| Mass removal | Hold the write (`[safety]`) | Remove at once |
| Rate floor | max >= `max_floor_mbps` (0.2) | `safe_min_mbps` (0.128) |
| Node `id`, `Parent Node ID` | Written ([ADR-0005](../brain/20-decisions/ADR-0005-topology-follows-v3.md)) | Not written |
| `flat_promotion` Parent Node | The circuit's own `client` node | Empty |
| Rate text | `50`, `157.5` | `50.0`, `157.5` |
| File write | Temporary file, `fsync`, rename | Write in place |

## Example

```toml
# /etc/lqosync3/lqosync3.toml

[runtime]
mode = "shadow"                 # shadow | apply
scan_interval_active = 25       # seconds, after a change
scan_interval_idle = 120        # seconds, when there is no change
error_retry_interval = 30       # seconds, after a cycle error
id_length = 8                   # length of new circuit IDs and device IDs
remove_inactive_entries = true
static_comment = "static"       # a row with this Comment is never removed (case is ignored)
subscriber_grace = 0            # seconds that a missing subscriber stays before removal
log_level = "info"              # error | warn | info | debug
state_dir = "/var/lib/lqosync3"

[router_access]
connect_timeout = 5             # seconds, for the TCP connection and the login
api_timeout = 10                # seconds, for each RouterOS API call
read_retries = 2                # more attempts for a failed read in the same cycle
retry_delay = 3                 # seconds between two attempts
offline_after = 300             # seconds (300 = 5 minutes) without a connection, then the router is offline
offline_action = "remove"       # remove | keep

[libreqos]
target = "auto"                 # auto | v2_pair | v3_pair | v3_network
directory = "/opt/libreqos/src" # LibreQoS lqos_directory
state_directory = "/opt/libreqos/state"
v3_config = "/etc/libreqos/lqos.conf"
v2_command = "/opt/libreqos/src/LibreQoS.py"
v2_args = ["--updateonly"]
v2_timeout = 300                # seconds
v3_status_timeout = 60          # seconds to wait for lqosd to accept the new files

[network]
mode = "grouped_router_root"    # refer to ARCHITECTURE.md, "Network modes"
preserve_network_config = false
root_name = "LibreQoS-Network"  # root name for flat_global_root
root_type = "site"
root_download_mbps = 640
root_upload_mbps = 640

[nodes]                         # names of the nodes that the tool makes; "type" is optional
ppp = { name = "PPP-{router}" }
ppp_plan = { name = "{profile}-{router}" }
hotspot = { name = "HS-{router}" }
dhcp_site = { name = "DHCP-{server}-{router}" }
dhcp_plan = { name = "PLAN-DHCP-{plan}-{router}" }
client = { type = "client" }    # flat_promotion only

[circuits]                      # Circuit Name of each source
pppoe = "{user}"
hotspot = "HS-{key}"            # key = MAC without colons, or the user name
dhcp = "DHCP-{key}"             # key = client name, host name, or MAC without colons

[speed]
rate_limit_order = "routeros_rx_tx"    # routeros_rx_tx | download_upload
unitless_unit = "bps"                  # bps | mbps
profile_name_regex = '(\d+(?:\.\d+)?)\s*([kKmMgG])'
pppoe_sources = ["secret_comment", "active_comment", "profile_comment", "profile_rate_limit", "profile_name", "default"]

[safety]
enabled = true                  # hold a write that removes too many rows
max_removed_percent = 20        # percent of the circuits in one cycle
max_removed_count = 50          # rows in one cycle

[bandwidth_defaults]
min_rate_percentage = 0.50
safe_min_mbps = 0.128           # floor of the min rate, 0.1 or more
max_floor_mbps = 0.2            # floor of the max rate, 0.2 or more (LibreQoS rule)
pppoe_default_profile = "default"
pppoe_default_rate = "10M/10M"
pppoe_aggregate_factor = 0.50
dhcp_default_per_client_mbps = 15
dhcp_download_factor = 0.5
dhcp_upload_factor = 0.5
hotspot_default_per_client_mbps = 10
hotspot_download_factor = 1.0
hotspot_upload_factor = 1.0

[backstop_caps.pppoe]
enabled = false
download_mbps = 600
upload_mbps = 600
multiplier = 1.0

[backstop_caps.dhcp]
enabled = false
download_mbps = 250
upload_mbps = 250
multiplier = 1.0

[backstop_caps.hotspot]
enabled = false
download_mbps = 100
upload_mbps = 100
multiplier = 1.0

[telegram]
enabled = false
bot_token_file = "/etc/lqosync3/secrets/telegram.token"
chat_id = ""
timeout = 10                    # seconds, for each send
max_message_chars = 3900        # the tool splits a longer message
notify_on_changes = true
notify_on_error = true
notify_on_preflight_warning = true
include_added = true
include_updated = true
include_removed = true
include_computation = true
include_telemetry = true
max_list_items = 15

[[routers]]
name = "Router-1"
address = "192.0.2.1"
port = 8729                     # 8729 = API over TLS, 8728 = plain API
tls = true
username = "lqosync-read"
password_file = "/etc/lqosync3/secrets/Router-1.password"

[routers.pppoe]
enabled = true
per_plan_node = true

[routers.hotspot]
enabled = false
include_mac = true
download_limit_mbps = 10
upload_limit_mbps = 10
download_factor = 1.0
upload_factor = 1.0

[routers.dhcp]
enabled = true
require_bound = false           # true = use only leases with status "bound"

[[routers.dhcp.servers]]
name = "LAN"
mode = "per_site"               # per_site | per_plan
speed_sources = ["server_comment", "settings"]
default_plan_down_mbps = 50
default_plan_up_mbps = 50
download_factor = 1.0
upload_factor = 1.0
```

## Removal guard

A bad router reply can make many subscribers disappear in one cycle. The guard stops this.

1. If one cycle removes more than `max_removed_count` rows, or more than `max_removed_percent` of the circuits, the tool holds the removal. It writes all other changes and sends an alert.
2. If the next cycle removes the same rows, the tool applies the removal.
3. "Sync now" in the TUI or `lqosync3 once --force` applies the removal at once.

## Router state

The tool keeps a state for each router. The state controls what happens to the subscribers of that router.

| State | Cause | Subscribers of the router |
|-------|-------|---------------------------|
| `online` | All reads in the cycle passed. | The tool uses the new data. |
| `failed` | The tool connected, but a read failed after `read_retries` more attempts. A login error is also `failed`. | The tool keeps the last good rows. The next cycle tries again. |
| `unreachable` | The tool cannot connect. The time is less than `offline_after`. | The tool keeps the last good rows. |
| `offline` | The tool cannot connect for `offline_after` seconds or more. | `offline_action = "remove"`: the tool removes the rows. `"keep"`: the tool keeps them. |

A timeout of an API call is a read failure. The tool closes the session, opens a new session, and tries the read again.
Set `offline_after = 0` to remove the subscribers at the first failed connection.

> [!caution]
> "Offline" means that the shaper cannot connect to the router API. If only the management path is down, the router can still send subscriber traffic.
> In that case, `offline_action = "remove"` removes the shaping of these subscribers. Make `offline_after` longer than a usual network problem.

## Subscriber presence

The tool removes a subscriber row when the subscriber is not present for `subscriber_grace` seconds. With `subscriber_grace = 0`, the tool removes the row in the first cycle without the subscriber, as `updatecsv.py` does.

| Source | Present when | Not present when |
|--------|--------------|------------------|
| PPPoE | The name is in `/ppp/active` and in `/ppp/secret`. The secret is not disabled. | The session closed, or the secret is deleted or disabled. |
| Hotspot | The user is in `/ip/hotspot/active`. | The session closed. |
| DHCP | The lease has a MAC and an address. With `require_bound = true`, the lease status must also be `bound`. | The lease is deleted, or (with `require_bound = true`) the status is not `bound`. |

The tool does these checks only for a router in the `online` state.

## Speed rules

The tool finds a speed for each subscriber. It tries the sources in the list order. The first source that gives a speed wins.

### PPPoE: `speed.pppoe_sources`

| Source | Reads |
|--------|-------|
| `secret_comment` | The comment of the PPP secret |
| `active_comment` | The comment of the PPP active session |
| `profile_comment` | The comment of the PPP profile |
| `profile_rate_limit` | The first rate pair in the profile `rate-limit` |
| `profile_name` | The profile name, with `speed.profile_name_regex` |
| `default` | `bandwidth_defaults.pppoe_default_rate` |

### DHCP: `routers.dhcp.servers[].speed_sources`

| Source | Reads |
|--------|-------|
| `server_comment` | The comment of the DHCP server in `/ip/dhcp-server` |
| `lease_comment` | The comment of the lease. The text before `|` is the client name. |
| `settings` | `default_plan_down_mbps` and `default_plan_up_mbps` of the server |

The default list depends on `mode`:

- `per_site`: `["server_comment", "settings"]`.
- `per_plan`: `["lease_comment"]`.

With `per_site`, the loader rejects `lease_comment`. The site node rate is the lease count multiplied by one server rate, so all leases of the server must have the same rate.

If no source gives a speed, the tool skips the lease and writes a log line.

### Hotspot

The tool uses `routers.hotspot.download_limit_mbps` and `upload_limit_mbps` for each user.

### Comment format

A comment gives a speed in one of these forms. The first form found wins.

1. `Name|25M/5M`: download 25 Mbps, upload 5 Mbps.
2. `25M/5M`: download 25 Mbps, upload 5 Mbps.
3. `25M`: 25 Mbps for download and upload.

Forms 1 and 2 accept the units `k`, `M`, and `G`. Form 3 accepts only `M`, because a name like `Wifi 5G` is not a speed. The unit must end the word: `5Main` is not a speed. Comments always use the download/upload order.

### Rate direction and units

| Key | Value | Meaning |
|-----|-------|---------|
| `rate_limit_order` | `routeros_rx_tx` (default) | RouterOS order. The first rate is the router receive rate, which is the client upload rate. This is what the router enforces. |
| | `download_upload` | The first rate in `A/B` is the download rate (`updatecsv.py` behavior). |
| `unitless_unit` | `bps` (default) | `10` is 10 bit/s (RouterOS behavior). |
| | `mbps` | `10` is 10 Mbps (`updatecsv.py` behavior). |

## Runtime modes

| `runtime.mode` | Router reads | LibreQoS files | Reload |
|----------------|--------------|----------------|--------|
| `shadow` | Yes | No. The tool writes candidate files to `state_dir/candidate/`. | No |
| `apply` | Yes | Yes | Yes |

Use `shadow` to compare the output with the Python service before the cutover.

## Node types

If a `[nodes]` entry has no `type`, the tool uses this default:

| Node | `v2_pair` | `v3_pair`, `v3_network` |
|------|-----------|--------------------------|
| `ppp`, `ppp_plan`, `dhcp_plan` | `plan` | `site` |
| `hotspot`, `dhcp_site` | `site` | `site` |
| `client` | `client` | `client` |

LibreQoS v3 with `compile_mode = "ap_site"` can keep only `site` and `ap` nodes. Refer to [ADR-0006](../brain/20-decisions/ADR-0006-v3-guards-and-node-types.md).
A `type` in `[nodes]` applies to all targets.

## LibreQoS v3 checks

- `network.mode = "no_parent"` with a v3 target: `check` fails and the cycle stops. Use `root_promotion`. v3 does not accept an empty Parent Node ID.
- `[topology] compile_mode` other than `full` in `/etc/libreqos/lqos.conf`: `check` and each cycle give a warning. With `flat`, v3 ignores the node tree.

## Name templates

`[nodes]` and `[circuits]` use these fields: `{router}`, `{profile}`, `{server}`, `{plan}`, `{user}`, and `{key}`. A `[circuits]` template can use `{router}`, for example `"{user}@{router}"`.
The tool records each node that it makes in `state_dir`. It deletes only these nodes. It does not delete a node that an operator added.

> [!caution]
> Do not change a `[circuits]` template on a live system. A new template gives new Circuit Names. Then the tool removes the old rows and adds new rows with new IDs.

## Map from `config.json`

The section names stay the same. These keys change.

| `config.json` key | TOML key | Change |
|-------------------|----------|--------|
| `runtime.scan_interval_active` | `runtime.scan_interval_active` | Same |
| `runtime.log_file` | (removed) | The tool writes to the systemd journal. |
| `libreqos.enabled` | `runtime.mode` | `false` is the same as `shadow`. |
| `libreqos.command` | `libreqos.v2_command` | Arguments go in `libreqos.v2_args`. |
| `libreqos.timeout_seconds` | `libreqos.v2_timeout` | Same value |
| `libreqos.sudo` | (removed) | The service operates as `root`. |
| `libreqos.shaped_devices_csv`, `libreqos.network_json` | `libreqos.directory` | The file names are fixed by LibreQoS. |
| `telegram.bot_token` | `telegram.bot_token_file` | Put the token in a file. |
| `telegram.bot_token_env`, `telegram.chat_id_env` | Same | Optional. A file key has priority. |
| `telegram.parse_mode` | (removed) | Always `HTML`. |
| `telegram.notify_on_start`, `telegram.notify_on_no_changes` | (removed) | `updatecsv.py` does not use them. |
| `routers[].password` | `routers[].password_file` | Put the password in a file. |
| `routers[].dhcp.servers[].download_limit_mbps` | `default_plan_down_mbps` | Same meaning. `import-json` uses the value with priority. |
| `routers[].dhcp.dhcp_server` (old list) | `[[routers.dhcp.servers]]` | `import-json` converts it. |
| Root `preserve_network_config`, `flat_network`, `no_parent` | `[network]` | `import-json` converts them. |
| (none) | `libreqos.target`, `routers[].tls`, `[router_access]`, `[speed]`, `[nodes]`, `[circuits]` | New |
| (none) | `runtime.static_comment`, `runtime.subscriber_grace`, `routers[].dhcp.require_bound` | New |

The planned command `lqosync3 import-json <config.json>` will write a TOML file and secret files from an old `config.json`.
