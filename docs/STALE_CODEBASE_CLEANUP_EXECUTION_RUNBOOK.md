# Stale Codebase Cleanup Execution Runbook

Use this runbook after the Rust backend production guard and drift monitor are already passing.

## 1. Confirm production state

```bash
systemctl status lqosync-core.service --no-pager
printf '{"version":"1","op":"self-test","payload":{}}' | lqosync-core
```

Expected:

```text
ok: true
build-full-rust-backend-steady-state-guard present
build-full-rust-backend-production-drift-monitor present
```

## 2. Generate cleanup plan

```bash
cd /opt/LQoSync
bash scripts/stale-codebase-cleanup-execution-plan.sh
```

Review archive candidates. Do not archive `/opt/lqosync-website` unless you intentionally retire the separate website service.

## 3. Execute guarded archive

```bash
export CONFIRM_STALE_CODEBASE_CLEANUP_EXECUTION=CONFIRM_STALE_CODEBASE_CLEANUP_EXECUTION
export LQOSYNC_CANONICAL_VERIFIED=1
export LQOSYNC_CORE_SELF_TEST_OK=1
sudo -E bash scripts/stale-codebase-cleanup-execute-guard.sh --execute
```

The archive is written to:

```text
/opt/LQoSync-archive/<timestamp>/
```

## 4. Verify

```bash
bash scripts/stale-codebase-post-cleanup-verify.sh
```

## 5. Restore, if needed

```bash
export CONFIRM_STALE_CODEBASE_RESTORE=CONFIRM_STALE_CODEBASE_RESTORE
sudo -E bash scripts/stale-codebase-restore-from-archive.sh /opt/LQoSync-archive/<timestamp> <item> <destination>
```

