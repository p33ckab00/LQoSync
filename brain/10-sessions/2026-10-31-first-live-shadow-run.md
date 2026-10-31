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

## Open items

- After 24 hours (2026-11-01 15:24 UTC, from the binary change), run `sudo lqosync3 diff` again.
- The cutover to `apply` mode needs the operator's approval.
- Plain API: think about `api-ssl` (port 8729) with `tls_ca_file` later.
