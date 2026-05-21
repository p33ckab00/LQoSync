# v8.2.3 Sync-Plan Gate Import Hardening Hotfix

This hotfix makes the Rust sync-plan authority gate import resilient in live deployments.

## Fixed runtime symptom

```text
name 'rust_sync_plan_authority_gate' is not defined
```

The function already exists in `engine/rust_core.py`, but live systems may still hit a NameError if `engine/run_cycle.py` was running from an older import namespace or the top-level import list was not refreshed.

## Change

`engine/run_cycle.py` now keeps the top-level import and also performs a local import immediately before the authority gate call. This is intentionally defensive and safe because it does not change the authority decision logic.

## Operator verification

```bash
cd /opt/LQoSync
bash scripts/verify-rust-sync-plan-gate-import-hardening.sh
find /opt/LQoSync -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
sudo systemctl restart lqosync
```

## Collector warnings

Warnings such as `Collector parity is not proven yet` are separate from this NameError. They indicate the Rust MikroTik collector parity stage is not complete yet and Python collector transport remains selected until the collector migration is completed.
