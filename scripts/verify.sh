#!/usr/bin/env bash
set -Eeuo pipefail
APP_DIR="${APP_DIR:-/opt/libreqos/src}"
SERVICE_NAME="${SERVICE_NAME:-lqosync-updatecsv}"
cd "${APP_DIR}"
echo "==> Python syntax"
python3 -m py_compile updatecsv.py

echo "==> JSON syntax"
python3 -m json.tool config.json >/dev/null

echo "==> Logical-core config"
python3 - <<'PY'
import json, os
from pathlib import Path
cfg = json.loads(Path('config.json').read_text(encoding='utf-8'))
net = cfg.get('network', {})
logical = net.get('logical_roots', {})
cores = os.cpu_count() or int(logical.get('fallback_count', 4))
reserve = int(logical.get('reserve_cores', 2))
count = max(1, cores - reserve)
count = max(int(logical.get('min_count', 1)), count)
count = min(int(logical.get('max_count', count)), count)
print('network.mode =', net.get('mode'))
print('logical_roots.enabled =', logical.get('enabled'))
print('cpu cores =', cores)
print('reserve cores =', reserve)
print('computed buckets =', count)
print('state file =', logical.get('state_file'))
assert net.get('mode') == 'logical_core_root'
assert logical.get('enabled') is True
PY

echo "==> Service status"
if systemctl list-unit-files "${SERVICE_NAME}.service" >/dev/null 2>&1; then
  systemctl --no-pager --full status "${SERVICE_NAME}" || true
else
  echo "WARN: ${SERVICE_NAME}.service not found"
fi
