# Installation

This document gives the one-line installation, the manual installation, the checks after installation, and the removal procedure.

> [!warning] Status: target contract
> `install.sh` and the release binary do not exist yet. The commands below are the contract for the implementation.

## Requirements

- Ubuntu 24.04 (Noble) on x86_64.
- LibreQoS 2.0 to 2.2, or LibreQoS 3.x.
- Root access.
- RouterOS API access from the shaper to each router. Use port `8729` (TLS) if possible.
- A read-only RouterOS user for each router.
- For an installation from source: Rust 1.88 or later and `cargo`.

## One-line installation

Use this command on the shaper:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync3/install.sh | sudo bash
```

> [!note]
> The tool is at the root of the branch `lqosync3` of `p33ckab00/LQoSync`. The branch has no shared history with the other branches.
> The file `install.sh` at the root of the other branches is a different installer. Do not use it for this tool.

To install from a local clone, use this command:

```bash
sudo bash /home/pi/LQoSync3.0/install.sh
```

### Installer options

| Option | Default | Effect |
|--------|---------|--------|
| `--mode shadow\|apply` | `shadow` | Sets `runtime.mode` in a new config file. The installer does not change an existing config file. |
| `--version <tag>` | latest release | Installs a specific release. |
| `--from-source` | off | Builds the binary with `cargo build --release --locked`. |
| `--import-json <path>` | off | Converts an old `config.json` to TOML and secret files. |
| `--no-start` | off | Installs the files, but does not start the service. |

Example with an import of the current Python configuration:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync3/install.sh | sudo bash -s -- --import-json /opt/libreqos/src/config.json
```

### What the installer does

1. It stops if the user is not `root`.
2. It stops if the operating system or the CPU architecture is not supported.
3. It reads the LibreQoS version with `dpkg-query -W libreqos`. It stops if the version is not 2.0 to 2.2 or 3.x.
4. It downloads the release binary and its SHA-256 file. It stops if the checksum is not correct.
5. It installs the binary to `/usr/local/bin/lqosync3`.
6. It makes `/etc/lqosync3/`, `/etc/lqosync3/secrets/` (mode `0700`), and `/var/lib/lqosync3/`.
7. It writes `/etc/lqosync3/lqosync3.toml` only if the file does not exist.
8. It installs `lqosync3.service` and runs `systemctl daemon-reload`.
9. It runs `lqosync3 check`. If the check fails, it does not start the service.
10. It starts the service in `shadow` mode.

The installer does not do these actions:

- It does not stop or change `lqosync-updatecsv.service` (the Python service).
- It does not change LibreQoS files, LibreQoS configuration, or LibreQoS services.
- It does not install, remove, or upgrade packages.
- It does not write to the routers.

## Manual installation

1. Build the binary:

   ```bash
   cargo build --release --locked
   ```

2. Install the binary:

   ```bash
   sudo install -m 0755 target/release/lqosync3 /usr/local/bin/lqosync3
   ```

3. Make the directories:

   ```bash
   sudo install -d -m 0755 /etc/lqosync3 /var/lib/lqosync3
   sudo install -d -m 0700 /etc/lqosync3/secrets
   ```

4. Copy the example configuration from [CONFIGURATION.md](CONFIGURATION.md) to `/etc/lqosync3/lqosync3.toml`.
5. Write each router password to its secret file:

   ```bash
   sudo install -m 0600 /dev/null /etc/lqosync3/secrets/Router-1.password
   sudo nano /etc/lqosync3/secrets/Router-1.password
   ```

6. Check the configuration:

   ```bash
   sudo lqosync3 check
   ```

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
   User=root
   NoNewPrivileges=yes
   ProtectSystem=full
   ReadWritePaths=/var/lib/lqosync3 /opt/libreqos/src

   [Install]
   WantedBy=multi-user.target
   ```

8. Start the service:

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

2. Make sure that the tool selected the correct target:

   ```bash
   sudo lqosync3 once
   sudo lqosync3 diff
   ```

   Expected result on LibreQoS 2.2: `target=v2_pair`.

3. Read the log of the first cycles:

   ```bash
   journalctl -u lqosync3.service -n 50
   ```

4. Make sure that the candidate files exist in `/var/lib/lqosync3/candidate/`.

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
