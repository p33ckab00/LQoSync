# Subscriber speed rules

These rules resolve subscriber speeds from RouterOS inventory and explicit per-server DHCP policy.
The parser and preview do not change a router or LibreQoS installation.
With selected sources, a healthy complete collection publishes a normalized local snapshot and generation automatically.

## Use the preview

1. Open **Speed rules**. Set the minimum fraction, minimum floor, and profile-name fallback.
2. Save the rules. The app stores them in its own `settings.toml`.
3. Open **Routers**, then select **Read inventory** for a subscriber router.
4. Select **Preview speed rules**. Review the maximum, minimum, source, raw rule, and errors.

Inventory is active only in the current process. Read it again after a restart.
The preview uses a recorded inventory and the current minimum policy.
A concurrent inventory or policy change discards the result. Reload the preview.
A failed latest read shows a warning if an earlier inventory remains available.
This is a rule preview. It does not prove the effective policy of a running router session.
External RADIUS overrides and unknown user mappings need separate collection support.

## Rule order

| Source | First match, in order |
| --- | --- |
| PPP | Secret comment, active comment, profile comment, profile rate, profile-name suffix |
| DHCP Auto | DHCP server comment, server-name speed suffix, configured per-server manual rates |
| Hotspot | Local user comment, profile comment, profile rate |

Comments keep the legacy subscriber download/upload order.
Examples: `Alice|25M/5M`, `Alice Smith 25M/5M`, and `Plan 25M`.
A single comment rate applies in both directions. Use a delimiter or whitespace before a speed token.
Multiple speed rules, malformed pairs, non-finite values, and conflicting duplicates remain unresolved.
The parser does not recover a smaller valid suffix from a malformed numeric token.

A profile-name fallback needs an explicit `k`, `M`, or `G` suffix, for example `Plan25M`.
A name such as `service2026` does not supply a speed.
Disable this fallback if router names do not encode plans.
An explicit invalid or unlimited profile rate never falls through to the profile name.
No local default maximum is assigned to unresolved users.

## DHCP server policy

Open **Speed rules → DHCP server speeds** after a fresh subscriber-router inventory read.
Select the exact router and DHCP server. Choose Auto, Server comment, Server name, or Manual.
Save the server rule. Existing automatic local publication checks run after the save.
Global minimum-policy saves preserve all per-server rules.
Stale server forms cannot overwrite a newer settings revision.

Auto uses these sources in order:

1. Server comment, for example `Site A | 25M/5M`.
2. Server-name suffix, for example `DHCP-25M` or `DHCP-25M/5M`.
3. Configured manual download and upload values.

Each source uses subscriber download/upload order. A single rate is symmetric.
Manual values use Mbps. They apply to each subscriber on that server.
Branch capacity, aggregation factors, and WAN caps remain separate topology controls.

Comment and name rules are orchestration conventions. They do not create RouterOS bandwidth policy.

Auto falls through only when a source contains no speed rule.
Malformed, ambiguous, zero, or unlimited rules remain unresolved.

An explicit mode reads only its selected source. Manual mode can explicitly override router text.
Remove a local rule to restore Auto without a manual fallback.
The app does not guess a speed if no source supplies a finite positive value.

The lookup uses the lease's actual `active-server`, or its configured server when no actual server exists.
A wildcard `server=all` alone cannot identify a source.

Router IDs and server names form the rule key. Server names remain exact, including spaces.
Failed, missing, duplicate, disabled, or unknown server status cannot supply a speed.
The collector reads server comments through the existing print allowlist. It sends no configuration writes.

For Auto or Comment mode, collect fresh inventory after this upgrade so the recorded server metadata includes comments.

```toml
[[speed_rules.dhcp_servers]]
router = "example-distro"
server = "dhcp-site-a"
mode = "auto" # auto, comment, name, manual
download = 25.0
upload = 5.0
```

Without a rule, a server uses Auto and has no manual fallback.
Older application settings remain readable. Existing lease-only plans need a server rule before their next valid publication.

