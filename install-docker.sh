#!/usr/bin/env bash
set -euo pipefail

# Docker/Compose install wrapper.
# Docker mode is host-integrated. Bare-metal/systemd is preferred for live LibreQoS.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INIT_POLICY="${LQOSYNC_INIT_POLICY:-preserve_existing}"
COMPOSE_FILE="${LQOSYNC_COMPOSE_FILE:-compose.preserve-existing.yaml}"

if [ "${EUID:-$(id -u)}" -ne 0 ]; then
  echo "Run as root: sudo bash install-docker.sh" >&2
  exit 1
fi

case "$INIT_POLICY" in
  preserve_existing|create_missing_only|overwrite_with_backup|smart_confirm) ;;
  *) echo "Invalid LQOSYNC_INIT_POLICY=$INIT_POLICY" >&2; exit 1 ;;
esac

apt-get update -qq
apt-get install -y docker.io docker-compose-plugin
systemctl enable --now docker

cd "$SCRIPT_DIR"
LQOSYNC_INIT_POLICY="$INIT_POLICY" docker compose -f "$COMPOSE_FILE" up -d --build

echo "[LQoSync Docker] Install complete. Logs: sudo docker logs -f lqosync"
