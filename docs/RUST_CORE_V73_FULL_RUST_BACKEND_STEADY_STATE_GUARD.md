# Rust Core v7.3 Full Rust Backend Steady-State Guard

`VERSION = 2.143.0-rc1`  
`rust/lqosync-core = 7.3.0`

## Purpose

v7.3 adds the steady-state production guard after v7.2 post-retirement verification.

This phase is for ongoing full-Rust production verification after Python backend retirement has already been guarded and verified.

## New operation

```text
build-full-rust-backend-steady-state-guard
```

## Required confirmation token

```text
CONFIRM_FULL_RUST_BACKEND_STEADY_STATE_GUARD
```

## What it verifies

```text
post-retirement verifier passed
Rust service/runtime remains authoritative
API traffic remains on Rust
Python/Flask does not drift back into service
WebUI/UX/static assets remain unchanged
rollback package and rollback test remain available
server cargo/self/production/post-retirement/steady-state checks pass
operator acknowledgement present
```

## Endpoint

```text
GET  /api/rust-core/full-rust-backend-steady-state-guard
POST /api/rust-core/full-rust-backend-steady-state-guard
```

## Script

```text
scripts/full-rust-backend-steady-state-guard.sh
```

Example:

```bash
export CONFIRM_FULL_RUST_BACKEND_STEADY_STATE_GUARD=CONFIRM_FULL_RUST_BACKEND_STEADY_STATE_GUARD
export RUST_BACKEND_ACTIVE=1
export RUST_API_HEALTHCHECK_PASSED=1
export API_TRAFFIC_SWITCHED_TO_RUST=1
export RUST_SERVICE_RUNTIME_AUTHORITATIVE=1
export FLASK_ROUTES_DISABLED=1
export PYTHON_BACKEND_RETIRED=1
export PYTHON_BACKEND_ROLLBACK_PACKAGE_READY=1
export ROLLBACK_TEST_PASSED=1
export SERVER_CARGO_TESTS_PASSED=1
export SELF_TEST_PASSED=1
export PRODUCTION_HEALTHCHECK_PASSED=1
export POST_RETIREMENT_HEALTHCHECK_PASSED=1
export STEADY_STATE_HEALTHCHECK_PASSED=1
export WEBUI_UX_UNCHANGED=1
export OPERATOR_FULL_RUST_BACKEND_STEADY_STATE_ACK=1
sudo -E scripts/full-rust-backend-steady-state-guard.sh
```

## Safety behavior

This operation is verification-only:

```text
No service mutation
No Python file deletion
No Flask route mutation
No API traffic switching
No config/state writes
No LibreQoS apply
WebUI/UX remains unchanged
rollback package remains required
```

## Successful state

When all gates pass, the result can report:

```text
full_rust_backend_steady_state_verified
full_rust_backend = true
full_rust_backend_production_enabled = true
rust_service_runtime_authoritative = true
python_backend_removed = true
python_backend_retired = true
python_drift_absent = true
webui_ux_unchanged = true
```
