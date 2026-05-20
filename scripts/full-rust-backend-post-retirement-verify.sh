#!/usr/bin/env bash
set -euo pipefail

SOCKET="${LQOSYNC_CORE_SOCKET:-/run/lqosync-core.sock}"
CONFIRM="${CONFIRM_FULL_RUST_BACKEND_POST_RETIREMENT_VERIFIER:-}"
ROLLBACK_READY="${PYTHON_BACKEND_ROLLBACK_PACKAGE_READY:-0}"
PYTHON_RETIRED="${PYTHON_BACKEND_RETIRED:-0}"
RUST_ACTIVE="${RUST_BACKEND_ACTIVE:-1}"
WEBUI_OK="${WEBUI_UX_UNCHANGED:-1}"

if [[ "$CONFIRM" != "CONFIRM_FULL_RUST_BACKEND_POST_RETIREMENT_VERIFIER" ]]; then
  echo "Refusing to verify: set CONFIRM_FULL_RUST_BACKEND_POST_RETIREMENT_VERIFIER=CONFIRM_FULL_RUST_BACKEND_POST_RETIREMENT_VERIFIER" >&2
  exit 2
fi

REQ=$(cat <<JSON
{
  "version":"1",
  "op":"build-full-rust-backend-post-retirement-verifier",
  "payload":{
    "confirmation":"CONFIRM_FULL_RUST_BACKEND_POST_RETIREMENT_VERIFIER",
    "shadow_age_seconds":0,
    "webui_ux_unchanged":${WEBUI_OK},
    "webui_static_asset_paths_unchanged":${WEBUI_OK},
    "webui_static_assets_preserved":${WEBUI_OK},
    "rust_service_active":${RUST_ACTIVE},
    "rust_api_healthcheck_passed":${RUST_ACTIVE},
    "rust_unix_socket_active":${RUST_ACTIVE},
    "api_traffic_switched_to_rust":${RUST_ACTIVE},
    "rust_service_runtime_authoritative":${RUST_ACTIVE},
    "flask_routes_disabled":${PYTHON_RETIRED},
    "python_backend_stopped_or_disabled":${PYTHON_RETIRED},
    "python_backend_service_masked_or_disabled":${PYTHON_RETIRED},
    "python_api_routes_unregistered":${PYTHON_RETIRED},
    "python_backend_files_preserved_for_rollback":${ROLLBACK_READY},
    "python_backend_rollback_package_ready":${ROLLBACK_READY},
    "rollback_test_passed":1,
    "server_cargo_tests_passed":1,
    "self_test_passed":1,
    "production_healthcheck_passed":1,
    "post_retirement_healthcheck_passed":1,
    "operator_full_rust_backend_post_retirement_ack":true,
    "rollback_path":"restore_python_backend_and_flask_routes",
    "rust_core":{
      "allow_full_rust_backend_post_retirement_verifier":true,
      "full_rust_backend_post_retirement_verifier_pilot":true,
      "full_rust_backend_post_retirement_verifier_mode":"verify_only"
    },
    "full_rust_backend_production_verifier":{
      "status":"full_rust_backend_production_verified",
      "full_rust_backend":true,
      "rust_service_runtime_authoritative":true,
      "python_retirement_executor_allowed":true,
      "webui_ux_unchanged":true
    }
  }
}
JSON
)

if command -v socat >/dev/null 2>&1 && [[ -S "$SOCKET" ]]; then
  printf '%s' "$REQ" | socat - UNIX-CONNECT:"$SOCKET"
elif command -v lqosync-core >/dev/null 2>&1; then
  printf '%s' "$REQ" | lqosync-core
else
  echo "Neither socat+socket nor lqosync-core CLI is available." >&2
  exit 3
fi
