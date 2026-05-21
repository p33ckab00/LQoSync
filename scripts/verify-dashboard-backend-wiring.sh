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
  if ! grep -q "$pattern" "$path"; then
    echo "MISSING[$label]: $path lacks $pattern" >&2
    fail=1
  else
    echo "ok|$label|$path"
  fi
}

check_contains engine/dashboard_modules.py "build_dashboard_module_wiring" "backend-helper"
check_contains app.py "build_dashboard_module_wiring" "app-import"
check_contains app.py "api_dashboard_modules" "api-route"
check_contains templates/dashboard.html "dashboard_backend_wiring_fragment.html" "dashboard-include"
check_contains templates/dashboard_backend_wiring_fragment.html "dashboard_modules" "template-context"
check_contains docs/RUST_CORE_V824_DASHBOARD_BACKEND_WIRING_AUDIT.md "Dashboard Backend Wiring" "docs"

python3 -m py_compile engine/dashboard_modules.py app.py
python3 - <<'PY'
from engine.dashboard_modules import build_dashboard_module_wiring
cfg = {
    "scheduler": {"engine": "rust", "allow_python_scheduler": False},
    "rust_core": {"enabled": True, "full_rust_backend_authority": True, "python_mutation_fallback": False, "python_runtime_role": "flask_webui_shell_only"},
    "paths": {"shaped_devices_csv": "/tmp/missing.csv", "network_json": "/tmp/missing.json"},
}
state = {"last_run": {"diff": {"policy_decision": {"verdict": "allowed"}, "client_changes": []}}}
res = build_dashboard_module_wiring(cfg, state, services={"lqosync-core": {"active": "active"}, "lqosync": {"active": "active"}}, git_status={"branch": "lqosync-in-rust"}, production_readiness={"checks": [], "score": 0}, health_report={"source_health": [], "performance_trends": {}, "libreqos_apply_health": {}}, setup_wizard={"progress": 0})
assert res["schema"] == "lqosync.dashboard_module_wiring.v1"
assert any(m["id"] == "rust_scheduler_authority" for m in res["modules"])
assert any(m["id"] == "rust_backend_authority" for m in res["modules"])
print("ok|runtime|dashboard module wiring helper")
PY

if [ "$fail" -ne 0 ]; then
  echo "FAIL: dashboard backend wiring verification failed" >&2
  exit 1
fi

echo "PASS: dashboard backend wiring verified"
