# Rust Core v4.6 Collector Authority Promotion Execution Rehearsal

Version mapping:

```text
LQoSync VERSION = 2.116.0-rc1
lqosync-core = 4.6.0
```

## Status

This phase is **not full Rust backend production yet**.

```text
Current phase:
Collector authority promotion execution rehearsal bridge
```

Python collectors remain authoritative. Rust can now produce a promotion execution rehearsal contract, but it still cannot promote collector authority, drive cleanup, write generated files, or apply LibreQoS.

## New Rust operation

```text
build-collector-authority-promotion-execution-rehearsal
```

## Purpose

v4.5 answered:

```text
Is the system ready to consider promotion?
```

v4.6 answers:

```text
Would a promotion execution be allowed if this were a later production-capable release?
```

The answer remains a rehearsal-only contract.

## Required confirmation

```text
CONFIRM_COLLECTOR_AUTHORITY_PROMOTION_EXECUTION_REHEARSAL
```

If the operation must build the prerequisite v4.5 readiness report internally, it can use a separate readiness confirmation field:

```json
{
  "confirmation": "CONFIRM_COLLECTOR_AUTHORITY_PROMOTION_EXECUTION_REHEARSAL",
  "collector_authority_promotion_readiness_confirmation": "CONFIRM_COLLECTOR_AUTHORITY_PROMOTION_READINESS"
}
```

This avoids confirmation-token collision between readiness and execution rehearsal.

## Safety guarantees

v4.6 still enforces:

```text
full_rust_backend = false
production_collector_authority_switched = false
collector_authority_promotion_supported = false
collector_authority_promotion_executed = false
rust_can_drive_cleanup = false
rust_can_drive_apply = false
rust_can_write_generated_files = false
safe_for_cleanup = false
write_allowed = false
apply_allowed = false
```

## Config defaults

```json
"rust_core": {
  "collector_authority_promotion_execution_rehearsal_pilot": false,
  "allow_collector_authority_promotion_execution_rehearsal": false,
  "collector_authority_promotion_execution_mode": "rehearsal_only",
  "collector_authority_promotion_execution_require_readiness": true,
  "collector_authority_promotion_execution_require_python_fallback": true,
  "collector_authority_promotion_execution_require_manual_confirmation": true,
  "collector_authority_promotion_execution_require_no_cleanup_apply": true,
  "collector_authority_promotion_execution_max_shadow_age_seconds": 900
}
```

## API endpoint

```text
GET  /api/rust-core/collector-authority-promotion-execution-rehearsal
POST /api/rust-core/collector-authority-promotion-execution-rehearsal
```

## Server validation

Run:

```bash
bash scripts/repair-script-permissions.sh
bash scripts/build-rust-core.sh
sudo bash scripts/install-rust-core.sh
sudo bash scripts/install-rust-core-daemon.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
```

Expected operation:

```text
build-collector-authority-promotion-execution-rehearsal
```

## Production note

This phase does not make LQoSync a full Rust backend. It prepares the final promotion handoff path while preserving Python collector fallback and blocking cleanup/apply/file-write authority.
