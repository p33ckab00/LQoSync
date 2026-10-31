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

Signals to `lqosync3 run`: `systemctl reload lqosync3` (`SIGHUP`) reads the config again. A bad config keeps the old one. `systemctl kill -s USR1 lqosync3` starts a cycle now and releases a guard hold.
After each cycle, `<state_dir>/status.json` shows the router states, the changes, and any error.
If a reload with a bad config fails, `config_error` in `status.json` shows the error until a good reload. A cycle starts immediately after each reload signal, so `status.json` shows the result at once.

Each command returns `0` if it succeeds. It returns a different value if it fails.

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

## Change the configuration

1. Edit `/etc/lqosync3/lqosync3.toml`.
2. Check the file:

   ```bash
   sudo lqosync3 check
   ```

3. Show the changes that the new configuration makes. In `apply` mode, `once` writes the live files. Use a copy of the config with `mode = "shadow"` and a different `state_dir` for this step.

   ```bash
   sudo lqosync3 once     # in shadow mode: writes only <state_dir>/candidate/
   sudo lqosync3 diff
   ```

4. Restart the service:

   ```bash
   sudo systemctl restart lqosync3.service
   ```

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
| No change after a router change | The output hash did not change | Run `once` in `shadow` mode and `diff`. Make sure that the router data changed. |
