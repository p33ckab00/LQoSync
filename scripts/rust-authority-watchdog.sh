#!/usr/bin/env bash
set -euo pipefail

# Non-mutating Rust authority watchdog.
# Verifies the promoted full-authority runtime has the evidence required to run safely:
#   - Rust full-authority config flags
#   - fresh preflight stamp
#   - recovery bundle with MANIFEST.json
#   - transaction journal parent path and authority flags

INSTALL_DIR="${LQOSYNC_INSTALL_DIR:-/opt/LQoSync}"
CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"
NOW_EPOCH="$(date +%s)"
FAIL=0
WARN=0

log() { echo "[rust-authority-watchdog] $*"; }
warn() { echo "[rust-authority-watchdog] WARN: $*"; WARN=$((WARN+1)); }
fail_check() { echo "[rust-authority-watchdog] FAIL: $*" >&2; FAIL=$((FAIL+1)); }

[ -f "$CONFIG_PATH" ] || { echo "FAIL: missing config: $CONFIG_PATH" >&2; exit 1; }

python3 - "$CONFIG_PATH" "$NOW_EPOCH" <<'PY' || FAIL=$((FAIL+1))
from __future__ import annotations
import json, os, pathlib, sys, time
cfg_path = pathlib.Path(sys.argv[1])
now = int(sys.argv[2])
cfg = json.loads(cfg_path.read_text())
rc = cfg.get('rust_core') or {}
paths = cfg.get('paths') or {}
failures=[]
warnings=[]

def emit(kind, name, detail=''):
    print(f"[rust-authority-watchdog] {kind}|{name}|{detail}")

def check(name, ok, detail=''):
    emit('ok' if ok else 'fail', name, detail)
    if not ok:
        failures.append(f"{name}: {detail}")

required_flags = {
    'full_rust_backend_authority': True,
    'python_mutation_fallback': False,
    'require_rust_authoritative_transaction': True,
    'execute_apply_manifest': True,
    'allow_rust_file_writes': True,
    'allow_rust_libreqos_apply': True,
    'append_transaction_journal': True,
    'allow_transaction_journal_writes': True,
}
for key, expected in required_flags.items():
    check(f'flag.{key}', rc.get(key) is expected, f"expected={expected} actual={rc.get(key)!r}")

if rc.get('rust_authority_watchdog_enabled') is not True:
    warnings.append('rust_authority_watchdog_enabled is not true; runtime gate is not active')
    emit('warn', 'flag.rust_authority_watchdog_enabled', str(rc.get('rust_authority_watchdog_enabled')))
else:
    emit('ok', 'flag.rust_authority_watchdog_enabled', 'true')

stamp_path = pathlib.Path(str(rc.get('rust_authority_preflight_stamp') or paths.get('rust_authority_preflight_stamp') or '/opt/LQoSync/state/rust_authority_preflight.json'))
max_age = int(rc.get('rust_authority_watchdog_max_preflight_age_seconds') or rc.get('rust_authority_preflight_max_age_seconds') or 900)
try:
    stamp = json.loads(stamp_path.read_text())
    age = max(0, now - int(stamp.get('created_epoch') or 0))
    check('preflight.status', stamp.get('status') == 'pass', str(stamp.get('status')))
    check('preflight.self_test_status', stamp.get('self_test_status') == 'ok', str(stamp.get('self_test_status')))
    check('preflight.fresh', max_age <= 0 or age <= max_age, f'age={age}s max={max_age}s path={stamp_path}')
except Exception as exc:
    check('preflight.readable', False, f'{stamp_path}: {exc}')

root = pathlib.Path(str(rc.get('rust_authority_recovery_bundle_dir') or '/opt/LQoSync/state/rust_authority_recovery'))
latest = None
if root.exists() and root.is_dir():
    dirs = sorted([p for p in root.iterdir() if p.is_dir()], key=lambda p: p.name, reverse=True)
    latest = dirs[0] if dirs else None
check('recovery.root', root.exists() and root.is_dir(), str(root))
check('recovery.latest', latest is not None, str(latest) if latest else 'none')
if latest is not None:
    manifest = latest / 'MANIFEST.json'
    check('recovery.manifest', manifest.exists() and manifest.is_file(), str(manifest))

journal = pathlib.Path(str(paths.get('transaction_journal') or '/opt/LQoSync/logs/transaction_journal.jsonl'))
check('journal.parent', journal.parent.exists() and journal.parent.is_dir(), str(journal.parent))
check('journal.parent_writable', journal.parent.exists() and os.access(journal.parent, os.W_OK), str(journal.parent))

if failures:
    print('FAIL: Rust authority watchdog failed: ' + '; '.join(failures[:8]), file=sys.stderr)
    raise SystemExit(1)
if warnings:
    print('WARN: ' + '; '.join(warnings), file=sys.stderr)
print('PASS: Rust authority watchdog checks passed')
PY

if [ "$FAIL" -ne 0 ]; then
  echo "FAIL: Rust authority watchdog failed ($FAIL failure groups, $WARN warnings)" >&2
  exit 1
fi

echo "PASS: Rust authority watchdog passed ($WARN warnings)"
