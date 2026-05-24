#!/usr/bin/env bash
set -euo pipefail

# Explicit ZIP/local-package updater.
# Run this from an extracted LQoSync ZIP directory.
# It refreshes /opt/LQoSync source while preserving operator/runtime files.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${LQOSYNC_INSTALL_DIR:-/opt/LQoSync}"
LIBREQOS_SRC="${LIBREQOS_SRC:-/opt/libreqos/src}"
INIT_POLICY="${LQOSYNC_INIT_POLICY:-preserve_existing}"
SERVICE_START_POLICY="${LQOSYNC_SERVICE_START_POLICY:-enable_only}"
TS="$(date +%Y%m%d_%H%M%S)"
BACKUP_ROOT="${LQOSYNC_BACKUP_ROOT:-/root/lqosync_zip_update_backups}"
BACKUP_DIR="$BACKUP_ROOT/$TS"

if [ "${EUID:-$(id -u)}" -ne 0 ]; then
  echo "Run as root: sudo bash update-from-zip.sh" >&2
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

mkdir -p "$BACKUP_DIR/libreqos_src"
echo "[LQoSync ZIP Update] Backup: $BACKUP_DIR"

cp -a "$INSTALL_DIR/users.json" "$BACKUP_DIR/users.json" 2>/dev/null || true
cp -a "$INSTALL_DIR/.env" "$BACKUP_DIR/.env" 2>/dev/null || true
cp -a "$INSTALL_DIR/state" "$BACKUP_DIR/state" 2>/dev/null || true
cp -a "$INSTALL_DIR/logs" "$BACKUP_DIR/logs" 2>/dev/null || true
cp -a "$INSTALL_DIR/backups" "$BACKUP_DIR/lqosync_backups" 2>/dev/null || true
cp -a "$LIBREQOS_SRC/config.json" "$BACKUP_DIR/libreqos_src/config.json" 2>/dev/null || true
cp -a "$LIBREQOS_SRC/ShapedDevices.csv" "$BACKUP_DIR/libreqos_src/ShapedDevices.csv" 2>/dev/null || true
cp -a "$LIBREQOS_SRC/network.json" "$BACKUP_DIR/libreqos_src/network.json" 2>/dev/null || true
cp -a "/etc/systemd/system/lqosync.service" "$BACKUP_DIR/lqosync.service" 2>/dev/null || true
cp -a "/etc/systemd/system/lqosync-core.service" "$BACKUP_DIR/lqosync-core.service" 2>/dev/null || true
cp -a "/etc/sudoers.d/lqosync" "$BACKUP_DIR/sudoers.lqosync" 2>/dev/null || true

if systemctl is-active --quiet lqosync-core 2>/dev/null; then
  echo "[LQoSync ZIP Update] Stopping lqosync-core before source refresh."
  systemctl stop lqosync-core || true
fi
if systemctl is-active --quiet lqosync 2>/dev/null; then
  echo "[LQoSync ZIP Update] Stopping retired lqosync service before source refresh."
  systemctl stop lqosync || true
fi

mkdir -p "$INSTALL_DIR"
rsync -a --delete \
  --exclude '.git' \
  --exclude 'venv' \
  --exclude 'users.json' \
  --exclude '.env' \
  --exclude 'state' \
  --exclude 'logs' \
  --exclude 'backups' \
  --exclude 'install_backups' \
  --exclude 'config_backups' \
  "$SCRIPT_DIR/" "$INSTALL_DIR/"

cp -a "$BACKUP_DIR/users.json" "$INSTALL_DIR/users.json" 2>/dev/null || true
cp -a "$BACKUP_DIR/.env" "$INSTALL_DIR/.env" 2>/dev/null || true
cp -a "$BACKUP_DIR/state" "$INSTALL_DIR/state" 2>/dev/null || true
cp -a "$BACKUP_DIR/logs" "$INSTALL_DIR/logs" 2>/dev/null || true
cp -a "$BACKUP_DIR/lqosync_backups" "$INSTALL_DIR/backups" 2>/dev/null || true

cd "$INSTALL_DIR"
LQOSYNC_INSTALL_DIR="$INSTALL_DIR" \
LIBREQOS_SRC="$LIBREQOS_SRC" \
LQOSYNC_INIT_POLICY="$INIT_POLICY" \
LQOSYNC_SERVICE_START_POLICY="$SERVICE_START_POLICY" \
LQOSYNC_INSTALL_MODE=baremetal \
bash install.sh

cat > "$BACKUP_DIR/zip_update_summary.json" <<JSON
{
  "timestamp": "$TS",
  "install_dir": "$INSTALL_DIR",
  "libreqos_src": "$LIBREQOS_SRC",
  "init_policy": "$INIT_POLICY",
  "service_start_policy": "$SERVICE_START_POLICY",
  "backup_dir": "$BACKUP_DIR"
}
JSON

cat <<NEXT

[LQoSync ZIP Update] Complete.
Backup:         $BACKUP_DIR
Service policy: $SERVICE_START_POLICY

If service was not started by policy:
  sudo systemctl start lqosync-core
NEXT
