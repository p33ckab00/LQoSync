#!/usr/bin/env bash
set -euo pipefail

# Capture the original ownership, modes, and ACLs before LQoSync adopts runtime
# permissions. Uninstall uses this snapshot to restore the server's previous
# state instead of guessing root defaults.

INSTALL_DIR="${LQOSYNC_INSTALL_DIR:-/opt/LQoSync}"
LIBREQOS_SRC_DIR="${LIBREQOS_SRC_DIR:-${LIBREQOS_SRC:-/opt/libreqos/src}}"
CONFIG_PATH="${CONFIG_PATH:-$LIBREQOS_SRC_DIR/config.json}"
SHAPED_DEVICES_PATH="${SHAPED_DEVICES_PATH:-$LIBREQOS_SRC_DIR/ShapedDevices.csv}"
NETWORK_JSON_PATH="${NETWORK_JSON_PATH:-$LIBREQOS_SRC_DIR/network.json}"
SNAPSHOT_ROOT="${LQOSYNC_PERMISSION_SNAPSHOT_ROOT:-/root/lqosync_permission_snapshots}"
STAMP="$(date +%Y%m%d_%H%M%S)"
SNAPSHOT_DIR="${LQOSYNC_PERMISSION_SNAPSHOT_DIR:-$SNAPSHOT_ROOT/$STAMP}"
FORCE="${LQOSYNC_FORCE_PERMISSION_SNAPSHOT:-false}"

if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
  echo "Run as root: sudo bash scripts/snapshot-runtime-permissions.sh" >&2
  exit 1
fi

as_true() {
  case "$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|y|on) return 0 ;;
    *) return 1 ;;
  esac
}

latest_from_pointer() {
  if [[ -f "$SNAPSHOT_ROOT/latest.path" ]]; then
    read -r p < "$SNAPSHOT_ROOT/latest.path" || true
    if [[ -n "${p:-}" && -f "$p/metadata.tsv" ]]; then
      printf '%s\n' "$p"
      return 0
    fi
  fi
  if [[ -f "$SNAPSHOT_ROOT/latest/metadata.tsv" ]]; then
    printf '%s\n' "$SNAPSHOT_ROOT/latest"
    return 0
  fi
  return 1
}

if ! as_true "$FORCE"; then
  if existing="$(latest_from_pointer 2>/dev/null)"; then
    echo "[LQoSync] Existing original permission snapshot kept: $existing"
    exit 0
  fi
fi

mkdir -p "$SNAPSHOT_DIR"
chmod 700 "$SNAPSHOT_ROOT" "$SNAPSHOT_DIR" 2>/dev/null || true

metadata_tmp="$SNAPSHOT_DIR/metadata.tsv.tmp"
metadata="$SNAPSHOT_DIR/metadata.tsv"
: > "$metadata_tmp"

record_missing() {
  printf 'missing\t\t\t\t%s\n' "$1" >> "$metadata_tmp"
}

record_one() {
  local p="$1"
  if [[ -e "$p" || -L "$p" ]]; then
    find "$p" -maxdepth 0 -printf '%y\t%m\t%U\t%G\t%p\n' >> "$metadata_tmp"
  else
    record_missing "$p"
  fi
}

record_tree() {
  local p="$1"
  if [[ -d "$p" ]]; then
    find "$p" -xdev -printf '%y\t%m\t%U\t%G\t%p\n' >> "$metadata_tmp"
  else
    record_missing "$p"
  fi
}

record_tree "$INSTALL_DIR"
record_one "$LIBREQOS_SRC_DIR"
record_one "$CONFIG_PATH"
record_one "$SHAPED_DEVICES_PATH"
record_one "$NETWORK_JSON_PATH"
record_one /var/log/lqosync.log

{
  printf '#type\tmode\tuid\tgid\tpath\n'
  awk -F '\t' 'NF >= 5 && !seen[$5]++ { print }' "$metadata_tmp"
} > "$metadata"
rm -f "$metadata_tmp"

acl_restore="$SNAPSHOT_DIR/acl.restore"
: > "$acl_restore"
if command -v getfacl >/dev/null 2>&1; then
  if [[ -d "$INSTALL_DIR" ]]; then
    getfacl -R -p "$INSTALL_DIR" >> "$acl_restore" 2>/dev/null || true
  fi
  for p in "$LIBREQOS_SRC_DIR" "$CONFIG_PATH" "$SHAPED_DEVICES_PATH" "$NETWORK_JSON_PATH" /var/log/lqosync.log; do
    [[ -e "$p" || -L "$p" ]] && getfacl -p "$p" >> "$acl_restore" 2>/dev/null || true
  done
else
  echo "[LQoSync] getfacl not available; ownership/mode metadata captured without ACL backup." >&2
fi

cat > "$SNAPSHOT_DIR/manifest.env" <<EOF
created_at=$STAMP
install_dir=$INSTALL_DIR
libreqos_src_dir=$LIBREQOS_SRC_DIR
config_path=$CONFIG_PATH
shaped_devices_path=$SHAPED_DEVICES_PATH
network_json_path=$NETWORK_JSON_PATH
metadata=$metadata
acl_restore=$acl_restore
EOF

ln -sfn "$SNAPSHOT_DIR" "$SNAPSHOT_ROOT/latest"
printf '%s\n' "$SNAPSHOT_DIR" > "$SNAPSHOT_ROOT/latest.path"

echo "[LQoSync] Original permission snapshot saved: $SNAPSHOT_DIR"
