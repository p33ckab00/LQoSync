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
| `lqosync3 tui` | Opens the terminal view: ShapedDevices, Network, Settings, Journal, Dashboard. Refer to "Terminal view". |

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
- Click a tab, or push `1` to `5`, or `Tab`.

| Tab | Keys |
|-----|------|
| 1 ShapedDevices | `/` search (name, ID, MAC, IP), `o` next sort column, `i` reverse order, `Enter` open the parent node |
| 2 Network | `Space` open or close a node, `e` expand all, `c` collapse all, `/` search |
| 3 Settings | Arrows select (`Left`/`Right` change the panel), `Enter` or `Space` edit, `r` reset to the default, `u` undo, `P` new secret, `+` add, `-` remove, `v` preview, `A` apply. Refer to "Change the configuration in the TUI". |
| 4 Journal | `l` level filter, `/` search, `f` follow, `Shift`+arrows or the mouse select lines, `y` copy, `s` select mode |
| 5 Dashboard | `S` sync now (asks first, releases a guard hold), `R` reload the config |

- `S` sends `SIGUSR1` and `R` sends `SIGHUP` to the MainPID of `tui.service_unit`. The TUI first checks that the process name is `lqosync3`. The result comes from `status.json`, not from the signal. If no result comes in `tui.apply_verify_timeout` seconds, the footer shows "no result from the service".
- `y` copies with OSC 52. The terminal does not show if the copy arrived. If the text does not arrive, push `s` (select mode): the mouse goes to the terminal, and the normal terminal selection operates. Push `s` again to end. `tui.osc52 = false` turns OSC 52 off.
- The Journal tab can read the journal only as root, or as a member of the group `adm` or `systemd-journal`. If not, the tab shows the `journalctl` hint in red.
- An unknown value shows `unknown`, never `0`. A file error shows in red in the footer.
- The Process panel of the Dashboard has one chart with four lines: CPU, RSS, disk I/O, and network of the service. CPU is in percent of one core. The other lines are in percent of their peak in the chart window. The line above the chart gives the current values. In the `mono` theme, each line has its own marker.
- Disk I/O and network come from the systemd accounting of `tui.service_unit`. The unit file of the tool turns it on. If the values show `unknown`, add the accounting to an installed unit:

  ```bash
  sudo mkdir -p /etc/systemd/system/lqosync3.service.d
  printf '[Service]\nIOAccounting=yes\nIPAccounting=yes\n' | sudo tee /etc/systemd/system/lqosync3.service.d/accounting.conf
  sudo systemctl daemon-reload
  sudo systemctl restart lqosync3.service
  ```

  The counters include the child processes of the service, for example `LibreQoS.py`. The network counter includes the router API and Telegram traffic.
- Each dialog shows its keys on its bottom border, and the footer shows them too. Only `y` confirms. Only `n` or `Esc` cancels. Other keys do nothing.
- `q` or `Ctrl+C` stops the view. With settings changes that are not applied, the view asks first.

## Change the configuration in the TUI

The Settings tab edits `/etc/lqosync3/lqosync3.toml`. The comments and the key order of the file stay. A reset removes the key from the file, so the default of `config/defaults.toml` applies. Root is necessary.

1. Start the view: `sudo lqosync3 tui`. Push `3`.
2. Select a group on the left, then a key in the center (`Right`). The help is on the right.
3. Push `Enter` or `Space`:
   - A boolean changes at once.
   - A key with fixed values opens a list. Select the value and push `Enter`.
   - A number or a text opens a field. Push `Enter` to keep the value, `Esc` to cancel. An error names the key.
   - A list opens a list editor: `a` add, `d` delete, `K` up, `J` down.
4. Routers: push `+` to add a router (name, address, user, password). The password goes to `/etc/lqosync3/secrets/<name>.password` at apply. Push `-` in a router group to remove a router. Its password file stays: delete it by hand.
5. DHCP servers: push `+` in the `dhcp` group of a router to add a server. Push `-` in a server group to remove it.
6. DHCP clients: the server groups of a router show the leases of the last cycle (from `snapshot.json`). Push `Enter` on a lease to set its download and upload max rates: this makes a `[[routers.dhcp.clients]]` entry. Push `r` on the lease to remove the override. A grey lease is not shaped, and its line gives the reason. The group `router <name> dhcp clients` shows each override, and "no active lease" if the client is not on the router now.
7. Push `P` in a router group to set a new router password. Push `P` in `telegram` to set a new bot token. The screen shows only `*`.
8. Push `v`. The yellow "next" bar (header and right panel border) always shows the next step: `[v] preview`, `preview running`, `[A] apply`, or `apply blocked`. The TUI checks the file with the rules of `lqosync3 check`. Then it runs the cycle core on `<state_dir>/snapshot.json` two times: with the current config, and with the new config. The right panel shows the changed lines of the file, the changed circuits and nodes, and the warnings.
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
