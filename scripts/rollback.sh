#!/usr/bin/env bash
set -Eeuo pipefail
APP_DIR="${APP_DIR:-/opt/libreqos/src}"
SERVICE_NAME="${SERVICE_NAME:-lqosync-updatecsv}"
BACKUP_DIR="${1:-${BACKUP_DIR:-}}"
[[ -n "${BACKUP_DIR}" ]] || { echo "Usage: sudo $0 /opt/libreqos/backups/logical-core-root-YYYYMMDD-HHMMSS" >&2; exit 1; }
[[ -d "${BACKUP_DIR}" ]] || { echo "Backup directory not found: ${BACKUP_DIR}" >&2; exit 1; }

systemctl stop "${SERVICE_NAME}" 2>/dev/null || true
for f in updatecsv.py config.json ShapedDevices.csv network.json logical_core_assignments.json; do
  if [[ -e "${BACKUP_DIR}/${f}.bak" ]]; then
    cp -a "${BACKUP_DIR}/${f}.bak" "${APP_DIR}/${f}"
    echo "Restored ${f}"
  fi
done
systemctl restart "${SERVICE_NAME}" 2>/dev/null || true
systemctl --no-pager --full status "${SERVICE_NAME}" 2>/dev/null || true
