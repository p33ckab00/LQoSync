# v8.2.2 Rust Authority Runtime Hotfix

This hotfix documents and fixes two live deployment findings.

## 1. `name 'rust_sync_plan_authority_gate' is not defined`

Symptom in WebUI warnings/errors:

```text
name 'rust_sync_plan_authority_gate' is not defined
```

Cause: `engine.run_cycle` called the Rust sync-plan authority gate but did not import the helper from `engine.rust_core`.

Fix: v8.2.2 imports `rust_sync_plan_authority_gate` in `engine/run_cycle.py`.

Immediate local workaround on an affected install is to update to v8.2.2 or add the missing import, then restart `lqosync`.

## 2. Rust shadow collector warnings

Possible warnings:

```text
Rust core: Collector parity is not proven yet; Python collectors remain authoritative.
Rust core: Rust-shadow collector dry-run bundle gates are not fully enabled; Python collectors remain selected.
Rust collector bundle shadow output differs from Python authoritative rows: score=0.00%, missing=23, extra=0, field_mismatches=0.
run_cycle Rust-shadow report gates are not fully enabled; Python run_cycle remains authoritative and no Rust-shadow bundle is selected for cycle comparison.
```

Meaning: the Rust daemon is running and may own scheduler/apply authority, but Rust collector parity is not yet proven for the MikroTik data-source path. Do not treat this as a UI/browser issue.

Operator action:

1. Do not enable unattended auto-apply until the hard error is fixed and a dry run is clean.
2. Keep `scheduler.engine=rust` and Rust apply gates enabled.
3. Treat collector-shadow warnings as a signal that the next major work item is Rust collector authority/parity, not WebUI/Django migration.
4. Use Dry Run first, compare rows, then Manual Apply.

## Canonical boundary after this hotfix

- Flask remains the WebUI shell.
- Rust daemon remains scheduler/apply/sync-plan authority.
- Collector transport may still use Python compatibility until Rust collector parity is proven.
