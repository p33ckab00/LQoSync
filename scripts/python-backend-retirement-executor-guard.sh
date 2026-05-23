#!/usr/bin/env bash
set -euo pipefail

DRY_RUN=1
if [[ "${1:-}" == "--execute" ]]; then
  DRY_RUN=0
fi

CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"

fail() {
  local message="$1"
  local code="$2"
  echo "Refusing Python backend retirement: ${message}" >&2
  exit "$code"
}

check_runtime_config_for_python_bridge() {
  if [[ ! -f "$CONFIG_PATH" ]]; then
    echo "Skip config bridge check: missing ${CONFIG_PATH}"
    return 0
  fi

  local bridge_line=""
  if bridge_line="$(python3 - "$CONFIG_PATH" <<'PY'
import json
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as fh:
    cfg = json.load(fh)

scheduler = cfg.get("scheduler") or {}
for key in ("rust_run_cycle_command", "manual_run_command"):
    value = str(scheduler.get(key) or "")
    if any(token in value for token in ("scripts/run_cycle_once.py", "venv/bin/python", "engine/run_cycle.py")):
        print(f"{key}={value}")
        sys.exit(10)
rust_core = cfg.get("rust_core") or {}
if not bool(rust_core.get("native_run_cycle_authority_enabled", False)):
    print("rust_core.native_run_cycle_authority_enabled=false")
    sys.exit(11)
if bool(rust_core.get("native_run_cycle_authority_python_fallback", False)):
    print("rust_core.native_run_cycle_authority_python_fallback=true")
    sys.exit(12)
PY
  )"; then
    return 0
  else
    local status=$?
    if [[ $status -eq 10 ]]; then
      fail "runtime config still routes scheduler authority through Python (${bridge_line})" 5
    elif [[ $status -eq 11 ]]; then
      fail "runtime config does not enable native Rust run-cycle authority (${bridge_line})" 6
    elif [[ $status -eq 12 ]]; then
      fail "runtime config still allows Python fallback for Rust run-cycle authority (${bridge_line})" 7
    elif [[ $status -ne 0 ]]; then
      fail "unable to inspect ${CONFIG_PATH} for Python backend bridge usage" 8
    fi
  fi
}

check_repo_for_python_bridge() {
  if [[ -f app.py ]] && grep -qE 'from engine\.run_cycle import run_cycle' app.py; then
    fail "app.py still imports engine.run_cycle, so WebUI/API flows still depend on the Python run-cycle backend" 7
  fi

  if [[ -f app.py ]] && grep -qE 'run_cycle\(mode=["'"'"']dry_run["'"'"']' app.py; then
    fail "WebUI dry-run routes still execute the Python run-cycle backend" 8
  fi

  if [[ -f rust/lqosync-core/src/rust_scheduler.rs ]] && grep -q 'scripts/run_cycle_once.py' rust/lqosync-core/src/rust_scheduler.rs; then
    fail "Rust scheduler defaults still point to scripts/run_cycle_once.py; replace that command with a native Rust run-cycle path first" 9
  fi

  if [[ -f engine/config_loader.py ]] && grep -q 'scripts/run_cycle_once.py scheduled' engine/config_loader.py; then
    fail "config loader defaults still seed scheduler.rust_run_cycle_command with the Python bridge" 10
  fi

  if [[ -f engine/config_loader.py ]] && grep -q 'scripts/run_cycle_once.py manual' engine/config_loader.py; then
    fail "config loader defaults still seed scheduler.manual_run_command with the Python bridge" 11
  fi

  if [[ -f scripts/promote-rust-full-authoritative-safe.sh ]] && grep -q 'scripts/run_cycle_once.py scheduled' scripts/promote-rust-full-authoritative-safe.sh; then
    fail "promotion scripts still wire scheduled runs through the Python bridge" 12
  fi

  if [[ -f scripts/promote-rust-full-authoritative-safe.sh ]] && grep -q 'scripts/run_cycle_once.py manual' scripts/promote-rust-full-authoritative-safe.sh; then
    fail "promotion scripts still wire manual runs through the Python bridge" 13
  fi

  if [[ -f rust/lqosync-core/src/rust_run_cycle_authority.rs ]] && grep -q 'scripts/run_cycle_once.py' rust/lqosync-core/src/rust_run_cycle_authority.rs; then
    fail "Rust run-cycle authority still contains the guarded Python fallback bridge; remove that fallback before deleting backend Python code" 14
  fi

  if [[ -f engine/config_loader.py ]] && grep -q 'native_run_cycle_authority_python_fallback", True' engine/config_loader.py; then
    fail "config loader still allows Python fallback for native Rust run-cycle authority" 15
  fi

  if [[ -f engine/config_loader.py ]] && ! grep -q 'native_run_cycle_authority_enabled", True' engine/config_loader.py; then
    fail "config loader does not default native Rust run-cycle authority on" 16
  fi
}

CONFIRM="${CONFIRM_PYTHON_BACKEND_RETIREMENT_EXECUTION:-}"
if [[ "$CONFIRM" != "CONFIRM_PYTHON_BACKEND_RETIREMENT_EXECUTION" ]]; then
  fail "set CONFIRM_PYTHON_BACKEND_RETIREMENT_EXECUTION=CONFIRM_PYTHON_BACKEND_RETIREMENT_EXECUTION" 2
fi

if [[ "${PYTHON_BACKEND_ROLLBACK_PACKAGE_READY:-0}" != "1" ]]; then
  fail "PYTHON_BACKEND_ROLLBACK_PACKAGE_READY=1 is required" 3
fi

if [[ "${FULL_RUST_BACKEND_PRODUCTION_VERIFIED:-0}" != "1" ]]; then
  fail "FULL_RUST_BACKEND_PRODUCTION_VERIFIED=1 is required" 4
fi

SERVICES=("${PYTHON_BACKEND_SERVICES:-lqosync lqosync-web lqosync-python}")

check_runtime_config_for_python_bridge
check_repo_for_python_bridge

echo "Python backend retirement executor guard"
echo "Mode: $([[ $DRY_RUN -eq 1 ]] && echo dry-run || echo execute)"
echo "Services: ${SERVICES[*]}"
echo "WebUI/UX/static assets are not modified by this script."

for svc in ${SERVICES[*]}; do
  if systemctl list-unit-files "${svc}.service" >/dev/null 2>&1; then
    if [[ $DRY_RUN -eq 1 ]]; then
      echo "DRY-RUN: systemctl disable --now ${svc}.service"
    else
      systemctl disable --now "${svc}.service"
    fi
  else
    echo "Skip missing service: ${svc}.service"
  fi
done

echo "Done. Rollback script remains required and should be tested separately."
