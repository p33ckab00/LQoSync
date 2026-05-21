#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
fail=0
check_file(){ [ -e "$1" ] && echo "ok|file|$1" || { echo "MISSING: $1" >&2; fail=1; }; }
contains(){ local f="$1" p="$2" label="$3"; check_file "$f"; grep -q -- "$p" "$f" && echo "ok|$label|$f" || { echo "MISSING[$label]: $f lacks $p" >&2; fail=1; }; }
not_contains(){ local f="$1" p="$2" label="$3"; check_file "$f"; if grep -q -- "$p" "$f"; then echo "FORBIDDEN[$label]: $f contains $p" >&2; fail=1; else echo "ok|absent-$label|$f"; fi; }

for f in \
  scheduler/runner.py \
  engine/rust_scheduler.py \
  docs/FULL_RUST_BACKEND_BOUNDARY.md \
  docs/RUST_CORE_V820_FULL_RUST_DAEMON_BOUNDARY.md \
  docs/lqosync_workflow_architecture.svg \
  lqosync_workflow_architecture.svg; do
  check_file "$f"
done

contains scheduler/runner.py 'RustAuthorityScheduler' scheduler-facade
contains scheduler/runner.py 'Python scheduler authority has been removed' scheduler-hard-boundary
not_contains scheduler/runner.py 'def _loop' python-loop-method
not_contains scheduler/runner.py 'threading.Thread(target=self._loop' python-loop-thread
not_contains scheduler/runner.py 'python_legacy' python-legacy-state
not_contains scheduler/runner.py 'from engine.run_cycle import run_cycle' direct-run-cycle-import
contains config.json.example '"allow_python_scheduler": false' python-scheduler-disabled
contains config.json.example '"python_mutation_fallback": false' python-mutation-fallback-disabled
contains engine/config_loader.py 'scheduler.setdefault("allow_python_scheduler", False)' loader-python-scheduler-disabled
contains engine/config_loader.py 'rust_core.setdefault("python_mutation_fallback", False)' loader-python-mutation-disabled
contains docs/FULL_RUST_BACKEND_BOUNDARY.md 'Rust authority daemon = backend authority' docs-boundary
contains docs/FULL_RUST_BACKEND_BOUNDARY.md 'not being rewritten to Django' docs-not-django
contains docs/lqosync_workflow_architecture.svg 'Python scheduler loop removed' svg-loop-removed
not_contains docs/lqosync_workflow_architecture.svg 'Python legacy loop' svg-no-legacy-loop
contains README.md 'v8.2.0 Full Rust daemon boundary' readme-v820
contains docs/DOCUMENTATION_INDEX.md 'RUST_CORE_V820_FULL_RUST_DAEMON_BOUNDARY.md' docs-index-v820
contains docs/docs_manifest.json 'rust.core_v820_full_rust_daemon_boundary' docs-manifest-v820

python3 - <<'PY'
import json
cfg=json.load(open('config.json.example'))
assert cfg['scheduler']['engine'] == 'rust'
assert cfg['scheduler']['allow_python_scheduler'] is False
assert cfg['rust_core']['python_mutation_fallback'] is False
assert cfg['rust_core']['full_rust_backend_authority'] is True
for key, value in cfg['rust_core'].items():
    if 'python_fallback' in key or 'python_backend_fallback' in key or 'python_fallback_backup' in key:
        assert value is False, f'{key} must be false in stable Rust boundary defaults'
print('ok|json|full Rust daemon boundary config')
PY

if [ "$fail" -ne 0 ]; then
  echo "FAIL: full Rust daemon boundary verification failed" >&2
  exit 1
fi
echo "PASS: full Rust daemon boundary cleanup verified"
