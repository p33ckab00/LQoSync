#!/usr/bin/env bash
set -euo pipefail

# Restore permissions after removing LQoSync.
# Prefer the original permission snapshot captured before adoption. If no
# snapshot exists, fall back to conservative managed LibreQoS file defaults.

INSTALL_DIR="${LQOSYNC_INSTALL_DIR:-/opt/LQoSync}"
LIBREQOS_SRC_DIR="${LIBREQOS_SRC_DIR:-/opt/libreqos/src}"
LQOSYNC_USER="${LQOSYNC_USER:-lqosync}"
SNAPSHOT_ROOT="${LQOSYNC_PERMISSION_SNAPSHOT_ROOT:-/root/lqosync_permission_snapshots}"
SNAPSHOT_DIR="${LQOSYNC_PERMISSION_SNAPSHOT_DIR:-}"
MODE="managed"

if [[ "${1:-}" == "--full" ]]; then
  MODE="full"
elif [[ "${1:-}" == "--managed" || -z "${1:-}" ]]; then
  MODE="managed"
else
  echo "Usage: sudo bash scripts/restore_libreqos_permissions.sh [--managed|--full]" >&2
  exit 2
fi

if [[ $EUID -ne 0 ]]; then
  echo "This script must be run as root. Use sudo." >&2
  exit 1
fi

if [[ ! -d "$LIBREQOS_SRC_DIR" ]]; then
  echo "LibreQoS source directory not found: $LIBREQOS_SRC_DIR" >&2
  exit 1
fi

find_snapshot() {
  if [[ -n "$SNAPSHOT_DIR" && -f "$SNAPSHOT_DIR/metadata.tsv" ]]; then
    printf '%s\n' "$SNAPSHOT_DIR"
    return 0
  fi
  if [[ -f "$SNAPSHOT_ROOT/latest.path" ]]; then
    local p=""
    read -r p < "$SNAPSHOT_ROOT/latest.path" || true
    if [[ -n "$p" && -f "$p/metadata.tsv" ]]; then
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

restore_from_snapshot() {
  local snap="$1"
  local metadata="$snap/metadata.tsv"
  local acl_restore="$snap/acl.restore"

  [[ -f "$metadata" ]] || return 1
  echo "[LQoSync] Restoring original permissions from snapshot: $snap"

  while IFS=$'\t' read -r type mode uid gid path; do
    [[ -z "${type:-}" || "$type" == \#* ]] && continue
    [[ "$type" == "missing" ]] && continue
    [[ -n "${path:-}" ]] || continue
    [[ -e "$path" || -L "$path" ]] || continue
    if [[ -n "${uid:-}" && -n "${gid:-}" ]]; then
      chown -h "$uid:$gid" "$path" 2>/dev/null || true
    fi
    if [[ "$type" != "l" && -n "${mode:-}" ]]; then
      chmod "$mode" "$path" 2>/dev/null || true
    fi
  done < "$metadata"

  if [[ -s "$acl_restore" ]]; then
    if command -v setfacl >/dev/null 2>&1; then
      setfacl --restore="$acl_restore" 2>/dev/null || true
      echo "[LQoSync] ACLs restored from original snapshot."
    else
      echo "[LQoSync] WARNING: setfacl not available; owner/mode restored but ACL restore skipped."
    fi
  fi

  echo "[LQoSync] Original permission snapshot restore complete."
  ls -ld "$INSTALL_DIR" "$LIBREQOS_SRC_DIR" 2>/dev/null || true
  ls -lah "$LIBREQOS_SRC_DIR/config.json" "$LIBREQOS_SRC_DIR/ShapedDevices.csv" "$LIBREQOS_SRC_DIR/network.json" 2>/dev/null || true
}

if snapshot="$(find_snapshot 2>/dev/null)"; then
  restore_from_snapshot "$snapshot"
  exit 0
fi

STAMP="$(date +%Y%m%d_%H%M%S)"
ACL_BACKUP="/root/lqosync_libreqos_acl_backup_${STAMP}.acl"

if command -v getfacl >/dev/null 2>&1; then
  {
    getfacl -p "$LIBREQOS_SRC_DIR" 2>/dev/null || true
    for f in config.json ShapedDevices.csv network.json; do
      [[ -e "$LIBREQOS_SRC_DIR/$f" ]] && getfacl -p "$LIBREQOS_SRC_DIR/$f" 2>/dev/null || true
    done
  } > "$ACL_BACKUP" || true
  echo "[LQoSync] ACL backup saved to: $ACL_BACKUP"
fi

if command -v setfacl >/dev/null 2>&1; then
  echo "[LQoSync] Removing ACL entries for user: $LQOSYNC_USER"
  setfacl -x "u:$LQOSYNC_USER" "$LIBREQOS_SRC_DIR" 2>/dev/null || true
  setfacl -d -x "u:$LQOSYNC_USER" "$LIBREQOS_SRC_DIR" 2>/dev/null || true
  for f in config.json ShapedDevices.csv network.json; do
    [[ -e "$LIBREQOS_SRC_DIR/$f" ]] && setfacl -x "u:$LQOSYNC_USER" "$LIBREQOS_SRC_DIR/$f" 2>/dev/null || true
  done
fi

if [[ "$MODE" == "full" ]]; then
  echo "[LQoSync] Restoring root:root ownership recursively under $LIBREQOS_SRC_DIR"
  chown -R root:root "$LIBREQOS_SRC_DIR"
else
  echo "[LQoSync] Restoring root:root ownership on managed LibreQoS files only"
  chown root:root "$LIBREQOS_SRC_DIR" 2>/dev/null || true
  for f in config.json ShapedDevices.csv network.json; do
    [[ -e "$LIBREQOS_SRC_DIR/$f" ]] && chown root:root "$LIBREQOS_SRC_DIR/$f" || true
  done
fi

# Conservative permissions after LQoSync removal. LibreQoS/manual root commands can
# still read these files. config.json may contain router credentials.
[[ -e "$LIBREQOS_SRC_DIR/config.json" ]] && chmod 600 "$LIBREQOS_SRC_DIR/config.json" || true
[[ -e "$LIBREQOS_SRC_DIR/ShapedDevices.csv" ]] && chmod 644 "$LIBREQOS_SRC_DIR/ShapedDevices.csv" || true
[[ -e "$LIBREQOS_SRC_DIR/network.json" ]] && chmod 644 "$LIBREQOS_SRC_DIR/network.json" || true
chmod 755 "$LIBREQOS_SRC_DIR" || true

echo "[LQoSync] LibreQoS permission restore complete."
ls -ld "$LIBREQOS_SRC_DIR" || true
ls -lah "$LIBREQOS_SRC_DIR/config.json" "$LIBREQOS_SRC_DIR/ShapedDevices.csv" "$LIBREQOS_SRC_DIR/network.json" 2>/dev/null || true
