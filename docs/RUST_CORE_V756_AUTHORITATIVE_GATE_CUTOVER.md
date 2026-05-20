# v7.5.6 Rust Authoritative Gate Cutover

This package promotes Rust from passive shadow mode to **authoritative gate mode** without removing Python fallback.

## What becomes authoritative

Rust now has production authority for:

- runtime output validation blockers
- sync-plan blocker enforcement
- fail-closed non-dry-run apply gating
- daemon-first health path when available

The effective config profile is:

```json
{
  "rust_core": {
    "enabled": true,
    "prefer_daemon": true,
    "enforce_validation": true,
    "enforce_sync_plan": true,
    "fail_closed_when_enforced": true,
    "authority_mode": "enforce_blockers",
    "self_test_on_status": true,
    "require_authority_readiness": true
  }
}
```

## No-breakage boundary

This cutover intentionally does **not** enable these high-risk authority surfaces:

```json
{
  "execute_apply_manifest": false,
  "allow_rust_file_writes": false,
  "allow_rust_libreqos_apply": false,
  "execute_rollback": false,
  "allow_rust_rollback_file_writes": false,
  "collector_authority_mode": "python_authoritative"
}
```

Reason: the Rust core currently reports that direct `LibreQoS.py` invocation is not implemented by Rust, and RouterOS live collector authority still requires a separate parity/cutover path. Python remains the supervised executor/fallback while Rust is the authoritative safety gate.

## One-command safe install

```bash
sudo bash install-rust-authoritative-safe.sh
```

This wrapper:

1. runs the production-safe installer
2. builds the Rust core
3. installs the Rust core binary
4. installs the Rust daemon
5. runs Rust self-test
6. backs up `/opt/libreqos/src/config.json`
7. promotes config to Rust authoritative gate mode
8. leaves the main `lqosync` service stopped/enabled by default

Start after review:

```bash
sudo systemctl start lqosync
```

## Promote an existing install only

```bash
cd /opt/LQoSync
sudo bash scripts/promote-rust-authoritative-safe.sh
```

Restart after promotion only when ready:

```bash
sudo RESTART_SERVICE=true bash scripts/promote-rust-authoritative-safe.sh
```

## Live production recommendation

Before enabling scheduler/auto-apply, run a dry run and review:

```bash
cd /opt/LQoSync
python3 scripts/doctor.py /opt/libreqos/src/config.json
sudo systemctl start lqosync
```

Then use the UI Dry Run page and review `rust_authority_gate`, `rust_sync_plan`, and `rust_core_validation`.

## Rollback

The promotion script stores a timestamped backup under:

```text
/root/lqosync_rust_authority_backups/<timestamp>/config.json.before-rust-authority
```

Rollback config:

```bash
sudo cp /root/lqosync_rust_authority_backups/<timestamp>/config.json.before-rust-authority /opt/libreqos/src/config.json
sudo systemctl restart lqosync
```