The last valid local generation remains selected while the new speed is unresolved.
Do not run an older binary against settings that contain the new per-server table.
The UI shows the first 200 configured rules and first 200 discovered server choices.
Larger configurations remain available in application-local TOML.

## Router rate fields

RouterOS rate fields use router receive/transmit order.
Router receive is subscriber upload. Router transmit is subscriber download.
Thus `10M/50M` supplies subscriber download 50 Mbps and upload 10 Mbps.
A single rate is symmetric. Bare rate numbers mean bits per second.
For example, `512000/1000000` supplies download 1 Mbps and upload 0.512 Mbps.
The unit multipliers are decimal. The parser also accepts `G` as a gigabit unit.

Burst, threshold, time, priority, and minimum fields receive syntax checks.
The preview exports no burst or priority policy. It reports the sustained maximum and minimum only.
Zero or unlimited maximum rates remain unresolved because they do not define a finite shaping plan.

Explicit router minima take precedence.
Otherwise, each direction uses `min(max(maximum * fraction, floor), maximum)`.
Defaults: fraction `0.5`, floor `0.128` Mbps.
The floor never increases a maximum or makes a minimum exceed it.
Maxima keep the router value. There is no legacy rounding or forced maximum floor.
Node aggregation factors and WAN caps remain separate topology settings.

```toml
[speed_rules]
min_rate_fraction = 0.5
min_floor_mbps = 0.128
profile_name_fallback = true
```

Existing settings without this table continue to load with these defaults.
The old `config.json` is not read or imported by this module.

## Differences from the reference script

PPP keeps the comment precedence from `updatecsv.py`.
Router rate directions and bare-rate units now follow RouterOS semantics.
The legacy script treated the first rate as download and bare numbers as Mbps.
DHCP uses the selected server policy for each bound lease. Lease comments and rates no longer supply subscriber speeds.
Hotspot local default speeds are not copied. The app reads the mapped Hotspot profile instead.
DHCP previews include bound, enabled leases. Missing binding status remains unresolved.
Unknown or disabled local users remain unresolved.
A failed required table cannot supply rates from its partial rows.
Duplicate session keys reject every conflicting entry, independent of input order.

## Performance and integrity

User, profile, and DHCP server joins use hash indexes. Each shared profile or DHCP server is parsed once per preview.
The per-server policy index is built once per router preview. Full policy validation runs once, outside subscriber loops.
Regular expressions are compiled once per process.
The app shares immutable inventories through `Arc`; preview requests do not clone every router row.
Preview computation runs outside the application lock and async worker.
The app shows at most 200 entries and 100 source errors per preview.
Counts include all entries. Raw router table pages show the first 20 rows per table.

Manual measurements cover 50,000 PPP sessions with 100 profiles and 50,000 DHCP leases with 1,000 server rules.
Run it with:

```bash
cargo test --locked --test speeds inventory_50k_performance -- --ignored --nocapture
cargo test --locked --test dhcp_server_speeds preview_50k_dhcp -- --ignored --nocapture
```

This is a local diagnostic measurement. It is not a production latency guarantee.
Source selection and complete snapshot publication are available in [subscriber orchestration](SUBSCRIBERS.md).
Policy saves can update normalized snapshots when every selected source remains fresh and healthy.
Do not use the preview alone as permission to replace subscribers after a partial scan.

## Sources

- [MikroTik PPP AAA](https://manual.mikrotik.com/docs/authentication-authorization-accounting/ppp-aaa/): direction, units, minimum fields, and rate grammar.
- [MikroTik DHCP](https://help.mikrotik.com/docs/spaces/ROS/pages/24805500/DHCP): lease rates and dynamic queues.
- [MikroTik Hotspot](https://help.mikrotik.com/docs/spaces/ROS/pages/56459266/HotSpot%2B-%2BCaptive%2Bportal): local user profiles and rate fields.
- Existing `updatecsv.py` bandwidth helpers and PPP/DHCP/Hotspot processing: read-only compatibility reference.
