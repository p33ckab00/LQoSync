# Docker Operations Guide

Docker mode is host-integrated. It uses host networking and host process namespace because LQoSync may write LibreQoS files and call the host LibreQoS apply command.

For live LibreQoS, bare-metal/systemd remains the preferred deployment mode. Use Docker only when you intentionally want host-integrated Docker.

## Install

```bash
cd /opt/LQoSync
sudo bash install-docker.sh
```

The Docker wrapper defaults to preserving live LibreQoS files:

```text
LQOSYNC_INIT_POLICY=preserve_existing
```

## Update

```bash
cd /opt/LQoSync
sudo bash update-docker.sh
```

## Uninstall Docker container

```bash
cd /opt/LQoSync
sudo bash uninstall-docker.sh
```

Remove image too:

```bash
sudo REMOVE_DOCKER_IMAGE=true bash uninstall-docker.sh
```

Remove runtime folder too:

```bash
sudo REMOVE_RUNTIME=true bash uninstall-docker.sh
```

## Manual Compose commands

```bash
sudo docker compose -f compose.preserve-existing.yaml up -d --build
sudo docker logs -f lqosync
sudo docker compose down
```
