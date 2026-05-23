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
check_exec() {
  local path="$1" label="$2"
  if [ -x "$path" ]; then
    echo "ok|$label|$path"
  else
    echo "MISSING-EXEC[$label]: $path" >&2
    fail=1
  fi
}

check_contains VERSION '2.152.' version
check_exec scripts/rust-full-authority-preflight.sh preflight-script
check_exec scripts/rust-full-authority-recovery-bundle.sh recovery-bundle-script
check_exec scripts/verify-rust-authority-supervisor.sh verifier-script
check_contains config.json.example 'full_rust_authority_supervisor_enabled' config-supervisor
check_contains config.json.example 'rust_authority_preflight_stamp' config-preflight-stamp
check_contains engine/config_loader.py 'full_rust_authority_supervisor_enabled' loader-default
check_contains rust/lqosync-core/src/rust_run_cycle_authority.rs 'rust_authority_preflight_required_failed' run-cycle-preflight-fail-closed
check_contains scripts/promote-rust-full-authoritative-safe.sh 'rust-full-authority-preflight.sh' promote-preflight
check_contains scripts/promote-rust-full-authoritative-safe.sh 'rust-full-authority-recovery-bundle.sh' promote-recovery-bundle
check_contains docs/RUST_CORE_V760_RUST_AUTHORITY_SUPERVISOR.md 'Rust Authority Supervisor' docs
check_contains docs/DOCUMENTATION_INDEX.md 'RUST_CORE_V810_RUST_SCHEDULER_AUTHORITY.md' docs-index

if [ "$fail" -ne 0 ]; then
  echo "FAIL: Rust authority supervisor verification failed" >&2
  exit 1
fi

echo "PASS: Rust authority supervisor wiring verified"
