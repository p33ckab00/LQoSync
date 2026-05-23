#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
fail=0
check_file(){ [ -e "$1" ] && echo "ok|file|$1" || { echo "MISSING: $1" >&2; fail=1; }; }
contains(){ local f="$1" p="$2" label="$3"; check_file "$f"; grep -q -- "$p" "$f" && echo "ok|$label|$f" || { echo "MISSING[$label]: $f lacks $p" >&2; fail=1; }; }

for f in \
  engine/rust_scheduler.py \
  scheduler/runner.py \
  scripts/rust-run-cycle-authority.sh \
  scripts/rust-scheduler-authority-status.sh \
  scripts/rust-scheduler-run-once.sh \
  rust/lqosync-core/src/rust_scheduler.rs \
  docs/RUST_CORE_V810_RUST_SCHEDULER_AUTHORITY.md \
  docs/PROJECT_CANONICAL_ARCHITECTURE.md \
  docs/FLASK_UI_SHELL.md; do
  check_file "$f"
done
contains config.json.example '"engine": "rust"' scheduler-engine
contains config.json.example '"allow_python_scheduler": false' python-scheduler-retired
contains engine/config_loader.py 'scheduler.setdefault("engine", "rust")' loader-scheduler-engine
contains engine/config_loader.py 'flask_webui_shell_only' flask-role
contains scheduler/runner.py 'RustAuthorityScheduler' scheduler-proxy
contains scheduler/runner.py 'Python no longer starts the production scheduler loop' python-loop-retired
contains engine/rust_scheduler.py 'scheduler-run-once' rust-run-once-wrapper
contains scripts/rust-run-cycle-authority.sh 'run-rust-cycle-authority' native-run-cycle-script
contains rust/lqosync-core/src/self_test.rs 'OP_SCHEDULER_RUN_ONCE' selftest-operation
contains rust/lqosync-core/src/self_test.rs 'OP_RUN_RUST_CYCLE_AUTHORITY' selftest-run-cycle-operation
contains rust/lqosync-core/src/main.rs 'scheduler-run-once' main-operation
contains rust/lqosync-core/src/main.rs 'run-rust-cycle-authority' main-run-cycle-operation
contains rust/lqosync-core/src/main.rs 'Enable the Rust scheduler authority loop' daemon-scheduler-flag
contains config.json.example 'rust-run-cycle-authority.sh manual' manual-command-default
contains config.json.example 'rust-run-cycle-authority.sh scheduled' scheduled-command-default
contains systemd/lqosync-core.service --scheduler systemd-scheduler-enabled
contains README.md 'Flask WebUI shell' readme-boundary
contains README.md 'not Django' readme-not-django
contains docs/DOCUMENTATION_INDEX.md 'RUST_CORE_V810_RUST_SCHEDULER_AUTHORITY.md' docs-index
contains docs/docs_manifest.json 'scheduler_authority' docs-manifest

python3 - <<'PY'
import json
cfg=json.load(open('config.json.example'))
assert cfg['scheduler']['engine']=='rust'
assert cfg['scheduler']['allow_python_scheduler'] is False
assert cfg['scheduler']['manual_run_command'].endswith('rust-run-cycle-authority.sh manual')
assert cfg['scheduler']['rust_run_cycle_command'].endswith('rust-run-cycle-authority.sh scheduled')
assert cfg['rust_core']['rust_scheduler_authority'] is True
assert cfg['rust_core']['native_run_cycle_authority_enabled'] is True
assert cfg['rust_core']['native_run_cycle_authority_python_fallback'] is False
assert cfg['rust_core']['python_runtime_role']=='flask_webui_shell_only'
print('ok|json|scheduler authority config')
PY

if [ "$fail" -ne 0 ]; then
  echo "FAIL: Rust scheduler authority verification failed" >&2
  exit 1
fi
echo "PASS: Rust scheduler authority wiring verified"
