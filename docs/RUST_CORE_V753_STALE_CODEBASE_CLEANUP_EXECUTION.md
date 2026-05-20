# Rust Core v7.5.3 Stale Codebase Cleanup Execution Guard

## Purpose

v7.5.3 adds a guarded, archive-first cleanup execution layer for stale LQoSync working trees after the full Rust backend production series is verified.

This release does **not** delete code. It archives known duplicate or legacy working directories only after explicit operator confirmation and final production guard checks.

## Canonical locations

```text
/opt/LQoSync                 canonical LQoSync app/runtime checkout
/opt/libreqos                LibreQoS application path, never touched by cleanup
/usr/local/bin/lqosync-core  Rust core binary, never archived
/etc/systemd/system/lqosync-core.service  Rust daemon unit, never archived
```

## Safe archive candidates

The guarded executor only attempts to archive these duplicate/legacy working trees:

```text
/home/pi/lqosync_docker
/home/pi/lqosync
/opt/lqosync
```

The executor refuses protected paths such as `/opt/LQoSync`, `/opt/libreqos`, `/usr/local/bin/lqosync-core`, and `lqosync-core.service`.

## New scripts

```text
scripts/stale-codebase-cleanup-execution-plan.sh
scripts/stale-codebase-cleanup-execute-guard.sh
scripts/stale-codebase-post-cleanup-verify.sh
scripts/stale-codebase-restore-from-archive.sh
```

## Recommended workflow

Generate an execution plan:

```bash
cd /opt/LQoSync
bash scripts/stale-codebase-cleanup-execution-plan.sh
```

Review the output. Confirm that:

```text
canonical_app_exists=true
rust_binary_exists=true
rust_service_active=active
self_test_ok=true
has_steady_state_guard=true
has_production_drift_monitor=true
```

Then execute guarded archive:

```bash
export CONFIRM_STALE_CODEBASE_CLEANUP_EXECUTION=CONFIRM_STALE_CODEBASE_CLEANUP_EXECUTION
export LQOSYNC_CANONICAL_VERIFIED=1
export LQOSYNC_CORE_SELF_TEST_OK=1
sudo -E bash scripts/stale-codebase-cleanup-execute-guard.sh --execute
```

Verify after cleanup:

```bash
bash scripts/stale-codebase-post-cleanup-verify.sh
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
```

## Restore

If a stale working tree must be restored:

```bash
export CONFIRM_STALE_CODEBASE_RESTORE=CONFIRM_STALE_CODEBASE_RESTORE
sudo -E bash scripts/stale-codebase-restore-from-archive.sh \
  /opt/LQoSync-archive/<timestamp> \
  lqosync_docker \
  /home/pi/lqosync_docker
```

## Safety guarantees

```text
No service restarts
No Python file deletion
No WebUI/static asset deletion
No LibreQoS mutation
No Rust daemon mutation
No archive of /opt/LQoSync
No archive of /opt/libreqos
Archive-first, restore-supported cleanup only
```

