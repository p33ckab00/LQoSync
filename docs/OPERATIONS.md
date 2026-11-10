# Operations

This document gives the operational rules and procedures for LQoSync3.

> [!warning] Status: LibreQoS 2.x only
> The binary exists for LibreQoS 2.x (`v2_pair`). LibreQoS v3 and the installer are not done yet. Test in `shadow` mode before the cutover.

## Operational rules

1. Operate only one sync tool in `apply` mode on a shaper. Two tools that write the same LibreQoS files make conflicts.
2. Start each new installation in `shadow` mode.
3. Compare the shadow output with the live files before the cutover.
4. Keep a backup of the live LibreQoS files before each cutover or upgrade.
5. Use a read-only RouterOS user. The tool does not write to routers.
6. Keep passwords and tokens in `/etc/lqosync3/secrets/`. Do not put them in Git, in notes, or in tickets.
7. Do not edit the LibreQoS files by hand while the tool operates in `apply` mode. The next cycle replaces the files.
8. Do a LibreQoS upgrade only in a maintenance window.

## Command reference

All commands take `--config <path>`. The default is `/etc/lqosync3/lqosync3.toml`.

| Command | Effect |
|---------|--------|
| `lqosync3 check` | Reads and validates the TOML file and the secret files. Checks the folders. Makes no router connection. |
| `lqosync3 once` | Does one cycle in the configured mode. In `shadow` mode it writes only `<state_dir>/candidate/`. |
| `lqosync3 once --force` | The same, and it releases a removal guard hold. |
| `lqosync3 diff` | Compares `<state_dir>/candidate/` with the live LibreQoS files: circuits and nodes added, removed, and changed. It operates only in `shadow` mode. If there are no candidate files, it stops with an error. Run `once` in `shadow` mode first. |
| `lqosync3 run` | Operates as a service. The systemd unit uses this command. |
| `lqosync3 defaults` | Prints all settings with their default values. |
| `lqosync3 detect` | Prints the target that `libreqos.target = "auto"` selects, and the reason. |
| `lqosync3 tui` | Opens Dashboard and the seven-page terminal console. Refer to "Terminal view". |

Signals to `lqosync3 run`: `systemctl reload lqosync3` (`SIGHUP`) reads the config again. A bad config keeps the old one. `systemctl kill -s USR1 lqosync3` starts a cycle now and releases a guard hold.
After each cycle, `<state_dir>/status.json` (version 3) shows the router states, the changes, and any error. It also shows `config_path`, `config_digest`, `cycle_seq`, `pid`, `trigger` (`start`, `timer`, `sync_now`, `reload`, `once`), `program_version`, and `telegram` (the last send: `disabled`, `none`, `ok`, or the error).
On LibreQoS v3, `status.json` also shows `native` (the native probe: `ready`, `blocked: <reason>`, or `error: <reason>`) and `bridge`.
`bridge` holds the load counters of the DPDK bridge from its journal: `budget_defers`, `iter_gt100ms`, `iter_gt1s` and `iter_max_ms`.
The bridge worker polls the NIC, so its CPU is always 100%. Use these counters, not the CPU, to see the load.
The counters start again at 0 when the bridge starts. If `budget_defers` or `iter_gt100ms` goes up, Telegram sends one alert (`notify_on_error`). The Dashboard shows both lines.
After each reload signal, the service writes `status.json` at once with `trigger = "reload"` and the `config_digest` of the loaded file. If the new config is bad, `config_error` shows the error until a good reload, and the old config stays. Then a cycle starts.
`lqosync3 check` gives a warning if the config file on disk is not the loaded config (the digests are different). Send `SIGHUP` to load it.
After each router read, the service writes `<state_dir>/snapshot.json` (mode `0600`). It holds the router tables of the last cycle, and no secret. Plan 4b uses it for the settings preview.

Each command returns `0` if it succeeds. It returns a different value if it fails.

## Terminal view

Start the terminal view on the shaper:

```bash
sudo lqosync3 tui
```

- Without `sudo`, the view is read-only. The header shows `READ-ONLY (not root)`, and `S` and `R` do not operate.
- The view reads the files and `status.json` each `tui.refresh_ms`. It does not change the service. It does not restart the service.
- Click a page or push `1` to `7`. Tab focuses controls within adapted screens. Use the sidebar or direct keys to change pages.

