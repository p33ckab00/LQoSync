# Rust Core v1.6 Authority Readiness Report

Rust Core v1.6 adds a read-only readiness evaluator for deciding whether Rust authority flags are safe to pilot.

## Operation

```text
evaluate-authority-readiness
```

The operation checks the current `rust_core` flags, Rust transport health, self-test result, generated file paths, transaction journal status, and rollback authority settings.

## Why it exists

By v1.5, the Rust core can rehearse and optionally execute file writes and rollback restores. Those capabilities are intentionally gated. v1.6 adds an explicit readiness report so operators can see whether the system is still in safe shadow mode, ready for sync-plan enforcement, or misconfigured with partial authority flags.

## API

```text
GET /api/rust-core/authority-readiness
```

The API is read-only and does not write, apply, or restore anything.

## Verdicts

| Verdict | Meaning |
|---|---|
| `shadow_safe` | Rust core is healthy and no authority flags are enabled. |
| `ready_for_sync_plan_enforcement` | Rust sync-plan blocker enforcement can be piloted. |
| `ready_for_authority_pilot` | File-write or rollback authority prerequisites are satisfied, but should only be enabled during a maintenance window. |
| `ready_with_warnings` | No hard blockers, but caution items exist. |
| `not_ready` | One or more blockers must be fixed before enabling authority flags. |

## Safety rules

- Read-only operation.
- Python remains authoritative by default.
- Partial authority flags are treated as blockers.
- Rust LibreQoS external apply remains delegated to Python in this release.
- File-write and rollback authority should be paired with transaction journal persistence.

## Recommended workflow

```text
1. Build and install Rust core.
2. Run self-test.
3. Review /api/rust-core/authority-readiness.
4. Enable only one authority class at a time.
5. Run Dry Run and inspect Rust Sync Plan, Apply Manifest, Transaction Journal, and Rollback sections.
6. Keep Python fallback available.
```
