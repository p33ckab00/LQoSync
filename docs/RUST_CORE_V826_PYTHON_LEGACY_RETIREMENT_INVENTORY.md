# v8.2.6 Python Legacy Retirement Inventory

This release adds a Rust-owned, non-mutating inventory gate for the final Python backend cleanup path.

It does **not** delete Python files. It classifies them so operators can separate the Flask WebUI shell from backend legacy candidates after full Rust production evidence is already healthy.

## Rust operation

```text
build-python-legacy-retirement-inventory
```

## Endpoint

```text
GET  /api/rust-core/python-legacy-retirement-inventory
POST /api/rust-core/python-legacy-retirement-inventory
```

## Confirmation token

```text
CONFIRM_PYTHON_LEGACY_RETIREMENT_INVENTORY
```

## What It Checks

- The full Rust backend production audit sentinel is healthy.
- Python/Flask drift is already absent.
- `python_runtime_role = flask_webui_shell_only`.
- WebUI/UX and static assets are preserved.
- Rollback package, rollback test, and rollback path are ready.
- Operator acknowledgement is present.
- No delete/archive/cleanup execution was requested.

## Classifications

- `webui_shell_required`: preserve Flask routes, templates, static assets, auth, and Rust protocol bridge files.
- `legacy_backend_candidate`: archive only after guarded full-Rust cutover and rollback gates pass.
- `python_shell_or_unknown`: inspect manually before cleanup.
- `non_python_or_asset`: preserve; outside Python backend retirement.

## Safety Contract

The operation is inventory-only:

```text
non_mutating=true
side_effects_allowed=false
delete_allowed=false
archive_plan_allowed=true only when all gates pass
```

`delete_allowed=false` is intentional. Actual file movement remains outside Rust core and must use guarded operational scripts with rollback support.

## Example

```json
{
  "version": "1",
  "op": "build-python-legacy-retirement-inventory",
  "payload": {
    "confirmation": "CONFIRM_PYTHON_LEGACY_RETIREMENT_INVENTORY",
    "full_rust_backend_production_audit_sentinel": {
      "status": "full_rust_backend_production_audit_sentinel_healthy",
      "full_rust_backend": true,
      "python_backend_removed": true,
      "python_backend_retired": true,
      "side_effects_allowed": false
    },
    "webui_ux_unchanged": true,
    "webui_static_asset_paths_unchanged": true,
    "webui_static_assets_preserved": true,
    "python_backend_rollback_package_ready": true,
    "rollback_test_passed": true,
    "rollback_path": "restore_python_backend_and_flask_routes",
    "operator_python_legacy_retirement_ack": true,
    "rust_core": {
      "allow_python_legacy_retirement_inventory": true,
      "python_legacy_retirement_inventory_pilot": true,
      "python_legacy_retirement_inventory_mode": "inventory_only",
      "python_runtime_role": "flask_webui_shell_only"
    }
  }
}
```

## Verification

```bash
cargo test --manifest-path rust/lqosync-core/Cargo.toml python_legacy_retirement_inventory
bash scripts/verify-rust-stable-release-cleanup.sh
python3 scripts/regression_check.py
```
