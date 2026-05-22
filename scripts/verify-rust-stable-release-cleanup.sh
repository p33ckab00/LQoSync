#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
fail=0
ok(){ echo "ok|$1|$2"; }
bad(){ echo "FAIL|$1|$2" >&2; fail=1; }
contains(){ local f="$1" p="$2" k="$3"; grep -q "$p" "$f" && ok "$k" "$f" || bad "$k" "$f missing $p"; }

version="$(tr -d '\n' < VERSION 2>/dev/null || true)"
case "$version" in
  2.152.*) ok version "$version" ;;
  *) bad version "expected 2.152.x v8.2 stable, got $version" ;;
esac

python3 - <<'PY'
import json, sys
cfg=json.load(open('config.json.example'))
rc=cfg.get('rust_core', {})
required_true = [
 'enabled','prefer_daemon','enforce_validation','enforce_sync_plan','fail_closed_when_enforced',
 'execute_apply_manifest','allow_rust_file_writes','allow_rust_libreqos_apply',
 'append_transaction_journal','allow_transaction_journal_writes','require_authority_readiness',
 'full_rust_backend_authority','full_rust_authority_supervisor_enabled','require_rust_authority_preflight',
 'rust_authority_watchdog_enabled','rust_live_stable_candidate_enabled','rust_set_and_forget_candidate_enabled',
 'rust_authority_quarantine_enabled','require_rust_authority_recovery_bundle','fail_closed_without_rust_authority',
 'require_rust_authoritative_transaction','require_collector_rust_validation','rust_stable_release',
 'python_backend_authority_removed','legacy_python_mutation_cleanup_complete'
]
errors=[]
for key in required_true:
    if rc.get(key) is not True:
        errors.append(f'{key}={rc.get(key)!r}')
if rc.get('python_mutation_fallback') is not False:
    errors.append(f"python_mutation_fallback={rc.get('python_mutation_fallback')!r}")
if rc.get('transaction_authority') != 'rust_full_authoritative':
    errors.append('transaction_authority not rust_full_authoritative')
if rc.get('collector_output_authority') != 'rust_validate_all':
    errors.append('collector_output_authority not rust_validate_all')
if rc.get('python_runtime_role') != 'flask_webui_shell_only':
    errors.append('python_runtime_role not Flask shell-only')
if errors:
    print('FAIL|config|' + '; '.join(errors))
    sys.exit(1)
print('ok|config|stable Rust authority defaults')
PY

contains engine/config_loader.py 'rust_stable_release' config-loader-stable
contains engine/run_cycle.py 'rust_set_and_forget_gate_failed' runtime-set-and-forget-gate
contains scripts/promote-rust-full-authoritative-safe.sh 'rust-set-and-forget-readiness.sh' promotion-readiness
contains docs/RUST_CORE_V800_STABLE_RUST_BACKEND_CLEANUP.md 'Python is .*not.* allowed to silently take over production mutation' stable-doc-boundary
contains docs/FULL_RUST_STABLE_OPERATIONS.md 'Rust authority daemon' stable-ops-guide
contains scripts/rust-stable-codebase-cleanup-inventory.sh 'flask_webui_shell_only' cleanup-inventory
contains rust/lqosync-core/src/self_test.rs 'build-python-legacy-retirement-inventory' legacy-retirement-op
contains docs/RUST_CORE_V826_PYTHON_LEGACY_RETIREMENT_INVENTORY.md 'delete_allowed=false' legacy-retirement-doc

if find . -path './.git' -prune -o -type f \( -name '*.orig' -o -name '*.rej' -o -name '*.bak' -o -name '*~' -o -name '*.pre_*' \) -print | grep -q .; then
  bad stale-files "stale backup/reject files found"
else
  ok stale-files "no stale backup/reject files"
fi

if [ "$fail" -ne 0 ]; then
  echo "FAIL: stable Rust cleanup verification failed" >&2
  exit 1
fi
echo "PASS: stable Rust cleanup verification passed"
