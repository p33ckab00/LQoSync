#!/usr/bin/env bash
set -euo pipefail
fail=0
check_contains() {
  local f="$1" p="$2" label="$3"
  if grep -q "$p" "$f"; then
    echo "ok|$label|$f"
  else
    echo "MISSING[$label]: $f lacks $p" >&2
    fail=1
  fi
}
check_contains engine/rust_core.py "def rust_sync_plan_authority_gate" "gate-definition"
check_contains engine/run_cycle.py "rust_sync_plan_authority_gate" "gate-call"
check_contains engine/run_cycle.py "local hardening" "local-import-hardening"
check_contains docs/RUST_CORE_V823_SYNC_PLAN_GATE_IMPORT_HARDENING.md "name 'rust_sync_plan_authority_gate' is not defined" "operator-error-doc"
python3 -m py_compile engine/run_cycle.py engine/rust_core.py
if [ "$fail" -ne 0 ]; then
  exit 1
fi
echo "PASS: Rust sync-plan gate import hardening verified"
