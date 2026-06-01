# LQoSync One-Line CLI Installer Branch

This branch is intentionally small and folder-based. It is **not** the full LQoSync source tree.

The goal is to provide a clean one-line installer, similar in installer/documentation style to public MikroTik-to-LibreQoS integration examples: clear install flow, central `config.json`, systemd service usage, MikroTik API guidance, verification, and rollback. The logic here remains LQoSync's logical-core design.

## Branch layout

```text
install.sh
src/updatecsv.py
templates/config.json
scripts/verify.sh
scripts/rollback.sh
docs/README.md
```

## What gets installed

The branch stores files in folders online:

```text
src/updatecsv.py
templates/config.json
```

During installation, both are installed together under LibreQoS source path:

```text
/opt/libreqos/src/updatecsv.py
/opt/libreqos/src/config.json
```

Default target path is `/opt/libreqos/src`. Override with `APP_DIR=/custom/path`.

## Safe first-time install flow

Recommended: install files first, then edit config, then start/enable service.

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/one-line-cli/install.sh | sudo RESERVE_CORES=2 MAX_COUNT=6 ROOT_DOWN=620 ROOT_UP=620 bash
```

By default, the installer:

- creates/updates `/opt/libreqos/src/updatecsv.py`
- creates/merges `/opt/libreqos/src/config.json`
- creates `lqosync-updatecsv.service` if missing
- does **not** enable or start the service yet

This lets you edit `config.json` first before running anything live.

## Edit config before running

Open the live config:

```bash
sudo nano /opt/libreqos/src/config.json
```

Minimum fields to check:

```json
{
  "network": {
    "root_download_mbps": 620,
    "root_upload_mbps": 620,
    "logical_roots": {
      "reserve_cores": 2,
      "max_count": 6
    }
  },
  "routers": [
    {
      "name": "RB5k9-Distro",
      "address": "10.200.1.2",
      "port": 8728,
      "username": "libreqos",
      "password": "CHANGE_ME",
      "pppoe": {
        "enabled": true,
        "per_plan_node": false
      }
    }
  ]
}
```

Important fields:

| Field | What to set |
|---|---|
| `routers[].address` | MikroTik API IP |
| `routers[].username` / `password` | MikroTik API read user |
| `root_download_mbps` / `root_upload_mbps` | total LibreQoS parent bandwidth |
| `reserve_cores` | CPU cores to leave for OS/system |
| `max_count` | max logical LCORE buckets |
| `pppoe.per_plan_node` | keep `false` for no plan-node mode |

## Optional: edit updatecsv.py before running

Only edit the script if you know what you are changing:

```bash
sudo nano /opt/libreqos/src/updatecsv.py
```

After any script edit, always validate:

```bash
sudo python3 -m py_compile /opt/libreqos/src/updatecsv.py
```

## Validate before service start

```bash
sudo python3 -m json.tool /opt/libreqos/src/config.json >/dev/null
sudo python3 -m py_compile /opt/libreqos/src/updatecsv.py
```

Or use the included verify script:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/one-line-cli/scripts/verify.sh | sudo bash
```

## Test one manual run before enabling

Run it manually first so you can see errors before enabling the background service:

```bash
cd /opt/libreqos/src
sudo python3 updatecsv.py
```

Stop with `Ctrl+C` after confirming it connects and writes as expected.

## Enable/start/stop/restart systemd

Default service name:

```text
lqosync-updatecsv.service
```

Commands:

```bash
sudo systemctl enable lqosync-updatecsv
sudo systemctl disable lqosync-updatecsv

sudo systemctl start lqosync-updatecsv
sudo systemctl stop lqosync-updatecsv
sudo systemctl restart lqosync-updatecsv

sudo systemctl status lqosync-updatecsv --no-pager
sudo journalctl -u lqosync-updatecsv -f
```

Enable and start together after config is ready:

```bash
sudo systemctl enable --now lqosync-updatecsv
```

## Auto-start during install, optional

For experienced users only, you can let the installer enable and start immediately:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/one-line-cli/install.sh | sudo AUTO_ENABLE=1 AUTO_START=1 RESERVE_CORES=2 MAX_COUNT=6 ROOT_DOWN=620 ROOT_UP=620 bash
```

## Config merge/replace behavior

Default:

```text
CONFIG_MODE=merge
```

This preserves existing router credentials when possible and applies the logical-core network block.

To fully replace the config with template values:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/one-line-cli/install.sh | sudo CONFIG_MODE=replace bash
```

## MikroTik API user example

Create a restricted read/API user on MikroTik:

```rsc
/user group add name=API_READ policy="read,sensitive,api,!policy,!local,!telnet,!ssh,!ftp,!reboot,!write,!test,!winbox,!password,!web,!sniff,!romon"
/user add name="libreqos" group=API_READ password="CHANGE_STRONG_PASSWORD" address="LIBREQOS_SERVER_IP" disabled=no
```

Then put that username/password in `/opt/libreqos/src/config.json`.

## Rollback

The installer prints a backup directory like:

```text
/opt/libreqos/backups/logical-core-root-YYYYMMDD-HHMMSS
```

Rollback:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/one-line-cli/scripts/rollback.sh | sudo BACKUP_DIR=/opt/libreqos/backups/logical-core-root-YYYYMMDD-HHMMSS bash
```

## Verify branch content

The `one-line-cli` branch should only contain installer files:

```text
README.md
TREE.txt
docs/README.md
install.sh
scripts/rollback.sh
scripts/verify.sh
src/updatecsv.py
templates/config.json
```
