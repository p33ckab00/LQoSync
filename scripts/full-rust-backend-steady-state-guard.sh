#!/usr/bin/env bash
set -euo pipefail

SOCKET_PATH="${LQOSYNC_CORE_SOCKET:-/run/lqosync-core.sock}"
CONFIRM="${CONFIRM_FULL_RUST_BACKEND_STEADY_STATE_GUARD:-}"

python3 - <<'PY'
import json, os, socket, sys
sock_path = os.environ.get("LQOSYNC_CORE_SOCKET", "/run/lqosync-core.sock")
req = {
    "version": "1",
    "op": "build-full-rust-backend-steady-state-guard",
    "payload": {
        "confirmation": os.environ.get("CONFIRM_FULL_RUST_BACKEND_STEADY_STATE_GUARD", ""),
        "shadow_age_seconds": int(os.environ.get("STEADY_STATE_SHADOW_AGE_SECONDS", "0")),
        "rust_service_active": os.environ.get("RUST_BACKEND_ACTIVE") == "1",
        "rust_api_healthcheck_passed": os.environ.get("RUST_API_HEALTHCHECK_PASSED") == "1",
        "rust_unix_socket_active": os.environ.get("RUST_UNIX_SOCKET_ACTIVE", "1") == "1",
        "api_traffic_switched_to_rust": os.environ.get("API_TRAFFIC_SWITCHED_TO_RUST") == "1",
        "rust_service_runtime_authoritative": os.environ.get("RUST_SERVICE_RUNTIME_AUTHORITATIVE") == "1",
        "flask_routes_disabled": os.environ.get("FLASK_ROUTES_DISABLED") == "1",
        "python_backend_stopped_or_disabled": os.environ.get("PYTHON_BACKEND_RETIRED") == "1",
        "python_backend_service_masked_or_disabled": os.environ.get("PYTHON_BACKEND_SERVICE_MASKED_OR_DISABLED", "1") == "1",
        "python_backend_unexpectedly_running": os.environ.get("PYTHON_BACKEND_UNEXPECTEDLY_RUNNING") == "1",
        "flask_routes_reappeared": os.environ.get("FLASK_ROUTES_REAPPEARED") == "1",
        "api_traffic_routed_to_python": os.environ.get("API_TRAFFIC_ROUTED_TO_PYTHON") == "1",
        "webui_ux_unchanged": os.environ.get("WEBUI_UX_UNCHANGED") == "1",
        "webui_static_asset_paths_unchanged": os.environ.get("WEBUI_STATIC_ASSET_PATHS_UNCHANGED", "1") == "1",
        "webui_static_assets_preserved": os.environ.get("WEBUI_STATIC_ASSETS_PRESERVED", "1") == "1",
        "python_backend_rollback_package_ready": os.environ.get("PYTHON_BACKEND_ROLLBACK_PACKAGE_READY") == "1",
        "rollback_path": os.environ.get("PYTHON_BACKEND_ROLLBACK_PATH", "restore_python_backend_and_flask_routes"),
        "rollback_test_passed": os.environ.get("ROLLBACK_TEST_PASSED") == "1",
        "server_cargo_tests_passed": os.environ.get("SERVER_CARGO_TESTS_PASSED") == "1",
        "self_test_passed": os.environ.get("SELF_TEST_PASSED") == "1",
        "production_healthcheck_passed": os.environ.get("PRODUCTION_HEALTHCHECK_PASSED") == "1",
        "post_retirement_healthcheck_passed": os.environ.get("POST_RETIREMENT_HEALTHCHECK_PASSED") == "1",
        "steady_state_healthcheck_passed": os.environ.get("STEADY_STATE_HEALTHCHECK_PASSED") == "1",
        "operator_full_rust_backend_steady_state_ack": os.environ.get("OPERATOR_FULL_RUST_BACKEND_STEADY_STATE_ACK") == "1",
        "full_rust_backend_post_retirement_verifier": {
            "status": "full_rust_backend_post_retirement_verified",
            "full_rust_backend": True,
            "python_backend_removed": os.environ.get("PYTHON_BACKEND_RETIRED") == "1",
            "webui_ux_unchanged": os.environ.get("WEBUI_UX_UNCHANGED") == "1",
        },
        "rust_core": {
            "full_rust_backend_steady_state_guard_pilot": True,
            "allow_full_rust_backend_steady_state_guard": True,
            "full_rust_backend_steady_state_guard_mode": "guard_only",
        }
    }
}
with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
    s.connect(sock_path)
    s.sendall(json.dumps(req).encode())
    s.shutdown(socket.SHUT_WR)
    data = b""
    while True:
        chunk = s.recv(65536)
        if not chunk:
            break
        data += chunk
print(data.decode())
PY
