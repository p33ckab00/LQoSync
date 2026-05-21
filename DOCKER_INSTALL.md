# Docker Install

Docker is secondary. Bare metal/systemd is preferred for live LibreQoS hosts.

```bash
sudo bash install-docker.sh
```

Update:

```bash
sudo bash update-docker.sh
```

Uninstall:

```bash
sudo bash uninstall-docker.sh
```

Docker must preserve `/opt/libreqos/src/config.json`, `ShapedDevices.csv`, and `network.json`.
