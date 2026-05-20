# Rust Core v4.7 Collector Authority Promotion Commit Plan

LQoSync `2.117.0-rc1` / `lqosync-core 4.7.0` adds `build-collector-authority-promotion-commit-plan`.

This is a non-mutating bridge after v4.6 promotion execution rehearsal. It prepares an auditable commit plan for a future Rust collector-authority promotion, but it still does not switch production collector authority.

## Operation

```text
build-collector-authority-promotion-commit-plan
```

## Required confirmation token

```text
CONFIRM_COLLECTOR_AUTHORITY_PROMOTION_COMMIT_PLAN
```

If the prerequisite promotion execution rehearsal must be built internally, pass its separate confirmation token as:

```json
{
  "collector_authority_promotion_execution_confirmation": "CONFIRM_COLLECTOR_AUTHORITY_PROMOTION_EXECUTION_REHEARSAL"
}
```

## Safety behavior

The commit plan is fail-safe and non-mutating:

- no live RouterOS reads
- no Rust collector promotion
- no cleanup authority transfer
- no generated file writes
- no LibreQoS apply authority
- Python collector fallback remains mandatory

The result explicitly keeps `full_rust_backend=false`, `production_collector_authority_switched=false`, `rust_can_drive_cleanup=false`, `rust_can_drive_apply=false`, and `rust_can_write_generated_files=false`.

## API endpoint

```text
GET  /api/rust-core/collector-authority-promotion-commit-plan
POST /api/rust-core/collector-authority-promotion-commit-plan
```

## Config defaults

```json
{
  "rust_core": {
    "collector_authority_promotion_commit_plan_pilot": false,
    "allow_collector_authority_promotion_commit_plan": false,
    "collector_authority_promotion_commit_mode": "plan_only",
    "collector_authority_promotion_commit_require_execution_rehearsal": true,
    "collector_authority_promotion_commit_require_python_fallback": true,
    "collector_authority_promotion_commit_require_manual_confirmation": true,
    "collector_authority_promotion_commit_require_no_cleanup_apply": true,
    "collector_authority_promotion_commit_max_shadow_age_seconds": 900
  }
}
```
