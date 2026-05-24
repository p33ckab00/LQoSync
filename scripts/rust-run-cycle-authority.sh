#!/usr/bin/env bash
set -euo pipefail

CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"
MODE="${1:-manual}"
CORE_BIN="${LQOSYNC_CORE_BIN:-$(command -v lqosync-core 2>/dev/null || true)}"

if [[ -z "$CORE_BIN" ]] || [[ ! -x "$CORE_BIN" ]]; then
  echo "ERROR: lqosync-core not found or not executable" >&2
  exit 1
fi

TMP_OUTPUT="$(mktemp)"
trap 'rm -f "$TMP_OUTPUT"' EXIT

"$CORE_BIN" <<JSON | tee "$TMP_OUTPUT"
{"version":"1","op":"run-rust-cycle-authority","payload":{"config_path":"$CONFIG_PATH","mode":"$MODE","execute":true}}
JSON

python3 - "$TMP_OUTPUT" <<'PY'
import json
import sys

try:
    with open(sys.argv[1], "r", encoding="utf-8") as fh:
        data = json.load(fh)
except Exception as exc:
    print(f"ERROR: invalid JSON from run-rust-cycle-authority: {exc}", file=sys.stderr)
    raise SystemExit(1)

result = data.get("result") or {}
status = str(result.get("status") or "")

success_statuses = {"success", "no_changes", "dry_run_complete", "rust_run_cycle_transport_deferred"}
if bool(data.get("ok")) and status in success_statuses:
    raise SystemExit(0)

raise SystemExit(1)
PY
