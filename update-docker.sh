#!/usr/bin/env bash
set -euo pipefail

# Docker/Compose update wrapper. Rebuilds local image and preserves live LibreQoS files by default.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INIT_POLICY="${LQOSYNC_INIT_POLICY:-preserve_existing}"
COMPOSE_FILE="${LQOSYNC_COMPOSE_FILE:-compose.preserve-existing.yaml}"
TS="$(date +%Y%m%d_%H%M%S)"
BACKUP_DIR="${LQOSYNC_BACKUP_ROOT:-/root/lqosync_docker_update_backups}/$TS"

if [ "${EUID:-$(id -u)}" -ne 0 ]; then
  echo "Run as root: sudo bash update-docker.sh" >&2
  exit 1
fi

mkdir -p "$BACKUP_DIR/libreqos_src"
cp -a /opt/LQoSync/users.json "$BACKUP_DIR/users.json" 2>/dev/null || true
cp -a /opt/LQoSync/.env "$BACKUP_DIR/.env" 2>/dev/null || true
cp -a /opt/LQoSync/state "$BACKUP_DIR/state" 2>/dev/null || true
cp -a /opt/libreqos/src/config.json "$BACKUP_DIR/libreqos_src/config.json" 2>/dev/null || true
cp -a /opt/libreqos/src/ShapedDevices.csv "$BACKUP_DIR/libreqos_src/ShapedDevices.csv" 2>/dev/null || true
cp -a /opt/libreqos/src/network.json "$BACKUP_DIR/libreqos_src/network.json" 2>/dev/null || true

cd "$SCRIPT_DIR"
LQOSYNC_INIT_POLICY="$INIT_POLICY" docker compose -f "$COMPOSE_FILE" up -d --build

echo "[LQoSync Docker] Update complete. Backup: $BACKUP_DIR"
