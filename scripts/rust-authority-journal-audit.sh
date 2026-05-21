#!/usr/bin/env bash
set -euo pipefail
CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"
NOW="$(date +%s)"
[ -f "$CONFIG_PATH" ] || { echo "FAIL: missing config: $CONFIG_PATH" >&2; exit 1; }
python3 - "$CONFIG_PATH" "$NOW" <<'PY'
from __future__ import annotations
import json, pathlib, sys, time
cfg_path = pathlib.Path(sys.argv[1]); now = int(sys.argv[2])
cfg = json.loads(cfg_path.read_text()); rc = cfg.get('rust_core') or {}; paths = cfg.get('paths') or {}
journal = pathlib.Path(str(paths.get('transaction_journal') or '/opt/LQoSync/logs/transaction_journal.jsonl'))
state_dir = pathlib.Path(str(rc.get('rust_set_and_forget_state_dir') or '/opt/LQoSync/state')); state_dir.mkdir(parents=True, exist_ok=True)
out = state_dir / 'rust_authority_journal_audit.json'
fail=[]; warn=[]; checks=[]
def check(name, ok, detail='', warning=False):
    print(f"[rust-authority-journal-audit] {'ok' if ok else ('warn' if warning else 'fail')}|{name}|{detail}")
    checks.append({'name': name, 'ok': bool(ok), 'detail': detail, 'warning': bool(warning)})
    if not ok: (warn if warning else fail).append(f'{name}: {detail}')
check('journal.parent', journal.parent.exists() and journal.parent.is_dir(), str(journal.parent))
entries=[]
if journal.exists():
    try:
        for i, line in enumerate(journal.read_text(errors='ignore').splitlines(), 1):
            if not line.strip(): continue
            try: entries.append(json.loads(line))
            except Exception as exc:
                check('journal.line_json', False, f'line={i} error={exc}'); break
        else: check('journal.jsonl_parse', True, f'entries={len(entries)}')
    except Exception as exc: check('journal.readable', False, str(exc))
else: check('journal.exists', False, str(journal), warning=True)
if entries:
    last=entries[-1]
    check('journal.latest_has_schema_or_kind', bool(last.get('schema') or last.get('kind') or last.get('operation')), str(last)[:300])
    bad=[e for e in entries[-10:] if isinstance(e, dict) and str(e.get('status','')).lower() in {'failed','error'}]
    check('journal.no_recent_failed_entries', not bad, f'bad_recent={len(bad)}')
payload={'schema':'lqosync.rust_authority_journal_audit.v1','created_epoch':now,'created_at':time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(now)),'status':'pass' if not fail else 'fail','warnings':warn,'failures':fail,'journal_path':str(journal),'entry_count':len(entries),'checks':checks}
out.write_text(json.dumps(payload, indent=2)+'\n')
print(f'journal_audit_report={out}')
if fail:
    print('FAIL: Rust authority journal audit failed: ' + '; '.join(fail[:8]), file=sys.stderr); raise SystemExit(1)
print('PASS_WITH_WARNINGS: ' + '; '.join(warn[:8]) if warn else 'PASS: Rust authority journal audit passed')
PY
