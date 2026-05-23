#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="${LQOSYNC_INSTALL_DIR:-/opt/LQoSync}"
LIBREQOS_SRC_DIR="${LIBREQOS_SRC_DIR:-${LIBREQOS_SRC:-/opt/libreqos/src}}"
CONFIG_PATH="${CONFIG_PATH:-$LIBREQOS_SRC_DIR/config.json}"
SHAPED_DEVICES_PATH="${SHAPED_DEVICES_PATH:-$LIBREQOS_SRC_DIR/ShapedDevices.csv}"
NETWORK_JSON_PATH="${NETWORK_JSON_PATH:-$LIBREQOS_SRC_DIR/network.json}"
USER_NAME="${LQOSYNC_USER:-lqosync}"
SERVICE_NAME="${LQOSYNC_SERVICE_NAME:-lqosync}"

if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
  echo "Run as root: sudo bash scripts/adopt-runtime-permissions.sh" >&2
  exit 1
fi

echo "[LQoSync] Adopting runtime user and permissions"

if ! id "$USER_NAME" >/dev/null 2>&1; then
  useradd --system --no-create-home --shell /usr/sbin/nologin "$USER_NAME"
fi

if getent group systemd-journal >/dev/null 2>&1; then
  usermod -aG systemd-journal "$USER_NAME" || true
fi

mkdir -p \
  "$INSTALL_DIR" \
  "$INSTALL_DIR/backups" \
  "$INSTALL_DIR/logs" \
  "$INSTALL_DIR/state" \
  "$INSTALL_DIR/config_backups" \
  "$INSTALL_DIR/install_backups" \
  "$LIBREQOS_SRC_DIR"

touch /var/log/lqosync.log || true

chown -R "$USER_NAME:$USER_NAME" "$INSTALL_DIR"
chown "$USER_NAME:$USER_NAME" /var/log/lqosync.log || true

for managed in "$CONFIG_PATH" "$SHAPED_DEVICES_PATH" "$NETWORK_JSON_PATH"; do
  if [[ -e "$managed" ]]; then
    chown "$USER_NAME:$USER_NAME" "$managed" || true
  fi
done

[[ -e "$CONFIG_PATH" ]] && chmod 600 "$CONFIG_PATH" || true
[[ -e "$SHAPED_DEVICES_PATH" ]] && chmod 664 "$SHAPED_DEVICES_PATH" || true
[[ -e "$NETWORK_JSON_PATH" ]] && chmod 664 "$NETWORK_JSON_PATH" || true
[[ -e "$INSTALL_DIR/users.json" ]] && chmod 600 "$INSTALL_DIR/users.json" || true
[[ -e "$INSTALL_DIR/.env" ]] && chmod 600 "$INSTALL_DIR/.env" || true

chmod 700 \
  "$INSTALL_DIR/backups" \
  "$INSTALL_DIR/state" \
  "$INSTALL_DIR/config_backups" \
  "$INSTALL_DIR/install_backups" || true

if command -v setfacl >/dev/null 2>&1; then
  setfacl -m "u:$USER_NAME:rwx" "$LIBREQOS_SRC_DIR" || true
  [[ -e "$CONFIG_PATH" ]] && setfacl -m "u:$USER_NAME:rw" "$CONFIG_PATH" || true
  [[ -e "$SHAPED_DEVICES_PATH" ]] && setfacl -m "u:$USER_NAME:rw" "$SHAPED_DEVICES_PATH" || true
  [[ -e "$NETWORK_JSON_PATH" ]] && setfacl -m "u:$USER_NAME:rw" "$NETWORK_JSON_PATH" || true
  setfacl -d -m "u:$USER_NAME:rwX" "$LIBREQOS_SRC_DIR" || true
  echo "[LQoSync] ACL permissions applied for $USER_NAME on $LIBREQOS_SRC_DIR"
else
  echo "[LQoSync] WARNING: setfacl not available. Install acl if atomic writes fail."
fi

if sudo -u "$USER_NAME" sh -c "touch '$LIBREQOS_SRC_DIR/.lqosync_acl_test' && rm -f '$LIBREQOS_SRC_DIR/.lqosync_acl_test'"; then
  echo "[LQoSync] Permission test passed: $USER_NAME can create temp files in $LIBREQOS_SRC_DIR"
else
  echo "[LQoSync] WARNING: $USER_NAME cannot create temp files in $LIBREQOS_SRC_DIR"
fi

echo "[LQoSync] Runtime ownership and managed-file permissions adopted for $USER_NAME"
echo "[LQoSync] Service name: $SERVICE_NAME"
