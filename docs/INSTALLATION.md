# Installation

This document gives the installation from a release, the manual installation from source, the checks after installation, and the removal procedure.

## Requirements

On the shaper (the LibreQoS server):

- Ubuntu 24.04 (Noble) on x86_64.
- LibreQoS 2.0 to 2.2, or LibreQoS 3.x.
- Root access (`sudo`). The service writes the LibreQoS files and reads the secret files.
- For an installation from source: Rust 1.89 or later and `cargo`.

On each MikroTik router:

- RouterOS 7.
- The API service on, and open to the shaper only. Use `api-ssl` (port `8729`, TLS) if possible.
- A RouterOS user for the tool, with the policies `read` and `api` only. The tool never writes to a router.

Refer to "Prepare each router" for the RouterOS commands.

## The values in the commands

The commands use these names. Replace each name with your value. Do not mix them up.

| Name | What it is | How to find it |
|---|---|---|
| `R1` | The name of the router in the tool config (`name` in `[[routers]]`). Use letters, digits, `-`, and `_`. | You select it. The secret file name uses it. |
| `ROUTER_IP` | The address that the shaper uses to connect to the router. | You put it in `address` of the router in the config. |
| `SHAPER_IP` | The source address of the shaper, as the router sees it. | On the shaper, run `ip route get ROUTER_IP`. The value after `src` is `SHAPER_IP`. If NAT is between them, use the NAT address. |
| `lqosync` | The RouterOS user and group of the tool. | You select it. Use the same name in `username` of the config. |

Example: `ip route get 10.10.0.1` shows `10.10.0.1 dev eth1 src 10.10.0.5`. Then `ROUTER_IP` is `10.10.0.1` and `SHAPER_IP` is `10.10.0.5`.

## Prepare each router

Run these commands in the RouterOS terminal (WinBox terminal or SSH) with an administrator account. Do this one time for each router.

1. Make a group that can only read, through the API:

   ```
   /user group add name=lqosync policy=read,api comment="lqosync3: read-only API"
   ```

   The group has no `write`, `ssh`, `winbox`, or `sensitive` policy. The tool cannot change the router and cannot read the PPP passwords.

2. Make the user. It can log in only from the shaper:

   ```
   /user add name=lqosync group=lqosync address=SHAPER_IP/32 password="<a long random password>"
   ```

   Keep the password for step 5 of the manual installation.

3. Open the API service to the shaper only. Use one of these:
   - TLS (recommended). It needs a certificate. Refer to "Router TLS with your own CA" in [OPERATIONS.md](OPERATIONS.md):

     ```
     /ip service set api-ssl address=SHAPER_IP/32 disabled=no
     ```

   - Plain API (port `8728`). The password goes over the network in clear text. `lqosync3 check` gives a warning:

     ```
     /ip service set api address=SHAPER_IP/32 disabled=no
     ```

   CAUTION: MAKE SURE THAT `SHAPER_IP` IS CORRECT. A WRONG ADDRESS STOPS ALL READS OF THE TOOL. WinBox and SSH do not change.

4. On the shaper, make sure that the port is open: `nc -vz ROUTER_IP 8729` (or `8728` for the plain API). Expected result: `succeeded`.

The tool reads only these menus, and only these fields:

| Menu | Fields | Used for |
|---|---|---|
| `/ppp/secret` | `name`, `profile`, `comment`, `disabled`, `caller-id` | PPPoE subscribers and their plans |
| `/ppp/active` | `name`, `address`, `caller-id`, `comment` | The IP of each online PPPoE subscriber |
| `/ppp/profile` | `name`, `comment`, `rate-limit` | The speed of each PPPoE plan |
| `/ip/hotspot/active` | `user`, `address`, `mac-address` | Hotspot subscribers |
| `/ip/dhcp-server` | `name`, `comment`, `disabled` | DHCP servers and their speed comments |
| `/ip/dhcp-server/lease` | `server`, `address`, `active-address`, `mac-address`, `host-name`, `comment`, `status` | DHCP subscribers |

The tool reads a menu only if its source (`pppoe`, `hotspot`, or `dhcp`) is on for that router.

## Installation from a release

Use this procedure on the shaper. It installs the release binary. It needs no clone and no Rust. The installer also upgrades an installed tool. Refer to [RELEASE.md](RELEASE.md) for the releases, the versions, and the rollback.

1. Prepare each router first (refer to "Prepare each router").
2. Download the installer, read it, then run it:

   ```bash
   curl -fsSL -o /tmp/install.sh https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync3/install.sh
   less /tmp/install.sh
   sudo bash /tmp/install.sh
   ```

