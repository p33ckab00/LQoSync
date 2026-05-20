#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 3 ]; then
  echo "Usage: sudo bash $0 <archive_root> <archive_item_name> <restore_destination>"
  echo "Example: sudo bash $0 /opt/LQoSync-archive/20260520-180000 lqosync_docker /home/pi/lqosync_docker"
  exit 1
fi

archive_root="$1"
item="$2"
dest="$3"
src="$archive_root/$item"

if [ "${CONFIRM_STALE_CODEBASE_RESTORE:-}" != "CONFIRM_STALE_CODEBASE_RESTORE" ]; then
  echo "ERROR: missing confirmation token."
  echo "Set: export CONFIRM_STALE_CODEBASE_RESTORE=CONFIRM_STALE_CODEBASE_RESTORE"
  exit 2
fi

if [ ! -e "$src" ]; then
  echo "ERROR: archive item not found: $src"
  exit 3
fi

case "$(readlink -f "$dest" 2>/dev/null || echo "$dest")" in
  /opt/LQoSync|/opt/LQoSync/*|/opt/libreqos|/opt/libreqos/*|/usr/local/bin/lqosync-core|/etc/systemd/system/lqosync-core.service)
    echo "ERROR: refusing restore into protected destination: $dest"
    exit 4
    ;;
esac

if [ -e "$dest" ]; then
  echo "ERROR: destination already exists: $dest"
  exit 5
fi

mkdir -p "$(dirname "$dest")"
echo "restoring: $src -> $dest"
mv "$src" "$dest"
echo "Restore complete."
