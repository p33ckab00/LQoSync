#!/usr/bin/env bash
set -euo pipefail
CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"
NOW="$(date +%s)"
[ -f "$CONFIG_PATH" ] || { echo "FAIL: missing config: $CONFIG_PATH" >&2; exit 1; }
python3 - "$CONFIG_PATH" "$NOW" <<'PY'
from __future__ import annotations
import json, pathlib, sys, time, hashlib
cfg_path=pathlib.Path(sys.argv[1]); now=int(sys.argv[2]); cfg=json.loads(cfg_path.read_text()); rc=cfg.get('rust_core') or {}
root=pathlib.Path(str(rc.get('rust_authority_last_good_snapshot_dir') or '/opt/LQoSync/state/rust_authority_last_good'))
report_root=pathlib.Path(str(rc.get('rust_authority_rollback_drill_dir') or '/opt/LQoSync/state/rust_authority_rollback_drills')); report_root.mkdir(parents=True, exist_ok=True)
fail=[]; warn=[]; checks=[]
def check(name, ok, detail='', warning=False):
    print(f"[rust-authority-rollback-drill] {'ok' if ok else ('warn' if warning else 'fail')}|{name}|{detail}")
    checks.append({'name':name,'ok':bool(ok),'detail':detail,'warning':bool(warning)})
    if not ok: (warn if warning else fail).append(f'{name}: {detail}')
latest=None
if root.exists() and root.is_dir():
    dirs=sorted([d for d in root.iterdir() if d.is_dir()], key=lambda d:d.name, reverse=True); latest=dirs[0] if dirs else None
check('last_good.root', root.exists() and root.is_dir(), str(root)); check('last_good.latest', latest is not None, str(latest) if latest else 'none')
if latest is not None:
    m=latest/'MANIFEST.json'; check('last_good.manifest', m.exists(), str(m))
    if m.exists():
        try:
            manifest=json.loads(m.read_text()); check('manifest.schema', manifest.get('schema')=='lqosync.rust_authority_last_good.v1', str(manifest.get('schema')))
            included=manifest.get('included_files') or []; check('manifest.included_files', bool(included), str(included), warning=True)
            for name in included: check(f'snapshot_file.{name}', (latest/name).exists() and (latest/name).is_file(), str(latest/name))
        except Exception as exc: check('manifest.readable', False, str(exc))
digest=None
if latest is not None and (latest/'MANIFEST.json').exists():
    h=hashlib.sha256()
    for p in sorted(latest.iterdir()):
        if p.is_file(): h.update(p.name.encode()); h.update(b'\0'); h.update(p.read_bytes())
    digest=h.hexdigest()
out=report_root/(time.strftime('%Y%m%d_%H%M%S', time.gmtime(now))+'.json')
payload={'schema':'lqosync.rust_authority_rollback_drill.v1','created_epoch':now,'created_at':time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(now)),'status':'pass' if not fail else 'fail','mode':'non_destructive_validation_only','latest_snapshot':str(latest) if latest else None,'snapshot_digest':digest,'warnings':warn,'failures':fail,'checks':checks}
out.write_text(json.dumps(payload, indent=2)+'\n'); (report_root/'latest.json').write_text(json.dumps(payload, indent=2)+'\n')
print(f'rollback_drill_report={out}')
if fail:
    print('FAIL: rollback drill failed: ' + '; '.join(fail[:8]), file=sys.stderr); raise SystemExit(1)
print('PASS_WITH_WARNINGS: ' + '; '.join(warn[:8]) if warn else 'PASS: rollback drill passed')
PY
