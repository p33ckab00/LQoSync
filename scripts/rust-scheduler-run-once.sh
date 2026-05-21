#!/usr/bin/env bash
set -euo pipefail
CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"
MODE="${1:-manual}"
CORE_BIN="${LQOSYNC_CORE_BIN:-$(command -v lqosync-core 2>/dev/null || true)}"
if [ -z "$CORE_BIN" ] || [ ! -x "$CORE_BIN" ]; then
  echo "ERROR: lqosync-core not found or not executable" >&2
  exit 1
fi
"$CORE_BIN" <<JSON
{"version":"1","op":"scheduler-run-once","payload":{"config_path":"$CONFIG_PATH","mode":"$MODE","execute":true}}
JSON
