#!/usr/bin/env bash
set -euo pipefail

# Manage the Rust authority quarantine marker. This is intentionally small and
# non-destructive: it writes/removes only the quarantine JSON marker, never
# generated LibreQoS files.

CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"
ACTION="${1:-status}"
REASON="${2:-operator_requested}"
NOW="$(date +%s)"

[ -f "$CONFIG_PATH" ] || { echo "FAIL: missing config: $CONFIG_PATH" >&2; exit 1; }

python3 - "$CONFIG_PATH" "$ACTION" "$REASON" "$NOW" <<'PY'
from __future__ import annotations
import json, pathlib, sys, time
cfg_path = pathlib.Path(sys.argv[1])
action = sys.argv[2]
reason = sys.argv[3]
now = int(sys.argv[4])
cfg = json.loads(cfg_path.read_text())
rc = cfg.get('rust_core') or {}
qpath = pathlib.Path(str(rc.get('rust_authority_quarantine_state') or '/opt/LQoSync/state/rust_authority_quarantine.json'))

def read_state():
    if not qpath.exists():
        return {'active': False, 'status': 'missing', 'path': str(qpath)}
    try:
        data = json.loads(qpath.read_text())
        data['path'] = str(qpath)
        return data
    except Exception as exc:
        return {'active': True, 'status': 'unreadable', 'path': str(qpath), 'error': str(exc)}

if action == 'status':
    print(json.dumps(read_state(), indent=2))
    raise SystemExit(0)
if action == 'enter':
    qpath.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        'schema': 'lqosync.rust_authority_quarantine.v1',
        'active': True,
        'status': reason,
        'created_epoch': now,
        'created_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(now)),
        'reason': reason,
        'manual': True,
    }
    qpath.write_text(json.dumps(payload, indent=2) + '\n')
    print(f'PASS: quarantine entered: {qpath}')
    raise SystemExit(0)
if action in {'clear', 'exit'}:
    qpath.parent.mkdir(parents=True, exist_ok=True)
    old = read_state()
    payload = {
        'schema': 'lqosync.rust_authority_quarantine.v1',
        'active': False,
        'status': 'cleared',
        'cleared_epoch': now,
        'cleared_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(now)),
        'reason': reason,
        'previous': old,
    }
    qpath.write_text(json.dumps(payload, indent=2) + '\n')
    print(f'PASS: quarantine cleared: {qpath}')
    raise SystemExit(0)
print('Usage: rust-authority-quarantine.sh status|enter|clear [reason]', file=sys.stderr)
raise SystemExit(2)
PY
