#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
fail=0
check_contains() {
  local path="$1" pattern="$2" label="$3"
  if [ ! -f "$path" ]; then
    echo "MISSING: $path" >&2
    fail=1
    return
  fi
  if ! grep -q "$pattern" "$path"; then
    echo "MISSING[$label]: $path lacks pattern: $pattern" >&2
    fail=1
  else
    echo "ok|$label|$path"
  fi
}

check_contains config.json.example '"full_rust_backend_authority": true' config-default
check_contains config.json.example '"python_mutation_fallback": false' no-python-mutation-fallback
check_contains config.json.example '"transaction_authority": "rust_full_authoritative"' transaction-authority
check_contains config.json.example '"execute_apply_manifest": true' execute-apply-manifest
check_contains config.json.example '"allow_rust_file_writes": true' file-write-authority
check_contains config.json.example '"allow_rust_libreqos_apply": true' libreqos-apply-authority
check_contains config.json.example '"collector_output_authority": "rust_validate_all"' collector-rust-validation
check_contains engine/config_loader.py 'full_rust_backend_authority' loader-default
check_contains rust/lqosync-core/src/rust_run_cycle_authority.rs 'rust_full_authority_missing_file_write_flags' fail-closed-file-flag
check_contains rust/lqosync-core/src/rust_run_cycle_authority.rs 'rust_full_authority_file_write_not_executed' fail-closed-file-write
check_contains rust/lqosync-core/src/rust_run_cycle_authority.rs 'rust_full_authority_libreqos_apply_not_executed' fail-closed-libreqos
check_contains scripts/promote-rust-full-authoritative-safe.sh 'python_mutation_fallback' promotion-lock
check_contains docs/RUST_CORE_V758_FULL_RUST_AUTHORITY_LOCKDOWN.md 'Full Rust Authority Lockdown' docs

if [ "$fail" -ne 0 ]; then
  echo "FAIL: Rust full authority lockdown verification failed" >&2
  exit 1
fi

echo "PASS: Rust full authority lockdown wiring verified"