| Page | Keys |
|---|---|
| 1 Dashboard | `r` refresh cached sources, `w` chart window, arrows scroll, `S` confirmed sync, `R` reload |
| 2 Subscribers | `/` search, `s/r/t` source/router/state, `o/i` sort/reverse, `e` details, `n` parent Network view, `c` copy |
| 3 Routers | `/` search, `t` state, `x` clear, `d` details, `s` Subscribers, `e` Settings, `c` endpoint copy |
| 4 LibreQoS | `p` preview/back, `s` confirmed sync, Tab source file, `i` issues, arrows selection/scroll |
| 5 Workers | `/` worker search, `t` sample filter, `x` clear, `e` details, `c` evidence copy |
| 6 Settings | Arrows select, Enter/Space edit, `g` next nested section, `r` reset, `u` undo, `P` secret, `+/-` add/remove, `v` preview, `A` apply |
| 7 Journal | `/` search, `l` minimum level, `x` clear search, `f` follow, `e` details, `y/c` original-text copy, `s` terminal selection |

Network is a parent-node detail view from Subscribers. Escape returns to Subscribers.
Enabled controls accept clicks. Overflowing panels accept wheel input, scrollbar track clicks, and thumb drags.
Use Shift+arrows or mouse drag to select Journal records. Retention can expire a paused selection; copy then requires a new selection.
Journal levels use explicit message prefixes. Unknown prefixes show UNKNOWN, not an inferred journal priority.

- `S` sends `SIGUSR1` and `R` sends `SIGHUP` to the MainPID of `tui.service_unit`. The TUI first checks that the process name is `lqosync3`. The result comes from `status.json`, not from the signal. If no result comes in `tui.apply_verify_timeout` seconds, the footer shows "no result from the service".
- `y` copies with OSC 52. The terminal does not show if the copy arrived. If the text does not arrive, push `s` (select mode): the mouse goes to the terminal, and the normal terminal selection operates. Push `s` again to end. `tui.osc52 = false` turns OSC 52 off.
- The Journal tab can read the journal only as root, or as a member of the group `adm` or `systemd-journal`. If not, the tab shows the `journalctl` hint in red.
- An unknown value shows `unknown`, never `0`. A file error shows in red in the footer.
- Dashboard shows five summary cards, LibreQoS DPDK Traffic, dataplane health, planned WAN, and alerts.
- Traffic comes from actual aggregate analytics byte counters. It requires two valid producer samples and a deployed monitor that writes history. Missing/stale data stays unavailable. Process/systemd network totals are not used for this chart.
- Each dialog shows its keys on its bottom border, and the footer shows them too. Use `y` or Confirm to proceed. Use `n`, Esc, or Cancel to cancel. Other keys do nothing.
- `q` or `Ctrl+C` stops the view. With settings changes that are not applied, the view asks first.

## Cycle collection and subscriber state

Dashboard, Subscribers, and Routers read the existing sync cycle's RouterOS observations.
There is no new background poll. The active/idle scan intervals still control collection; Refresh only rereads local files.
`collection.json` in `runtime.state_dir` is a bounded, atomic public cache. It contains names, IP/MAC identity, source, state, and timestamps.
It omits passwords, tokens, comments, and arbitrary RouterOS fields. The raw Settings preview snapshot remains private.

