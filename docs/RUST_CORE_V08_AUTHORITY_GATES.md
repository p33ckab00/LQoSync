# Rust Core v0.8 Authority Gates

Rust Core v0.8 introduces an **opt-in authority gate** for the shadow sync plan.

Python remains authoritative by default. The Rust sync plan still runs as a diagnostic unless the operator explicitly enables enforcement.

## Default behavior

```json
"rust_core": {
  "enforce_sync_plan": false,
  "authority_mode": "shadow"
}
```

In this mode:

- Rust validates, diffs, normalizes, and evaluates the sync plan.
- Dry Run shows Rust verdicts, blockers, and next actions.
- Apply behavior remains controlled by the existing Python policy engine.
- Rust blockers are warnings unless `enforce_sync_plan` is enabled.

## Enforced behavior

To enable opt-in enforcement:

```json
"rust_core": {
  "enabled": true,
  "prefer_daemon": true,
  "enforce_sync_plan": true,
  "fail_closed_when_enforced": true,
  "authority_mode": "enforce_blockers",
  "unix_socket": "/run/lqosync-core.sock"
}
```

When enabled, non-dry-run cycles are blocked before file writes if the Rust sync plan reports:

- `blocked_by_shadow_plan`
- Rust validation blockers
- preflight blockers carried into the sync plan
- circuit shadow blockers
- policy shadow blockers

If `fail_closed_when_enforced=true` and the Rust core is unavailable, the apply cycle blocks rather than silently falling back.

## Dry Run behavior

Dry Run never writes files. When enforcement is enabled, Dry Run shows what the gate would do, but it does not return a production failure.

## Rollback

Disable the authority gate and restart the app/service:

```json
"rust_core": {
  "enforce_sync_plan": false,
  "authority_mode": "shadow"
}
```

The Rust daemon may remain installed. Python fallback and subprocess fallback remain available.

## Operator workflow

Recommended rollout:

1. Keep `enforce_sync_plan=false`.
2. Run several Dry Runs.
3. Compare Python policy decision and Rust sync plan.
4. Enable `prefer_daemon=true`.
5. Enable `enforce_sync_plan=true` only after Rust/Python parity is trusted.

## Safety note

This is still not a full Rust run cycle. It is a gate around the existing Python run cycle. Python still collects, builds, backs up, writes, and applies. Rust decides whether an unsafe non-dry-run cycle should be blocked when enforcement is explicitly enabled.
