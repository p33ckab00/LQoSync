#!/usr/bin/env bash
set -euo pipefail

# Create a last-good snapshot for Rust live-stable operation. This is a local
# copy of current config/generated files and runtime state for operator rollback
# decision support. It does not restore or mutate live LibreQoS files.

CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"
[ -f "$CONFIG_PATH" ] || { echo "FAIL: missing config: $CONFIG_PATH" >&2; exit 1; }
python3 - "$CONFIG_PATH" <<'PY'
from __future__ import annotations
import hashlib, json, pathlib, shutil, sys, time
cfg_path = pathlib.Path(sys.argv[1])
cfg = json.loads(cfg_path.read_text())
rc = cfg.get('rust_core') or {}
paths = cfg.get('paths') or {}
root = pathlib.Path(str(rc.get('rust_authority_last_good_snapshot_dir') or '/opt/LQoSync/state/rust_authority_last_good'))
ts = time.strftime('%Y%m%d_%H%M%S', time.gmtime())
out = root / ts
out.mkdir(parents=True, exist_ok=True)
manifest = {
    'schema': 'lqosync.rust_authority_last_good.v1',
    'created_epoch': int(time.time()),
    'created_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
    'config_path': str(cfg_path),
    'included': [],
    'missing': [],
}
items = [
    ('config.json', cfg_path),
    ('ShapedDevices.csv', pathlib.Path(str(paths.get('shaped_devices_csv') or '/opt/libreqos/src/ShapedDevices.csv'))),
    ('network.json', pathlib.Path(str(paths.get('network_json') or '/opt/libreqos/src/network.json'))),
    ('runtime_state.json', pathlib.Path(str(paths.get('runtime_state') or '/opt/LQoSync/state/runtime_state.json'))),
    ('transaction_journal.jsonl', pathlib.Path(str(paths.get('transaction_journal') or '/opt/LQoSync/logs/transaction_journal.jsonl'))),
]
for name, src in items:
    if src.exists() and src.is_file():
        dst = out / name
        shutil.copy2(src, dst)
        digest = hashlib.sha256(dst.read_bytes()).hexdigest()
        manifest['included'].append({'name': name, 'source': str(src), 'sha256': digest, 'bytes': dst.stat().st_size})
    else:
        manifest['missing'].append({'name': name, 'source': str(src)})
(out / 'MANIFEST.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(f'PASS: last-good snapshot created: {out}')
PY
