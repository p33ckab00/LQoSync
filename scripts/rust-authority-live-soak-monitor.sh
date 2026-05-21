#!/usr/bin/env bash
set -euo pipefail

# Non-mutating live-stable candidate monitor. Intended for post-promotion soak:
# verifies quarantine state, last-good snapshot, recovery bundle, preflight stamp,
# runtime state, and transaction journal readability.

CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"
NOW="$(date +%s)"
[ -f "$CONFIG_PATH" ] || { echo "FAIL: missing config: $CONFIG_PATH" >&2; exit 1; }
python3 - "$CONFIG_PATH" "$NOW" <<'PY'
from __future__ import annotations
import json, pathlib, sys
cfg_path = pathlib.Path(sys.argv[1])
now = int(sys.argv[2])
cfg = json.loads(cfg_path.read_text())
rc = cfg.get('rust_core') or {}
paths = cfg.get('paths') or {}
fail=[]
warn=[]

def emit(kind, name, detail=''):
    print(f'[rust-authority-live-soak] {kind}|{name}|{detail}')

def check(name, ok, detail='', warning=False):
    emit('ok' if ok else ('warn' if warning else 'fail'), name, detail)
    if not ok:
        (warn if warning else fail).append(f'{name}: {detail}')

check('flag.full_rust_backend_authority', rc.get('full_rust_backend_authority') is True, str(rc.get('full_rust_backend_authority')))
check('flag.python_mutation_fallback', rc.get('python_mutation_fallback') is False, str(rc.get('python_mutation_fallback')))
check('flag.rust_live_stable_candidate_enabled', rc.get('rust_live_stable_candidate_enabled') is True, str(rc.get('rust_live_stable_candidate_enabled')), warning=True)
check('flag.rust_authority_quarantine_enabled', rc.get('rust_authority_quarantine_enabled') is True, str(rc.get('rust_authority_quarantine_enabled')), warning=True)

qpath = pathlib.Path(str(rc.get('rust_authority_quarantine_state') or '/opt/LQoSync/state/rust_authority_quarantine.json'))
if qpath.exists():
    try:
        q = json.loads(qpath.read_text())
        check('quarantine.clear', not bool(q.get('active')), f"active={q.get('active')} status={q.get('status')}")
    except Exception as exc:
        check('quarantine.readable', False, str(exc))
else:
    check('quarantine.marker_absent', True, str(qpath))

for label, key, default in [
    ('recovery_bundle', 'rust_authority_recovery_bundle_dir', '/opt/LQoSync/state/rust_authority_recovery'),
    ('last_good_snapshot', 'rust_authority_last_good_snapshot_dir', '/opt/LQoSync/state/rust_authority_last_good'),
]:
    root = pathlib.Path(str(rc.get(key) or default))
    latest = None
    if root.exists() and root.is_dir():
        dirs = sorted([d for d in root.iterdir() if d.is_dir()], key=lambda d: d.name, reverse=True)
        latest = dirs[0] if dirs else None
    check(f'{label}.latest', latest is not None, str(latest) if latest else str(root), warning=(label == 'last_good_snapshot'))
    if latest is not None:
        check(f'{label}.manifest', (latest / 'MANIFEST.json').exists(), str(latest / 'MANIFEST.json'))

stamp = pathlib.Path(str(rc.get('rust_authority_preflight_stamp') or '/opt/LQoSync/state/rust_authority_preflight.json'))
max_age = int(rc.get('rust_authority_watchdog_max_preflight_age_seconds') or rc.get('rust_authority_preflight_max_age_seconds') or 900)
try:
    s = json.loads(stamp.read_text())
    age = max(0, now - int(s.get('created_epoch') or 0))
    check('preflight.status', s.get('status') == 'pass', str(s.get('status')))
    check('preflight.self_test_status', s.get('self_test_status') == 'ok', str(s.get('self_test_status')))
    check('preflight.fresh', max_age <= 0 or age <= max_age, f'age={age} max={max_age}')
except Exception as exc:
    check('preflight.readable', False, f'{stamp}: {exc}')

runtime = pathlib.Path(str(paths.get('runtime_state') or '/opt/LQoSync/state/runtime_state.json'))
if runtime.exists():
    try:
        rs = json.loads(runtime.read_text())
        check('runtime.last_error_clear', not bool(rs.get('last_error')), str(rs.get('last_error')), warning=True)
        check('runtime.scheduler_not_error', rs.get('scheduler_state') != 'error', str(rs.get('scheduler_state')), warning=True)
    except Exception as exc:
        check('runtime.readable', False, str(exc), warning=True)
else:
    check('runtime.exists', False, str(runtime), warning=True)

journal = pathlib.Path(str(paths.get('transaction_journal') or '/opt/LQoSync/logs/transaction_journal.jsonl'))
check('journal.parent', journal.parent.exists() and journal.parent.is_dir(), str(journal.parent))
if journal.exists():
    try:
        lines = [ln for ln in journal.read_text(errors='ignore').splitlines() if ln.strip()]
        check('journal.readable', True, f'entries={len(lines)}')
    except Exception as exc:
        check('journal.readable', False, str(exc), warning=True)
else:
    check('journal.exists', False, str(journal), warning=True)

if fail:
    print('FAIL: live soak monitor failed: ' + '; '.join(fail[:8]), file=sys.stderr)
    raise SystemExit(1)
if warn:
    print('PASS_WITH_WARNINGS: ' + '; '.join(warn[:8]))
else:
    print('PASS: Rust authority live soak monitor passed')
PY
