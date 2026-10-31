---
type: session
created: 2026-10-31
tags: [session, shadow-run, production]
status: done
related: ["[[HANDOFF]]", "[[ADR-0009-dhcp-discovery-and-exclusions]]"]
---

# 2026-10-31 — First live shadow run

## Goal

Run lqosync3 in `shadow` mode against the live router and compare the result with the files of the Python service.

## What changed

- The operator changed the RouterOS user `libreqos` to the `read` group.
- Binary `/usr/local/bin/lqosync3` (release build of commit `ace5918`).
- Config `/etc/lqosync3/lqosync3.toml`. The values match `/opt/libreqos/src/config.json`: `root_promotion`, root 620/620, `min_rate_percentage = 0.1`, `pppoe_aggregate_factor = 0.35`, DHCP 45 Mbps for each client, backstop caps on (PPPoE 620, DHCP 250).
- Router `FSEHub-Distro`, `10.200.1.2:8728`, plain API (`tls = false`, the same as the Python service). DHCP `discover = false` with the 4 servers of the Python config.
- The password file came from `config.json` with a script that does not show the password.
- `lqosync3.service` started at 14:51 in `shadow` mode.

## Results

- `check`: all OK. One warning for the plain API.
- `once`: router online, 62 rows, 167 ms. Added 0, updated 26, removed 0. The 26 updates were text only (`25.0` to `25`, node ID columns). The next cycle gave updated 0.
- `diff`: 26 circuits in each file. One difference only: `DHCP-libreqos` on server `LibreQoEMgt` changed from 25 to 35 Mbps.
- Cause: the server comment is `35M/35M`. lqosync3 uses the server comment before the settings. The Python service uses only the settings. The new result is correct.
- The router has 4 DHCP servers, the same as the config. `discover = true` gives the same result.

## Telegram (15:24 UTC)

- Installed the Plan 2b binary (commit `056a91e`). The token came from `config.json` with a script that does not show the token.
- `check`: `OK    telegram: the bot token is readable`.
- Test: removed `/var/lib/lqosync3/candidate/network.json`, then sent `SIGUSR1`. The cycle wrote the candidate again, and the message `[shadow] OK: lqosync3 candidate files changed` arrived in the chat.
- Until the cutover, the chat gets messages from two tools: the Python service and lqosync3 (`[shadow]`).

## Cutover to apply (15:35 UTC)

The operator chose the cutover before the 24-hour shadow period ended.

- Backup: `/var/backups/lqosync/20261031T153517Z` (the last Python files).
- Stopped `lqosync3.service`. Stopped and disabled `lqosync-updatecsv.service`. Set `runtime.mode = "apply"`.
- Removed `/var/lib/lqosync3/state.json`, so the first cycle took the circuit IDs from the live files.
- First cycle: `updated 27 | files changed: true | reload: ok`. The next cycle gave updated 0.
- `LibreQoS.py --validate`: `Rust validated ShapedDevices.csv`, `ShapedDevices.csv passed validation`, `network.json passed validation`. LibreQoS 2.2 accepts the 14 columns and the node `id` key (ADR-0005 check done).
- The circuit IDs stayed (for example `TISZWB5U`). The files kept the owner `pi:pi` and the mode `0644`.
- Telegram: `OK: lqosync3 update applied`.
- Operator feedback: the Updated list showed only column names. The 27 updates were format only (`25.0` to `25`) and the new `Parent Node ID`. Fix: compare rates as numbers, and show the values (`old -> new`) and the IPv4, rates and parent of added and removed circuits.

## Open items

- Plain API: think about `api-ssl` (port 8729) with `tls_ca_file` later.
