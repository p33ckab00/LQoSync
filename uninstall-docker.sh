#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="${LQOSYNC_COMPOSE_FILE:-compose.preserve-existing.yaml}"
REMOVE_DOCKER_IMAGE="${REMOVE_DOCKER_IMAGE:-false}"
REMOVE_RUNTIME="${REMOVE_RUNTIME:-false}"
TS="$(date +%Y%m%d_%H%M%S)"
BACKUP_DIR="${LQOSYNC_BACKUP_ROOT:-/root/lqosync_docker_uninstall_backups}/$TS"

if [ "${EUID:-$(id -u)}" -ne 0 ]; then
  echo "Run as root: sudo bash uninstall-docker.sh" >&2
  exit 1
fi

mkdir -p "$BACKUP_DIR/libreqos_src"
cp -a /opt/LQoSync "$BACKUP_DIR/opt_LQoSync" 2>/dev/null || true
cp -a /opt/libreqos/src/config.json "$BACKUP_DIR/libreqos_src/config.json" 2>/dev/null || true
cp -a /opt/libreqos/src/ShapedDevices.csv "$BACKUP_DIR/libreqos_src/ShapedDevices.csv" 2>/dev/null || true
cp -a /opt/libreqos/src/network.json "$BACKUP_DIR/libreqos_src/network.json" 2>/dev/null || true

cd "$SCRIPT_DIR"
docker compose -f "$COMPOSE_FILE" down 2>/dev/null || docker compose down 2>/dev/null || true

if [ "$REMOVE_DOCKER_IMAGE" = "true" ]; then
  docker image rm lqosync:local lqosync:2.148.1-rc1 2>/dev/null || true
fi

if [ "$REMOVE_RUNTIME" = "true" ]; then
  rm -rf /opt/LQoSync
  echo "[LQoSync Docker] Removed /opt/LQoSync"
else
  echo "[LQoSync Docker] Kept /opt/LQoSync. To remove it: sudo REMOVE_RUNTIME=true bash uninstall-docker.sh"
fi

echo "[LQoSync Docker] Uninstall complete. Backup: $BACKUP_DIR"