3. The installer writes `/etc/lqosync3/lqosync3.toml` with `mode = "shadow"` and no router. Thus `lqosync3 check` fails, and the service does not start yet.
4. Add the routers in the TUI: `sudo lqosync3 tui`, tab 3, key `+`. Then push `v`, `A`, and `y`.
5. Start the service:

   ```bash
   sudo systemctl enable --now lqosync3.service
   ```

6. Do the "Checks after installation".

> [!note]
> The tool is at the root of the branch `lqosync3` of `p33ckab00/LQoSync`. The branch has no shared history with the other branches.
> The file `install.sh` of the other branches is a different installer. Do not use it for this tool.

### Installer options

| Option | Default | Effect |
|--------|---------|--------|
| `--version v3.0.N` | the newest `v3` release | Installs this release. |
| `--mode shadow\|apply` | `shadow` | Sets `runtime.mode` in a new config file. The installer does not change an existing config file. |
| `--no-start` | off | Installs the files, but does not start or restart the service. |

The installer has no import of the Python `config.json`. Write the TOML file by hand, or use the TUI.

### What the installer does

1. It stops if the user is not `root`.
2. It stops if the operating system is not Ubuntu 24.04, or the CPU is not x86_64.
3. It reads the LibreQoS version with `dpkg-query -W libreqos`. It stops if the version is not 2.0 to 2.2 or 3.x.
4. It downloads the release binary, its SHA-256 file, and `lqosync3.service`. It stops if the checksum is not correct.
5. It installs the binary to `/usr/local/bin/lqosync3` and the unit to `/etc/systemd/system/lqosync3.service`. Then it runs `systemctl daemon-reload`.
6. It makes `/etc/lqosync3/`, `/etc/lqosync3/secrets/` (mode `0700`), and `/var/lib/lqosync3/`.
7. It writes `/etc/lqosync3/lqosync3.toml` only if the file does not exist.
8. It gives a warning if `lqosync-updatecsv.service` (the Python service) is active.
9. It runs `lqosync3 check`. If the check fails, it does not start the service.
10. It enables and restarts the service, unless you use `--no-start`.

The installer does not do these actions:

- It does not stop or change `lqosync-updatecsv.service`.
- It does not change an existing config file or the secret files.
- It does not change LibreQoS files, LibreQoS configuration, or LibreQoS services.
- It does not install, remove, or upgrade packages.
- It does not write to the routers.

## Manual installation

Run all commands of this procedure on the shaper. The commands with `sudo` need root.

1. Build the binary. Run this command as your normal user, in the clone of the repository. It does not need root:

   ```bash
   cargo build --release --locked
   ```

2. Install the binary. This copies it to `/usr/local/bin`, with mode `0755`:

   ```bash
   sudo install -m 0755 target/release/lqosync3 /usr/local/bin/lqosync3
   ```

3. Make the directories:
   - `/etc/lqosync3` holds the config.
   - `/etc/lqosync3/secrets` holds the passwords. Only root can open it.
   - `/var/lib/lqosync3` holds the state of the service.

   ```bash
   sudo install -d -m 0755 /etc/lqosync3 /var/lib/lqosync3
   sudo install -d -m 0700 /etc/lqosync3/secrets
   ```

4. Copy the example configuration from [CONFIGURATION.md](CONFIGURATION.md) to `/etc/lqosync3/lqosync3.toml`. For each router, set:
   - `name = "R1"`
   - `address = "ROUTER_IP"`
   - `username = "lqosync"` (the user of "Prepare each router")
   - `password_file = "/etc/lqosync3/secrets/R1.password"`
   - `tls = true` and `port = 8729` for `api-ssl`, or `tls = false` and `port = 8728` for the plain API.
5. Write the password of the `lqosync` user of each router to its secret file. The first command makes an empty file with mode `0600` and owner `root`. The second command opens it in an editor:

   ```bash
   sudo install -m 0600 /dev/null /etc/lqosync3/secrets/R1.password
   sudo nano /etc/lqosync3/secrets/R1.password
   ```

   Type only the password on one line. Save and close. The tool ignores one newline at the end of the file. It does not accept a file with a different mode or owner.

6. Check the configuration. The command reads `/etc/lqosync3/lqosync3.toml` (use `--config <path>` for another file). It does not connect to the routers. It checks the keys, the secret files, the TLS files, the paths, and the LibreQoS target:

   ```bash
   sudo lqosync3 check
   ```

   Expected result: no `ERROR` line. A `WARN` line tells you about a risk, for example the plain API.

