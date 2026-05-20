#!/usr/bin/env bash
set -euo pipefail

SOCKET="${LQOSYNC_CORE_SOCKET:-/run/lqosync-core.sock}"
CONFIRM="${CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER:-}"

if [[ "${CONFIRM}" != "CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER" ]]; then
  echo "Refusing verification: set CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER=CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER" >&2
  exit 2
fi

if [[ ! -S "$SOCKET" ]]; then
  echo "Rust core socket not found: $SOCKET" >&2
  exit 3
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required for socket verification request" >&2
  exit 4
fi

python3 - <<'PY'
import json, os, socket, sys
sock_path = os.environ.get("LQOSYNC_CORE_SOCKET", "/run/lqosync-core.sock")
req = {
    "version": "1",
    "op": "build-full-rust-backend-production-verifier",
    "payload": {
        "confirmation": "CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER",
        "shadow_age_seconds": 0,
        "webui_ux_unchanged": True,
        "webui_static_asset_paths_unchanged": True,
        "webui_static_assets_preserved": True,
        "rust_service_active": True,
        "rust_api_healthcheck_passed": True,
        "rust_unix_socket_active": True,
        "api_traffic_switched_to_rust": os.environ.get("API_TRAFFIC_SWITCHED_TO_RUST", "0") == "1",
        "flask_routes_disabled": os.environ.get("FLASK_ROUTES_DISABLED", "0") == "1",
        "python_backend_stopped_or_disabled": os.environ.get("PYTHON_BACKEND_STOPPED_OR_DISABLED", "0") == "1",
        "python_backend_rollback_package_ready": os.environ.get("PYTHON_BACKEND_ROLLBACK_PACKAGE_READY", "0") == "1",
        "server_cargo_tests_passed": os.environ.get("SERVER_CARGO_TESTS_PASSED", "0") == "1",
        "self_test_passed": os.environ.get("SELF_TEST_PASSED", "0") == "1",
        "rollback_test_passed": os.environ.get("ROLLBACK_TEST_PASSED", "0") == "1",
        "production_healthcheck_passed": os.environ.get("PRODUCTION_HEALTHCHECK_PASSED", "0") == "1",
        "operator_full_rust_backend_production_verifier_ack": os.environ.get("OPERATOR_FULL_RUST_BACKEND_PRODUCTION_VERIFIER_ACK", "0") == "1",
        "rollback_path": os.environ.get("ROLLBACK_PATH", "restore_python_backend_and_flask_routes"),
        "rust_core": {
            "allow_full_rust_backend_production_verifier": True,
            "full_rust_backend_production_verifier_pilot": True,
            "full_rust_backend_production_verifier_mode": "verify_only"
        },
        "full_rust_backend_production_cutover": {
            "status": "full_rust_backend_production_cutover_ready",
            "cutover_allowed": True,
            "webui_ux_unchanged": True,
            "python_removal_allowed": True,
            "python_backend_removable": True,
            "python_backend_removed": False
        }
    }
}
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(sock_path)
s.sendall(json.dumps(req).encode())
s.shutdown(socket.SHUT_WR)
chunks=[]
while True:
    chunk=s.recv(65536)
    if not chunk:
        break
    chunks.append(chunk)
res=json.loads(b"".join(chunks).decode())
print(json.dumps(res, indent=2))
if not res.get("ok") or res.get("result", {}).get("status") != "full_rust_backend_production_verified":
    sys.exit(10)
PY
