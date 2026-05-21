# Uninstallation

Stop services first:

```bash
sudo systemctl stop lqosync lqosync-core
```

Run:

```bash
sudo bash uninstall.sh
```

Docker:

```bash
sudo bash uninstall-docker.sh
```

Uninstall does not delete LibreQoS itself. Back up `/opt/libreqos/src` before destructive operations.
