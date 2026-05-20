# Rust Core v7.5.7 Full Rust Apply Authority

This release moves the apply execution path from Rust authoritative gate mode to Rust apply authority mode.

## What Rust owns in this mode

When promoted with `scripts/promote-rust-full-authoritative-safe.sh` or installed with `install-rust-full-authoritative-safe.sh`, Rust owns:

- validation enforcement
- sync-plan authority gates
- atomic writes for `ShapedDevices.csv`
- atomic writes for `network.json`
- transaction preview/result reporting
- optional transaction journal append path
- external `LibreQoS.py --updateonly` execution
- apply-run stdout/stderr/meta logs under `paths.libreqos_apply_log_dir`

## What Python still owns

Python remains the compatibility shell for:

- Flask WebUI/API routes
- scheduler entrypoint and service process
- RouterOS collector compatibility path
- emergency fallback when Rust authority flags are not enabled

The production data mutation path is Rust-owned once these flags are enabled:

```json
{
  "execute_apply_manifest": true,
  "allow_rust_file_writes": true,
  "allow_rust_libreqos_apply": true,
  "append_transaction_journal": true,
  "allow_transaction_journal_writes": true
}
```

## Safe install

```bash
sudo bash install-rust-full-authoritative-safe.sh
```

This keeps the main `lqosync` service enabled but not restarted by default. Run Dry Run and review the diff before starting/restarting the service.

## Existing install promotion

```bash
cd /opt/LQoSync
sudo bash scripts/promote-rust-full-authoritative-safe.sh
```

The promotion script requires `lqosync-core self-test` to pass and backs up `/opt/libreqos/src/config.json` before changing authority flags.

## No-breakage behavior

- Dry-run never writes files or runs `LibreQoS.py`.
- `file_drift_policy=block` is enforced before Rust writes.
- If Rust authoritative apply fails, the cycle fails closed instead of silently falling back to Python mutation.
- If Rust authority flags are disabled, Python keeps the old write/apply path.
- `backup_before_apply=true` remains enabled.

### v7.5.8 full authority lock note

When using `install-rust-full-authoritative-safe.sh` or `scripts/promote-rust-full-authoritative-safe.sh`, full Rust authority mode now sets `python_mutation_fallback=false`. Python remains the WebUI/scheduler shell, but production file writes and LibreQoS apply must be completed by Rust or the cycle fails closed.