PPPoE state joins local account inventory with active sessions. Active RADIUS users are included without requiring a local secret.
Hotspot state joins active sessions with local account names and disabled flags when that source is enabled.
The Hotspot inventory read is optional: failure leaves account totals unknown and does not stop shaping reads.
Offline totals cover known local accounts, not an external RADIUS account database.
DHCP rows show BOUND, UNBOUND, or UNKNOWN from lease status. BOUND does not prove current device reachability.
The semantics follow [MikroTik PPP](https://manual.mikrotik.com/docs/authentication-authorization-accounting/ppp-aaa/),
[Hotspot](https://manual.mikrotik.com/docs/authentication-authorization-accounting/hotspot-captive-portal/), and
[DHCP](https://help.mikrotik.com/docs/spaces/ROS/pages/24805500/DHCP).

Subscribers includes collected accounts that have no shaping row. These show `unmatched`, unassigned shaping IDs, and no shaping parent. Ambiguous identities also stay unmatched.
This view never inserts those accounts into LibreQoS inputs. Shaping previews and removal/grace rules remain separate.
State filters include Online, Offline, Bound, Unbound, Disabled, and Unknown; Unknown also includes stale evidence.
The status strip shows cycle age and lease/disabled counts. The Online/Offline cards show known session/account counts.
Failed routers, an expired cache, or an older process/cycle cannot supply live states. A failed read never implies disconnected subscribers.
Disabled sources show OFF/DISABLED, not zero users. Enable a source through Settings only when its collection/shaping is intended.

## Change the configuration in the TUI

The Settings tab edits `/etc/lqosync3/lqosync3.toml`. The comments and the key order of the file stay. A reset removes the key from the file, so the default of `config/defaults.toml` applies. Root is necessary.

1. Start the view: `sudo lqosync3 tui`. Push `6`.
2. Select a group on the left, then a key in the center (`Right`). The help is on the right.
3. Push `Enter` or `Space`:
   - A boolean changes at once.
   - A key with fixed values opens a list. Select the value and push `Enter`.
   - A number or a text opens a field. Push `Enter` to keep the value, `Esc` to cancel. An error names the key.
   - A list opens a list editor: `a` add, `d` delete, `K` up, `J` down.
4. Routers: select the Routers category and push `+`. Fill the name, address, port, TLS, username, masked password/file, and source switches. Advanced fields include the CA file. Add to draft validates the complete trial configuration first. The password goes to `/etc/lqosync3/secrets/<name>.password` at apply. Push `-` in a router group to remove a router. Its password file stays: delete it by hand.
5. DHCP servers: push `+` in the `dhcp` group of a router to add a server. Push `-` in a server group to remove it.
6. DHCP clients: the server groups of a router show the leases of the last cycle (from `snapshot.json`). Push `Enter` on a lease to set its download and upload max rates: this makes a `[[routers.dhcp.clients]]` entry. Push `r` on the lease to remove the override. A grey lease is not shaped, and its line gives the reason. The group `router <name> dhcp clients` shows each override, and "no active lease" if the client is not on the router now.
7. Push `P` in a router group to set a new router password. Push `P` in `telegram` to set a new bot token. The screen shows only `*`.
8. Push `v`. The review panel shows the next step: `[v] preview`, `preview running`, `[A] apply`, or `apply blocked`. The TUI checks the file with the rules of `lqosync3 check`. Then it runs the cycle core on `<state_dir>/snapshot.json` two times: with the current config, and with the new config. The right panel shows the changed lines of the file, the changed circuits and nodes, and the warnings.
9. Read the warnings. Examples:
   - an old `snapshot.json`,
   - a router with no snapshot,
   - a source that the last cycle did not read,
   - circuits that the removal guard holds. The next cycle removes them.
10. Push `A` and read the confirmation. Push `y` to apply, or `n` or `Esc` to cancel. Other keys do nothing. The TUI:
   - stops if another person changed the file after the start of the edit,
   - copies the old file to `<state_dir>/config-backups/<unix seconds>.toml` (a second apply in the same second gets `-1`, `-2`; it keeps `runtime.keep_backups` files),
   - writes the secret files (mode `0600`, owner root), then the config file,
   - sends `SIGHUP` to the service.
11. Wait for the result in the header. "applied" shows only when `status.json` has the digest of the new file. "not applied" shows the `config_error` of the service: the old config stays. "not verified" shows the two digests.

NOTE: `runtime.state_dir` is read-only in the TUI. To change it, stop the service, copy the folder, and edit the file by hand.

NOTE: The TUI never restarts the service. A config file with a TOML error opens read-only: fix it by hand.

## Daily tasks

1. Examine the service state:

   ```bash
   systemctl status lqosync3.service
   ```

2. Examine the recent log:

   ```bash
   journalctl -u lqosync3.service --since "1 hour ago"
   ```

3. Find the errors:

   ```bash
   journalctl -u lqosync3.service -p warning --since today
   ```

Each cycle writes one summary line. The line has the counts of added, updated, and removed circuits, the time of each router read, and the result of the apply step.

## Change the configuration by hand

You can edit `/etc/lqosync3/lqosync3.toml` with a text editor instead of the TUI. Read these facts first:

- The service does not watch the file. It uses the new file only after a reload (`SIGHUP`) or a restart.
- At a reload, a bad file does not stop the service. The old configuration stays, and `status.json` shows `config_error`. The Dashboard shows the error.
- At a restart, a bad file stops the service. systemd tries again each 10 seconds (`Restart=on-failure`), and no cycle runs until you correct the file.
- Until the reload, `lqosync3 check` and the Dashboard tell you that the file on disk is not the loaded configuration.
- If the TUI Settings tab is open during your edit, the TUI finds the change. It shows "the file on disk changed" and does not apply its draft over your edit.

A manual edit does not give the protections of the TUI:

- no preview of the changed circuits and nodes, and no removal guard warnings. The TUI preview starts from the file on disk, so it cannot show a change that is already in the file. For a preview, make the change in the TUI.
- no backup in `<state_dir>/config-backups/`,
- no secret files. A password file that you write must have mode `0600` and owner `root`.

WARNING: DO NOT USE A RESTART AFTER A MANUAL EDIT. A BAD FILE THEN STOPS ALL SYNC CYCLES. USE A RELOAD.

Procedure:

1. Make a copy of the file:

   ```bash
   sudo cp /etc/lqosync3/lqosync3.toml /etc/lqosync3/lqosync3.toml.bak
   ```

2. Edit the file:

   ```bash
   sudo nano /etc/lqosync3/lqosync3.toml
   ```

3. Check the file:

   ```bash
   sudo lqosync3 check
   ```

   Expected result: no `ERROR` line. If an `ERROR` line shows, correct the file and do this step again. Do not go to step 4 with an error.

4. Load the file:

   ```bash
   sudo systemctl reload lqosync3.service
   ```

5. Make sure that the service loaded the file. On the Dashboard, `config error` must be `none` and `config` must be `loaded`. Or read `status.json`:

   ```bash
   sudo lqosync3 check
   ```

   Expected result: no line "the config file on disk is not the loaded config".

To go back to the old file: copy `lqosync3.toml.bak` back to `lqosync3.toml`, then do steps 3 and 4 again.

## Cutover from the Python service

Do this procedure one time, when you replace `updatecsv.py`.

> [!caution]
> Make a backup of the live files before step 4. If not, you cannot go back to the last good state.

1. Install the tool in `shadow` mode. Refer to [INSTALLATION.md](INSTALLATION.md).
2. Let the tool operate for a minimum of 24 hours.
3. Compare the candidate files with the live files:

   ```bash
   diff <(sort /opt/libreqos/src/ShapedDevices.csv) <(sort /var/lib/lqosync3/candidate/ShapedDevices.csv)
   diff <(jq -S . /opt/libreqos/src/network.json) <(jq -S . /var/lib/lqosync3/candidate/network.json)
   ```

   Expected result: no difference, or only differences that you know about. The circuit IDs of new subscribers can be different.

4. Make a backup of the live files:

   ```bash
   sudo install -d -m 0700 /var/backups/lqosync
   sudo cp -a /opt/libreqos/src/ShapedDevices.csv /opt/libreqos/src/network.json /var/backups/lqosync/
   ```

5. Stop and disable the Python service:

   ```bash
   sudo systemctl disable --now lqosync-updatecsv.service
   ```

6. Set `runtime.mode = "apply"` in `/etc/lqosync3/lqosync3.toml`.
7. Restart the tool:

   ```bash
   sudo systemctl restart lqosync3.service
   ```

8. Read the log of the first cycle. Make sure that the apply result is `applied` or `unchanged`.
9. Make sure that LibreQoS shapes the circuits. Use the LibreQoS web UI or `lqtop`.

## Rollback to the Python service

1. Stop the tool:

   ```bash
   sudo systemctl disable --now lqosync3.service
   ```

2. If the live files are not correct, restore the backup:

   ```bash
   sudo cp -a /var/backups/lqosync/ShapedDevices.csv /var/backups/lqosync/network.json /opt/libreqos/src/
   ```

3. Start the Python service:

   ```bash
   sudo systemctl enable --now lqosync-updatecsv.service
   ```

4. On LibreQoS 2.x, apply the restored files:

   ```bash
   sudo /opt/libreqos/src/LibreQoS.py --updateonly
   ```

## LibreQoS upgrade from 2.2 to 3.x

The tool supports both versions. The upgrade changes the output target. Do this procedure in a maintenance window.

> [!caution]
> LibreQoS 3.x is in beta. The `libreqos_network.json` format has no public documentation. Test the upgrade on a test shaper first.

1. Make a backup of the live files. Use step 4 of the cutover procedure.
2. Run the LibreQoS v3 preflight:

   ```bash
   sudo libreqos-v3-preflight --pair-dir /opt/libreqos/src
   ```

   Expected result: `PASS` and `topology: validated source=legacy-pair`.

3. Set the tool to `shadow` mode. Restart the tool.
4. Upgrade LibreQoS. Follow the LibreQoS upgrade instructions.
5. Select the v3 topology source in `/etc/libreqos/lqos.conf`:
   - For the first upgrade, keep `source = "shaped_devices"`. The tool then uses `v3_pair`, which is the same file format as v2.
   - After a stable period, set `source = "libreqos_network"`. The tool then uses `v3_network`.
6. Set `compile_mode = "full"` in the `[topology]` section if you use a tree mode.
7. Make sure that the tool selected the expected target:

   ```bash
   sudo lqosync3 detect
   ```

8. Run one cycle in `shadow` mode and read the result:

   ```bash
   sudo lqosync3 once     # in shadow mode: writes only <state_dir>/candidate/
   sudo lqosync3 diff
   ```

9. Set the tool to `apply` mode. Restart the tool.
10. Make sure that `lqosd` accepted the files. Read the v3 topology status in the LibreQoS web UI, or read `/opt/libreqos/state/topology/topology_runtime_status.json`.

> [!note]
> In v3, `LibreQoS.py --updateonly` does nothing. The tool does not start it on v3. `lqosd` reads the changed files by itself.

## Router TLS with your own CA

Use this procedure to connect to a router on `api-ssl` (port 8729) with a certificate check. A MikroTik router has no certificate from a public CA. Thus you make a CA on the router and sign the API certificate with it. Then you give the CA file to the tool.

Do this procedure one time for each router.

### The values

Find these values before you start. Write them down. Do not mix them up.

| Name in the commands | What it is | How to find it |
|---|---|---|
| `R1` | The `name` of the router in the tool config. | The group `router <name>` in the TUI Settings tab, or `name =` in `[[routers]]`. |
| `ROUTER_IP` | The address that the shaper uses to connect to the router. It is the `address` of the router in the tool config. | The key `address` of the router in the TUI. It must be the same text, character for character. |
| `SHAPER_IP` | The source address of the shaper, as the router sees it. | On the shaper, run `ip route get ROUTER_IP`. The value after `src` is `SHAPER_IP`. If NAT is between the shaper and the router, use the NAT address. |

Example: the router has `address = "10.10.0.1"`. On the shaper, `ip route get 10.10.0.1` shows `10.10.0.1 dev eth1 src 10.10.0.5`. Then `ROUTER_IP` is `10.10.0.1` and `SHAPER_IP` is `10.10.0.5`.

NOTE: The tool checks the certificate against `address`. If `address` is an IP address, the certificate must have `IP:ROUTER_IP`. If `address` is a DNS name, use `DNS:<the name>` in step 3.

### Procedure

On the router (RouterOS 7 terminal, with an administrator account):

1. Make the CA:

   ```
   /certificate add name=lqos-ca common-name=lqos-ca key-usage=key-cert-sign,crl-sign days-valid=3650
   /certificate sign lqos-ca
   ```

2. Make sure that the CA has the flags `K`, `L`, and `T`: `/certificate print`.
3. Make the API certificate. Put `ROUTER_IP` in both places:

   ```
   /certificate add name=api-ssl common-name=ROUTER_IP subject-alt-name=IP:ROUTER_IP days-valid=3650
   /certificate sign api-ssl ca=lqos-ca
   ```

4. Set the certificate on the API service. Put `SHAPER_IP` in `address`. Only the shaper can then use the API:

   ```
   /ip service set api-ssl certificate=api-ssl address=SHAPER_IP/32 disabled=no
   ```

   CAUTION: MAKE SURE THAT `SHAPER_IP` IS CORRECT. A WRONG ADDRESS STOPS ALL ROUTER READS OF THE TOOL. WinBox and SSH do not change.

5. Export the CA certificate (not its key):

   ```
   /certificate export-certificate lqos-ca
   ```

   The router writes the file `cert_export_lqos-ca.crt` in Files.

On the shaper:

6. Copy the CA file from the router to the shaper:

   ```bash
   scp admin@ROUTER_IP:cert_export_lqos-ca.crt /tmp/R1-ca.crt
   sudo install -d -m 0755 /etc/lqosync3/ca
   sudo install -m 0644 /tmp/R1-ca.crt /etc/lqosync3/ca/R1.crt
   rm /tmp/R1-ca.crt
   ```

7. Test the TLS connection before you change the config:

   ```bash
   openssl s_client -connect ROUTER_IP:8729 -CAfile /etc/lqosync3/ca/R1.crt -verify_ip ROUTER_IP -brief </dev/null
   ```

   Expected result: `Verification: OK`.

8. In the Settings tab of the TUI, select the group `router R1`. Set these keys:
   - `tls = true`
   - `port = 8729`
   - `tls_verify = true`
   - `tls_ca_file = "/etc/lqosync3/ca/R1.crt"`
9. Push `v`, read the preview, then push `A` and `y`.
10. Wait for "applied" in the header. The next cycle reads the router with TLS. Make sure that the Dashboard shows the router as `online`.

### Problems

| Error in the log or in openssl | Cause | Action |
|---|---|---|
| `NotValidForName`, or openssl `IP address mismatch` | The certificate does not have `IP:ROUTER_IP`, or `address` is a different text. | Do step 3 again with the `address` of the config. Then do step 4. |
| `UnknownIssuer`, or openssl `unable to get local issuer certificate` | `tls_ca_file` is not the CA that signed `api-ssl`. | Do steps 5 and 6 again. |
| Timeout or `Connection refused` on 8729 | `api-ssl` is off, or `address` of the service is not `SHAPER_IP`. | Run `/ip service print` on the router. Do step 4 again. |
| `check`: `tls_ca_file ... cannot be read` | Wrong path, or the file is missing. | Do step 6 again. |

## Turn on Telegram

1. Write the bot token to the secret file:

   ```bash
   sudo install -m 0600 /dev/null /etc/lqosync3/secrets/telegram.token
   sudo nano /etc/lqosync3/secrets/telegram.token
   ```

2. In `/etc/lqosync3/lqosync3.toml`, set:

   ```toml
   [telegram]
   enabled = true
   chat_id = "<chat ID>"
   ```

3. Check the configuration:

   ```bash
   sudo lqosync3 check
   ```

   Expected result: `OK    telegram: the bot token is readable`.

4. Reload the service:

   ```bash
   sudo systemctl reload lqosync3.service
   ```

## Backups that the tool keeps

In `apply` mode, the tool makes a backup of the live files before it replaces them. Each backup is a folder in `/var/lib/lqosync3/applied/`. The folder name is the Unix time in seconds, with 12 digits.

- The tool keeps the newest `runtime.keep_backups` folders (default 10).
- The tool removes only folders with a 12-digit name. It never removes the backup that it just made.
- Other folders in `applied/` stay. You can keep a manual copy there.

The tool writes `network.json` before `ShapedDevices.csv`. Each new file gets the mode and the owner of the old file.

## Troubleshooting

| Symptom | Possible cause | Action |
|---------|----------------|--------|
| `check` fails with `unknown key` | Spelling error in the TOML file | Correct the key name. |
| `check` fails with a secret file error | Mode is not `0600`, or the owner is not `root` | Run `sudo chmod 0600` and `sudo chown root:root` on the file. |
| A router read fails with a timeout | Firewall, API service off, or a slow router | Make sure that `/ip service` has `api-ssl` or `api` on. Test the port with `nc -vz <address> 8729`. |
| `detect` shows `target=none` | v3 uses a built-in integration as the topology source | Set `[topology] source` to `shaped_devices` or `libreqos_network`, or stop the tool. |
| Apply result is `rejected` | The validator found a duplicate IP, a missing parent, or a bad rate | Read the log line. Correct the router data. |
| v3 keeps the old topology | `lqosd` rejected `libreqos_network.json` | Read `journalctl -u libreqos-lqosd.service`. Look for `libreqos_network.json rejected`. |
| Circuits do not use the node tree on v3 | `compile_mode` is not `full` | Set `compile_mode = "full"` in `/etc/libreqos/lqos.conf`. |
| Reload fails on a new LibreQoS with an error about `ShapedDevices.lastLoaded.csv` | `--updateonly` ran before the first full run of LibreQoS | The tool does a full run when that file does not exist in `libreqos.state_directory`. Make sure that `libreqos.state_directory` is the LibreQoS state directory. |
| No change after a router change | The output hash did not change | Run `once` in `shadow` mode and `diff`. Make sure that the router data changed. |
## Worker utilization

Open `lqosync3 tui` and press `5` for Workers.
Use Up, Down, Page Up, Page Down, Home, and End to scroll.
The redesign starts on Dashboard. The navigation keys are 1 Dashboard, 2 Subscribers, 3 Routers,
4 LibreQoS, 5 Workers, 6 Settings, and 7 Journal.
Wide terminals use a sidebar. Compact terminals retain a header, navigation strip, body, and footer.
Parent-node inspection opens Network detail from Subscribers. Press Escape to return.
Press `T` or click the wide header theme control to change the current view theme.
Persist the default theme through Settings. The header control does not write the configuration.

The table shows the DPDK worker ID, busy percentage, idle percentage, and mean backlog in packets.
Busy excludes empty receive polling. Idle is the complement of busy.
These percentages differ from the operating system CPU percentage of a polling worker.
A missing backlog shows `unknown`. Missing worker fields do not become zero values.

The service reads a snapshot every two seconds from `libreqos.v3_analytics_socket`.
Its default is `/dev/shm/libreqos-analytics.sock`. Match this path to the bridge's analytics socket.
The service normally runs as root and can read the root-only socket.
The TUI reads the public cache at `<runtime.state_dir>/worker-utilization.json`.
This cache contains worker metrics, aggregate traffic history, buffer/ring diagnostics, optional bridge queue totals, and classification traffic.
It contains no host or flow records.

The first frame establishes the producer timestamp. An advancing timestamp enables the live display.
Snapshots and cached values expire after six seconds. A socket error marks the last-good values stale.
An older bridge without the worker fields shows `unavailable`.
Freshness describes the snapshot, not a separate health verdict for each worker.
The Dashboard keeps its journal counters when worker telemetry is unavailable.

This feature does not change worker counts, queue counts, shaping files, or the topology source.
The monitor applies config reloads and stops with the service.
After a binary upgrade, reopen the TUI to use the new tab.

### Buffer and ring pressure

The Workers tab also shows buffer/ring pressure and its causes.
The same two-second poll reads the diagnostic fields. It adds no socket request.
The service journal records each pressure state change once:

```bash
journalctl -u lqosync3.service -g 'buffer/ring pressure' --since '10 minutes ago'
```

The monitor warns when free pool buffers are at or below 10 percent.
It also warns when any handoff ring exceeds 75 percent, or aggregate ring use reaches 75 percent.
These thresholds indicate operational pressure. They do not prove a dataplane failure.
A single-worker topology can have no handoff rings. The display then shows `no rings`.

New RX buffer exhaustion events (`rx_nombuf`) and pool pressure drops also produce warnings.
`New drops` shows increments since the preceding valid sample, normally two seconds earlier.
RX events and pool drops are separate signals. Do not add them as a total packet loss count.
The first sample establishes counter baselines. A decreasing counter shows `unknown` for that interval.
A read failure, producer restart, or sample gap above six seconds establishes a new baseline.
Repeated snapshots do not replay increments. Journal messages do not repeat while the same causes remain active.

Missing fields show `unknown` or `unavailable`. Invalid gauges do not hide valid worker utilization.
Zero pool capacity is unknown. Stale telemetry shows unknown current pressure, never a recovery verdict.
`clear` means that all monitored fields are available and no current threshold or increment indicates pressure.
This feature adds local TUI and journal alerts. It does not change Telegram notification policy.


### Whole-bridge queue statistics

The same two-second analytics poll reads optional `qdisc_totals`.
The public cache adds `queue.totals`, `queue.drop_delta`, and `queue.aqm_delta`.
Older caches and bridges without these fields remain readable. Missing values show `unknown` or `unavailable`, never zero.

- `drop_pkts` is the cumulative queue drop count.
- `aqm_marks` is the cumulative AQM mark count. It is distinct from the drop count.
- `backlog_bytes` is the current aggregate queue backlog in bytes, not a counter increment.
- `New/sample` shows increments since the previous valid producer sample. It is not an event rate per second.

Counter decreases, read errors, producer timestamp regression, or sample gaps above six seconds require a new baseline.
Repeated producer snapshots do not replay increments. Stale displays retain labeled last-known totals but hide current increments.
Do not add queue drops to RX/pool drop counts. Do not interpret AQM marks as packet drops or a complete packet-loss percentage.
These measurements cover the whole bridge. They do not describe one selected worker, subscriber, or WAN.

Dashboard shows the measurements in Dataplane health. Use `j`/`k`, the mouse wheel, or the scrollbar to inspect wrapped lines.
Workers shows them in its existing detail pane. Tab to the detail pane and use Up/Down, or use the mouse wheel/scrollbar.
The detail pane supports expansion. Queue measurements remain separate from the selected worker's mean backlog in packets.
The UI reads the cache and performs no socket requests during rendering.

For read-only source verification, build `cargo build --locked --example dpdk_queue_probe`.
Run `sudo ./target/debug/examples/dpdk_queue_probe` where the socket requires root.
An optional first argument selects a different analytics socket.
The probe prints three timestamped aggregate samples and never prints host or flow identities.
A successful probe does not deploy a service binary or activate the new cache fields.


### Mapped and unmapped traffic

Dashboard retains the Actual delivered chart and adds Mapped and Unmapped views.
Click the source buttons or press `v` to change the chart source.
Time-window controls apply to the selected source. An incomplete window shows `warming`.
Dashboard health and Workers details show class rates and cumulative byte totals.
Workers copy output retains the same scope and stale labels.

The bridge counts `mapped` and `unmapped` bytes at enqueue, before it limits host rows.
Mapped means that the bridge resolved a circuit for the subscriber IP.
Unmapped means best-effort classification. It does not prove unauthorized use or packet loss.
Actual uses delivered bytes. Do not infer unmapped traffic by subtracting mapped bytes from Actual.
Do not sum capped host/flow records to estimate class totals.
These are whole-bridge classes, not WAN-specific or selected-worker measurements.

The same two-second snapshot poll supplies the class counters. It adds no request or router scan.
The public cache adds `classification.mapped` and `classification.unmapped`.
Each class contains optional cumulative `bytes` and a `history` of download/upload Mbps.
The source timestamp supplies the rate interval. Each class retains at most 1,801 points and one hour.
Source errors, resets, and gaps establish new baselines. Duplicate frames do not replay class values.
A missing class does not invalidate the other class or worker data.
Unknown values remain unknown. Fresh evidence of no traffic can show zero.
Stale views retain labeled last-known totals and historical points but hide current rates.

Old caches and sources without class fields remain readable. Their class measurements show unavailable.
A corrupt class history is discarded without hiding valid worker metrics or the other class.
The cache keeps its 1 MiB read limit. Rendering reads cached data and draws only the selected chart history.
No subscriber identity or flow record enters this cache.

Build `cargo build --locked --example dpdk_mapping_probe` for read-only source verification.
Run `sudo ./target/debug/examples/dpdk_mapping_probe` where the analytics socket requires root.
An optional first argument selects a different socket. The probe prints three aggregate samples.
It labels delivered and enqueued counters separately and prints no subscriber identities.
A probe does not deploy a service binary or activate new cache fields.
