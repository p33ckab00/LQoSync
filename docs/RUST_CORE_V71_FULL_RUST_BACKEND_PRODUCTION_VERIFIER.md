# Rust Core v7.1 Full Rust Backend Production Verifier / Python Retirement Guard

`VERSION = 2.141.0-rc1`  
`lqosync-core = 7.1.0`

## Purpose

v7.1 adds the post-cutover production verifier and Python retirement executor guard.

This phase verifies that Rust is actually serving the backend runtime while WebUI/UX/static assets remain unchanged. It can mark Python retirement as allowed only when Rust production, rollback, tests, operator confirmation, and service health gates pass.

## New Rust operation

```text
build-full-rust-backend-production-verifier
```

## Required confirmation token

```text
CONFIRM_FULL_RUST_BACKEND_PRODUCTION_VERIFIER
```

## New scripts

```text
scripts/full-rust-backend-production-verify.sh
scripts/python-backend-retirement-executor-guard.sh
```

`python-backend-retirement-executor-guard.sh` is dry-run by default. It requires:

```text
CONFIRM_PYTHON_BACKEND_RETIREMENT_EXECUTION=CONFIRM_PYTHON_BACKEND_RETIREMENT_EXECUTION
PYTHON_BACKEND_ROLLBACK_PACKAGE_READY=1
FULL_RUST_BACKEND_PRODUCTION_VERIFIED=1
```

Then use `--execute` for supervised service retirement. WebUI/UX/static assets are not modified by the script.

## New endpoint

```text
GET  /api/rust-core/full-rust-backend-production-verifier
POST /api/rust-core/full-rust-backend-production-verifier
```

## Status

This is final production verification and Python retirement guard stage. It can report:

```text
full_rust_backend = true
full_rust_backend_production_enabled = true
rust_service_runtime_authoritative = true
python_backend_removable = true
python_removal_allowed = true
```

But the Rust core still does not directly delete files or stop services. OS-level retirement is supervised by the guarded script.