7. Install the service unit as `/etc/systemd/system/lqosync3.service`:

   The file is [`systemd/lqosync3.service`](../systemd/lqosync3.service):

   ```ini
   [Unit]
   Description=lqosync3: MikroTik subscriber sync for LibreQoS
   After=network-online.target
   Wants=network-online.target

   [Service]
   Type=simple
   ExecStart=/usr/local/bin/lqosync3 run --config /etc/lqosync3/lqosync3.toml
   ExecReload=/bin/kill -HUP $MAINPID
   Restart=on-failure
   RestartSec=10
   # Stop sends SIGTERM to lqosync3 only; it finishes the cycle, and a running LibreQoS.py is not stopped.
   KillMode=mixed
   TimeoutStopSec=600
   User=root
   NoNewPrivileges=yes
   ProtectSystem=full
   ReadWritePaths=/var/lib/lqosync3 /opt/libreqos/src
   # The TUI Dashboard shows the disk I/O and network bytes of the service from these counters.
   IOAccounting=yes
   IPAccounting=yes

   [Install]
   WantedBy=multi-user.target
   ```

8. Start the service. The first command reads the new unit file. The second starts the service now and at each boot. The service starts in the `runtime.mode` of the config (`shadow` writes only candidate files):

   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable --now lqosync3.service
   ```

> [!note]
> In `v2_pair` mode, the service starts `LibreQoS.py --updateonly`. That command needs full system access. Do not add `ProtectSystem=strict` until a test shows that the reload still operates.

## Checks after installation

1. Make sure that the service operates:

   ```bash
   systemctl status lqosync3.service
   ```

2. Make sure that the tool selected the correct target. `detect` reads the LibreQoS version and configuration only:

   ```bash
   sudo lqosync3 detect
   ```

   Expected result on LibreQoS 2.2: `target=v2_pair`.

3. After the first cycle, compare the candidate files with the live LibreQoS files. `diff` reads files only:

   ```bash
   sudo lqosync3 diff
   ```

   NOTE: Do not run `lqosync3 once` while the service operates. The service holds the lock in `/var/lib/lqosync3`, so `once` stops with an error. In `apply` mode, `once` also writes the LibreQoS files.

4. Read the log of the first cycles. Look for `router R1: online` and for errors:

   ```bash
   journalctl -u lqosync3.service -n 50
   ```

5. Make sure that the candidate files exist in `/var/lib/lqosync3/candidate/`.

## New LibreQoS installation

A new LibreQoS has no subscriber data in `/opt/libreqos/src`, and it did not do a full run. LibreQoS shapes no subscriber until the files exist. The tool makes the files and starts the first full run:

1. Install and start the tool in `shadow` mode. The tool writes the files in `/var/lib/lqosync3/candidate/` only.
2. Examine the candidate files: `sudo lqosync3 diff`, or the ShapedDevices and Network tabs of the TUI.
3. Set `runtime.mode = "apply"` (TUI Settings tab, group `runtime`, then `v`, `A`, `y`).
4. At the next cycle, the tool writes `network.json` and `ShapedDevices.csv` in `/opt/libreqos/src`.
5. Then the tool runs `LibreQoS.py` without `--updateonly` (a full run, `libreqos.v2_first_args`). `--updateonly` cannot operate before the first full run, because `/opt/libreqos/state/shaping/ShapedDevices.lastLoaded.csv` does not exist.
6. Make sure that the Dashboard shows `reload: ok (first full run)`. If the full run fails, the tool tries it again at each cycle. Read the error in `journalctl -u lqosync3.service`.

After the first full run, LibreQoS has `ShapedDevices.lastLoaded.csv`. Then each reload uses `--updateonly` (`libreqos.v2_args`).

NOTE: On LibreQoS 3.x (`v3_pair` and `v3_network`), `lqosd` reads the new files by itself. The tool runs no reload command.

## Removal

1. Stop and disable the service:

   ```bash
   sudo systemctl disable --now lqosync3.service
   ```

2. Remove the unit and the binary:

   ```bash
   sudo rm /etc/systemd/system/lqosync3.service /usr/local/bin/lqosync3
   sudo systemctl daemon-reload
   ```

3. If you do not need the configuration and the secrets, remove them:

   ```bash
   sudo rm -r /etc/lqosync3 /var/lib/lqosync3
   ```

> [!caution]
> If the tool operated in `apply` mode, start the Python service before the removal. If not, no tool updates the LibreQoS files.
