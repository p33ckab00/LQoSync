#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
fail=0
check() { if ! grep -q "$2" "$1"; then echo "MISSING: $1 lacks $2" >&2; fail=1; else echo "ok|$1|$2"; fi; }
[ -x lqosyncctl.sh ] || { echo "MISSING executable: lqosyncctl.sh" >&2; fail=1; }
check lqosyncctl.sh "install|update)"
check lqosyncctl.sh "update)"
check lqosyncctl.sh "uninstall)"
check lqosyncctl.sh "adopt)"
check lqosyncctl.sh "check)"
check lqosyncctl.sh "verify)"
check lqosyncctl.sh "safe.directory"
check lqosyncctl.sh "rustup.rs"
check lqosyncctl.sh "ensure_acl_available"
check lqosyncctl.sh "backup_permission_snapshot"
check lqosyncctl.sh "adopt-runtime-permissions.sh"
check scripts/snapshot-runtime-permissions.sh "Original permission snapshot saved"
check scripts/adopt-runtime-permissions.sh "snapshot-runtime-permissions.sh"
check scripts/restore_libreqos_permissions.sh "Restoring original permissions from snapshot"
check app.py "_dry_run_failure_result"
check app.py "Dry-run route failed"
check app.py "api_sync_dry_run"
check app.py "rust_execute_apply_transaction"
check templates/dry_run.html "Dry Run failed safely"
if [ "$fail" -ne 0 ]; then exit 1; fi
echo "PASS: one-line operations and dry-run hardening verified"
