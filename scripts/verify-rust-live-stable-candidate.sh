#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
fail=0
check_contains() {
  local path="$1" pattern="$2" label="$3"
  if [ ! -f "$path" ]; then
    echo "MISSING[$label]: $path" >&2
    fail=1
    return
  fi
  if grep -q "$pattern" "$path"; then
    echo "ok|$label|$path"
  else
    echo "MISSING[$label]: $path lacks $pattern" >&2
    fail=1
  fi
}
check_exec() {
  local path="$1" label="$2"
  if [ -x "$path" ]; then
    echo "ok|$label|$path"
  else
    echo "MISSING_EXEC[$label]: $path" >&2
    fail=1
  fi
}
check_contains VERSION '2.152.' version
check_contains config.json.example 'rust_live_stable_candidate_enabled' config-live-stable
check_contains config.json.example 'rust_authority_quarantine_enabled' config-quarantine
check_contains engine/config_loader.py 'rust_live_stable_candidate_enabled' loader-live-stable
check_contains rust/lqosync-core/src/rust_run_cycle_authority.rs 'rust_live_stable_gate_failed' run-cycle-live-stable-gate
check_contains rust/lqosync-core/src/rust_run_cycle_authority.rs 'rust_authority_quarantine.v1' run-cycle-quarantine-marker
check_contains rust/lqosync-core/src/rust_run_cycle_authority.rs 'rust_authority_last_good.v1' run-cycle-last-good
check_exec scripts/rust-authority-quarantine.sh quarantine-script
check_exec scripts/rust-authority-last-good-snapshot.sh last-good-script
check_exec scripts/rust-authority-live-soak-monitor.sh soak-monitor-script
check_contains scripts/promote-rust-full-authoritative-safe.sh 'rust_live_stable_candidate_enabled' promotion-live-stable
check_contains docs/RUST_CORE_V770_LIVE_STABLE_CANDIDATE.md 'v7.7.0' docs
if [ "$fail" -ne 0 ]; then
  echo "FAIL: Rust live-stable candidate wiring check failed" >&2
  exit 1
fi
echo "PASS: Rust live-stable candidate wiring verified"
