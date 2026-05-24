#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVICE_SRC="$ROOT_DIR/systemd/lqosync-core.service"
SERVICE_DEST="${LQOSYNC_CORE_SERVICE_DEST:-/etc/systemd/system/lqosync-core.service}"
BIN="${LQOSYNC_CORE_DEST:-/usr/local/bin/lqosync-core}"
SERVICE_NAME="${LQOSYNC_CORE_SERVICE_NAME:-lqosync-core}"
SERVICE_START_POLICY="${LQOSYNC_CORE_SERVICE_START_POLICY:-${LQOSYNC_SERVICE_START_POLICY:-restart}}"
if [ ! -x "$BIN" ]; then
  echo "Rust core binary not installed at $BIN. Run sudo bash scripts/install-rust-core.sh first." >&2
  exit 1
fi
install -m 0644 "$SERVICE_SRC" "$SERVICE_DEST"
systemctl daemon-reload
case "$SERVICE_START_POLICY" in
  restart)
    systemctl enable "${SERVICE_NAME}.service"
    if systemctl is-active --quiet "${SERVICE_NAME}.service"; then
      systemctl restart "${SERVICE_NAME}.service"
    else
      systemctl start "${SERVICE_NAME}.service"
    fi
    echo "Installed and started ${SERVICE_NAME} Rust backend service with scheduler, socket, and HTTP authority enabled by systemd unit."
    ;;
  enable_only)
    systemctl enable "${SERVICE_NAME}.service"
    echo "Installed and enabled ${SERVICE_NAME} Rust backend service without starting it."
    ;;
  leave_stopped)
    systemctl disable "${SERVICE_NAME}.service" 2>/dev/null || true
    systemctl stop "${SERVICE_NAME}.service" 2>/dev/null || true
    echo "Installed ${SERVICE_NAME} Rust backend service and left it stopped by policy."
    ;;
  *)
    echo "Invalid LQOSYNC_CORE_SERVICE_START_POLICY=$SERVICE_START_POLICY" >&2
    exit 1
    ;;
esac
systemctl status "${SERVICE_NAME}.service" --no-pager || true
