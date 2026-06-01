# Operator Guide

## Install destination

Online branch path:

```text
src/updatecsv.py
templates/config.json
```

Installed path:

```text
/opt/libreqos/src/updatecsv.py
/opt/libreqos/src/config.json
```

## Recommended operator sequence

1. Run one-line installer.
2. Edit `/opt/libreqos/src/config.json`.
3. Optional: edit `/opt/libreqos/src/updatecsv.py`.
4. Validate JSON and Python syntax.
5. Run once manually.
6. Enable/start systemd.
7. Watch logs.

## Commands

Install only:

```bash
curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/one-line-cli/install.sh | sudo bash
```

Edit config:

```bash
sudo nano /opt/libreqos/src/config.json
```

Edit script, optional:

```bash
sudo nano /opt/libreqos/src/updatecsv.py
```

Validate:

```bash
sudo python3 -m json.tool /opt/libreqos/src/config.json >/dev/null
sudo python3 -m py_compile /opt/libreqos/src/updatecsv.py
```

Manual run:

```bash
cd /opt/libreqos/src
sudo python3 updatecsv.py
```

Enable and start:

```bash
sudo systemctl enable --now lqosync-updatecsv
```

Service management:

```bash
sudo systemctl start lqosync-updatecsv
sudo systemctl stop lqosync-updatecsv
sudo systemctl restart lqosync-updatecsv
sudo systemctl disable lqosync-updatecsv
sudo journalctl -u lqosync-updatecsv -f
```

## Logical-core mode

Formula:

```text
logical_buckets = os.cpu_count() - reserve_cores
```

Then clamp:

```text
logical_buckets >= min_count
logical_buckets <= max_count
```

Recommended for around 25 clients:

```json
{
  "reserve_cores": 2,
  "min_count": 2,
  "max_count": 6,
  "rebalance_threshold_clients": 3,
  "max_rebalance_moves_per_cycle": 1
}
```

## Why edit config before start?

The template includes example router credentials and bandwidth values. Starting the service before editing may cause connection errors or write a shape tree that does not match your network.

Always confirm:

- MikroTik address
- API username/password
- PPPoE/DHCP enabled flags
- DHCP server names
- root bandwidth
- logical root count limits
