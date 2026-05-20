#!/usr/bin/env bash
set -euo pipefail

DRY_RUN=1
if [[ "${1:-}" == "--execute" ]]; then
  DRY_RUN=0
fi

CONFIRM="${CONFIRM_PYTHON_BACKEND_RETIREMENT_EXECUTION:-}"
if [[ "$CONFIRM" != "CONFIRM_PYTHON_BACKEND_RETIREMENT_EXECUTION" ]]; then
  echo "Refusing Python backend retirement: set CONFIRM_PYTHON_BACKEND_RETIREMENT_EXECUTION=CONFIRM_PYTHON_BACKEND_RETIREMENT_EXECUTION" >&2
  exit 2
fi

if [[ "${PYTHON_BACKEND_ROLLBACK_PACKAGE_READY:-0}" != "1" ]]; then
  echo "Refusing Python backend retirement: PYTHON_BACKEND_ROLLBACK_PACKAGE_READY=1 is required" >&2
  exit 3
fi

if [[ "${FULL_RUST_BACKEND_PRODUCTION_VERIFIED:-0}" != "1" ]]; then
  echo "Refusing Python backend retirement: FULL_RUST_BACKEND_PRODUCTION_VERIFIED=1 is required" >&2
  exit 4
fi

SERVICES=("${PYTHON_BACKEND_SERVICES:-lqosync lqosync-web lqosync-python}")

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
