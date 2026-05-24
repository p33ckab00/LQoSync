#!/usr/bin/env bash
set -euo pipefail

# Explicit ZIP/local-package installer.
# Run this from an extracted LQoSync ZIP directory.
# Production-safe defaults: preserve live LibreQoS files and do not start/restart service.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${LQOSYNC_INSTALL_DIR:-/opt/LQoSync}"
LIBREQOS_SRC="${LIBREQOS_SRC:-/opt/libreqos/src}"
INIT_POLICY="${LQOSYNC_INIT_POLICY:-preserve_existing}"
SERVICE_START_POLICY="${LQOSYNC_SERVICE_START_POLICY:-enable_only}"

if [ "${EUID:-$(id -u)}" -ne 0 ]; then
  echo "Run as root: sudo bash install-from-zip.sh" >&2
  exit 1
fi

case "$INIT_POLICY" in
  preserve_existing|create_missing_only|overwrite_with_backup|smart_confirm) ;;
  *) echo "Invalid LQOSYNC_INIT_POLICY=$INIT_POLICY" >&2; exit 1 ;;
esac
case "$SERVICE_START_POLICY" in
  restart|enable_only|leave_stopped) ;;
  *) echo "Invalid LQOSYNC_SERVICE_START_POLICY=$SERVICE_START_POLICY" >&2; exit 1 ;;
esac

if [ ! -f "$SCRIPT_DIR/install.sh" ]; then
  echo "ERROR: install.sh not found. Run from the extracted LQoSync ZIP root." >&2
  exit 1
fi

if [ -f "$SCRIPT_DIR/install-production-safe.sh" ]; then
  echo "[LQoSync ZIP Install] Using production-safe installer wrapper."
  LQOSYNC_INSTALL_DIR="$INSTALL_DIR" \
  LIBREQOS_SRC="$LIBREQOS_SRC" \
  LQOSYNC_INIT_POLICY="$INIT_POLICY" \
  LQOSYNC_SERVICE_START_POLICY="$SERVICE_START_POLICY" \
  bash "$SCRIPT_DIR/install-production-safe.sh"
else
  echo "[LQoSync ZIP Install] Using install.sh with ZIP-safe defaults."
  cd "$SCRIPT_DIR"
  LQOSYNC_INSTALL_DIR="$INSTALL_DIR" \
  LIBREQOS_SRC="$LIBREQOS_SRC" \
  LQOSYNC_INIT_POLICY="$INIT_POLICY" \
  LQOSYNC_SERVICE_START_POLICY="$SERVICE_START_POLICY" \
  LQOSYNC_INSTALL_MODE=baremetal \
  bash install.sh
fi

cat <<NEXT

[LQoSync ZIP Install] Complete.
Install dir:      $INSTALL_DIR
LibreQoS src:     $LIBREQOS_SRC
Init policy:      $INIT_POLICY
Service policy:   $SERVICE_START_POLICY

Next for live systems:
  1. Review Config Center
  2. Run verification / dry-run CLI checks
  3. Start service if not already started: sudo systemctl start lqosync-core
NEXT
