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

check_contains VERSION '2.151.0' version
check_exec scripts/rust-authority-watchdog.sh watchdog-script
check_exec scripts/verify-rust-authority-watchdog.sh verifier-script
check_contains config.json.example 'rust_authority_watchdog_enabled' config-watchdog-enabled
check_contains engine/config_loader.py 'rust_authority_watchdog_enabled' loader-watchdog-default
check_contains engine/run_cycle.py 'rust_authority_watchdog_required_failed' run-cycle-watchdog-fail-closed
check_contains scripts/promote-rust-full-authoritative-safe.sh 'rust_authority_watchdog_enabled' promote-watchdog
check_contains docs/RUST_CORE_V761_RUST_AUTHORITY_WATCHDOG.md 'Rust Authority Watchdog' docs
check_contains docs/DOCUMENTATION_INDEX.md 'RUST_CORE_V810_RUST_SCHEDULER_AUTHORITY.md' docs-index

if [ "$fail" -ne 0 ]; then
  echo "FAIL: Rust authority watchdog verification failed" >&2
  exit 1
fi

echo "PASS: Rust authority watchdog wiring verified"
