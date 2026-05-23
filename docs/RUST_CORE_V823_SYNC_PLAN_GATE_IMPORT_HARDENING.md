# v8.2.3 Sync-Plan Gate Import Hardening Hotfix

This hotfix makes the Rust sync-plan authority gate import resilient in live deployments.

Historical note: as of v8.2.7, the affected Python run-cycle entrypoint has
been retired from the active backend package. Keep this note for operators
upgrading from older v8.2.3 installs.

## Fixed runtime symptom

```text
name 'rust_sync_plan_authority_gate' is not defined
```

The function already exists in `engine/rust_core.py`, but older live systems may still hit a NameError if `engine/run_cycle.py` was running from an older import namespace or the top-level import list was not refreshed.

## Change

In v8.2.3, `engine/run_cycle.py` kept the top-level import and also performed a local import immediately before the authority gate call. Current v8.2.7 installs use Rust run-cycle authority instead.

## Operator verification

```bash
cd /opt/LQoSync
bash scripts/verify-rust-sync-plan-gate-import-hardening.sh
find /opt/LQoSync -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
sudo systemctl restart lqosync
```

## Collector warnings

Warnings such as `Collector parity is not proven yet` are separate from this NameError. They indicate the Rust MikroTik collector parity stage is not complete yet and Python collector transport remains selected until the collector migration is completed.
