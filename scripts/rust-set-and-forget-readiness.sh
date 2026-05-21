#!/usr/bin/env bash
set -euo pipefail
CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"
WRITE_STAMP="false"; if [ "${1:-}" = "--write-stamp" ]; then WRITE_STAMP="true"; fi
NOW="$(date +%s)"
[ -f "$CONFIG_PATH" ] || { echo "FAIL: missing config: $CONFIG_PATH" >&2; exit 1; }
python3 - "$CONFIG_PATH" "$NOW" "$WRITE_STAMP" <<'PY'
from __future__ import annotations
import json, pathlib, sys, time
cfg_path=pathlib.Path(sys.argv[1]); now=int(sys.argv[2]); write_stamp=sys.argv[3]=='true'
cfg=json.loads(cfg_path.read_text()); rc=cfg.get('rust_core') or {}; paths=cfg.get('paths') or {}
evidence_path=pathlib.Path(str(rc.get('rust_set_and_forget_readiness_evidence') or '/opt/LQoSync/state/rust_set_and_forget_readiness.json')); evidence_path.parent.mkdir(parents=True, exist_ok=True)
fail=[]; warn=[]; checks={}
def check(name, ok, detail='', warning=False):
    print(f"[rust-set-and-forget-readiness] {'ok' if ok else ('warn' if warning else 'fail')}|{name}|{detail}")
    checks[name]={'ok':bool(ok),'detail':detail,'warning':bool(warning)}
    if not ok: (warn if warning else fail).append(f'{name}: {detail}')
check('full_rust_backend_authority', rc.get('full_rust_backend_authority') is True, str(rc.get('full_rust_backend_authority')))
check('python_mutation_fallback_disabled', rc.get('python_mutation_fallback') is False, str(rc.get('python_mutation_fallback')))
check('live_stable_candidate_enabled', rc.get('rust_live_stable_candidate_enabled') is True, str(rc.get('rust_live_stable_candidate_enabled')))
check('set_and_forget_candidate_enabled', rc.get('rust_set_and_forget_candidate_enabled') is True, str(rc.get('rust_set_and_forget_candidate_enabled')), warning=True)
q=pathlib.Path(str(rc.get('rust_authority_quarantine_state') or '/opt/LQoSync/state/rust_authority_quarantine.json'))
if q.exists():
    try: qd=json.loads(q.read_text()); check('quarantine_clear', not bool(qd.get('active')), f"active={qd.get('active')} status={qd.get('status')}")
    except Exception as exc: check('quarantine_readable', False, str(exc))
else: check('quarantine_clear', True, 'missing_ok')
stamp=pathlib.Path(str(rc.get('rust_authority_preflight_stamp') or '/opt/LQoSync/state/rust_authority_preflight.json'))
try:
    sp=json.loads(stamp.read_text()); max_age=int(rc.get('rust_authority_watchdog_max_preflight_age_seconds') or 900); age=max(0, now-int(sp.get('created_epoch') or 0))
    check('preflight_stamp', sp.get('status')=='pass' and sp.get('self_test_status')=='ok' and (max_age<=0 or age<=max_age), f"status={sp.get('status')} self={sp.get('self_test_status')} age={age}")
except Exception as exc: check('preflight_stamp', False, str(exc))
lgroot=pathlib.Path(str(rc.get('rust_authority_last_good_snapshot_dir') or '/opt/LQoSync/state/rust_authority_last_good')); latest_lg=None
if lgroot.exists():
    dirs=sorted([d for d in lgroot.iterdir() if d.is_dir()], key=lambda d:d.name, reverse=True); latest_lg=dirs[0] if dirs else None
check('last_good_snapshot', latest_lg is not None and (latest_lg/'MANIFEST.json').exists(), str(latest_lg) if latest_lg else str(lgroot))
rroot=pathlib.Path(str(rc.get('rust_authority_recovery_bundle_dir') or '/opt/LQoSync/state/rust_authority_recovery')); latest_rb=None
if rroot.exists():
    dirs=sorted([d for d in rroot.iterdir() if d.is_dir()], key=lambda d:d.name, reverse=True); latest_rb=dirs[0] if dirs else None
check('recovery_bundle', latest_rb is not None and (latest_rb/'MANIFEST.json').exists(), str(latest_rb) if latest_rb else str(rroot))
try: jad=json.loads(pathlib.Path('/opt/LQoSync/state/rust_authority_journal_audit.json').read_text()); check('journal_audit', jad.get('status')=='pass', str(jad.get('status')))
except Exception as exc: check('journal_audit', False, str(exc))
try: rdd=json.loads((pathlib.Path(str(rc.get('rust_authority_rollback_drill_dir') or '/opt/LQoSync/state/rust_authority_rollback_drills'))/'latest.json').read_text()); check('rollback_drill', rdd.get('status')=='pass', str(rdd.get('status')))
except Exception as exc: check('rollback_drill', False, str(exc))
runtime=pathlib.Path(str(paths.get('runtime_state') or '/opt/LQoSync/state/runtime_state.json'))
try:
    if runtime.exists():
        rs=json.loads(runtime.read_text()); check('live_soak_monitor', not bool(rs.get('last_error')) and rs.get('scheduler_state')!='error', f"last_error={rs.get('last_error')} scheduler={rs.get('scheduler_state')}")
    else: check('live_soak_monitor', True, 'runtime_state_missing_ok_before_first_cycle')
except Exception as exc: check('live_soak_monitor', False, str(exc))
status='pass' if not fail else 'fail'; payload={'schema':'lqosync.rust_set_and_forget_readiness.v1','created_epoch':now,'created_at':time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(now)),'status':status,'warnings':warn,'failures':fail,'checks':checks}
if write_stamp:
    evidence_path.write_text(json.dumps(payload, indent=2)+'\n'); print(f'readiness_evidence={evidence_path}')
else: print(json.dumps(payload, indent=2))
if fail:
    print('FAIL: set-and-forget readiness failed: ' + '; '.join(fail[:8]), file=sys.stderr); raise SystemExit(1)
print('PASS_WITH_WARNINGS: ' + '; '.join(warn[:8]) if warn else 'PASS: set-and-forget readiness passed')
PY
