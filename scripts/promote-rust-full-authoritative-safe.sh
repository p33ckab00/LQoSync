#!/usr/bin/env bash
set -euo pipefail

# Promote an existing LQoSync install to Rust apply authority mode.
# Rust owns validation, sync-plan gating, atomic file writes, transaction journal,
# and LibreQoS.py external apply execution. Python remains the UI/scheduler and
# emergency compatibility shell; it no longer performs writes/apply when the Rust
# authority flags are enabled and healthy.

CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"
INSTALL_DIR="${LQOSYNC_INSTALL_DIR:-/opt/LQoSync}"
CORE_BIN="${LQOSYNC_CORE_BIN:-$(command -v lqosync-core 2>/dev/null || true)}"
SERVICE_NAME="${LQOSYNC_SERVICE_NAME:-lqosync}"
RESTART_SERVICE="${RESTART_SERVICE:-false}"
RUN_RUST_AUTHORITY_PREFLIGHT="${RUN_RUST_AUTHORITY_PREFLIGHT:-true}"
RUN_RUST_AUTHORITY_RECOVERY_BUNDLE="${RUN_RUST_AUTHORITY_RECOVERY_BUNDLE:-true}"
RUN_RUST_AUTHORITY_WATCHDOG="${RUN_RUST_AUTHORITY_WATCHDOG:-true}"
TS="$(date +%Y%m%d_%H%M%S)"
BACKUP_DIR="${LQOSYNC_RUST_FULL_AUTH_BACKUP_DIR:-/root/lqosync_rust_full_authority_backups/$TS}"

log() { echo "[LQoSync Rust Full Authority] $*"; }
fail() { echo "[LQoSync Rust Full Authority] ERROR: $*" >&2; exit 1; }
as_bool() {
  case "$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|y|on) return 0 ;;
    *) return 1 ;;
  esac
}

if [ "${EUID:-$(id -u)}" -ne 0 ]; then
  fail "Run as root: sudo bash scripts/promote-rust-full-authoritative-safe.sh"
fi
[ -f "$CONFIG_PATH" ] || fail "config not found: $CONFIG_PATH"
[ -n "$CORE_BIN" ] || fail "lqosync-core not found. Build/install Rust core first."
[ -x "$CORE_BIN" ] || fail "lqosync-core is not executable: $CORE_BIN"

log "Running Rust self-test before changing config..."
SELF_TEST_OUT="$($CORE_BIN <<'LQOSYNC_SELF_TEST_JSON'
{"version":"1","op":"self-test","payload":{}}
LQOSYNC_SELF_TEST_JSON
)" || fail "lqosync-core self-test command failed"
SELF_TEST_JSON="$SELF_TEST_OUT" python3 - <<'LQOSYNC_SELF_TEST_PY'
import json, os, sys
try:
    data=json.loads(os.environ.get('SELF_TEST_JSON',''))
except Exception as exc:
    print(f"Invalid self-test JSON: {exc}", file=sys.stderr)
    raise SystemExit(1)
ok = bool(data.get('ok')) and (data.get('result') or {}).get('status') == 'ok'
ops = set((data.get('result') or {}).get('operations') or [])
required = {'execute-apply-transaction', 'build-apply-manifest', 'evaluate-sync-plan', 'append-transaction-journal'}
missing = sorted(required - ops)
if not ok or missing:
    print(json.dumps(data, indent=2)[:4000], file=sys.stderr)
    if missing:
        print('missing operations: ' + ', '.join(missing), file=sys.stderr)
    raise SystemExit(1)
print('self-test ok')
LQOSYNC_SELF_TEST_PY

mkdir -p "$BACKUP_DIR"
cp -a "$CONFIG_PATH" "$BACKUP_DIR/config.json.before-rust-full-authority"
log "Backed up config: $BACKUP_DIR/config.json.before-rust-full-authority"

if as_bool "$RUN_RUST_AUTHORITY_RECOVERY_BUNDLE" && [ -x "$INSTALL_DIR/scripts/rust-full-authority-recovery-bundle.sh" ]; then
  log "Creating Rust authority recovery bundle before promotion..."
  CONFIG_PATH="$CONFIG_PATH" LQOSYNC_INSTALL_DIR="$INSTALL_DIR" "$INSTALL_DIR/scripts/rust-full-authority-recovery-bundle.sh" || fail "recovery bundle creation failed"
fi

python3 - "$CONFIG_PATH" <<'LQOSYNC_PROMOTE_PY'
from __future__ import annotations
import json, sys, pathlib, tempfile, os
path = pathlib.Path(sys.argv[1])
cfg = json.loads(path.read_text())
app = cfg.setdefault('app', {})
app['backup_before_apply'] = True
app['file_drift_policy'] = 'block'
rc = cfg.setdefault('rust_core', {})
rc.update({
    'enabled': True,
    'prefer_daemon': True,
    'enforce_validation': True,
    'enforce_sync_plan': True,
    'fail_closed_when_enforced': True,
    'authority_mode': 'enforce_blockers',
    'self_test_on_status': True,
    'require_authority_readiness': True,
    'full_rust_backend_authority': True,
    'python_mutation_fallback': False,
    'full_rust_authority_supervisor_enabled': True,
    'require_rust_authority_preflight': True,
    'fail_closed_on_authority_preflight_failure': True,
    'rust_authority_preflight_stamp': '/opt/LQoSync/state/rust_authority_preflight.json',
    'rust_authority_preflight_max_age_seconds': 900,
    'require_rust_authority_recovery_bundle': True,
    'rust_authority_recovery_bundle_dir': '/opt/LQoSync/state/rust_authority_recovery',
    'rust_authority_recovery_bundle_before_promotion': True,
    'rust_authority_supervisor_mode': 'operator_supervised',
    'rust_authority_watchdog_enabled': True,
    'fail_closed_on_authority_watchdog_failure': True,
    'rust_authority_watchdog_require_fresh_preflight': True,
    'rust_authority_watchdog_max_preflight_age_seconds': 900,
    'rust_authority_watchdog_require_recovery_bundle': True,
    'rust_authority_watchdog_require_transaction_journal_path': True,
    'fail_closed_without_rust_authority': True,
    'require_rust_authoritative_transaction': True,
    'transaction_authority': 'rust_full_authoritative',
    'execute_apply_manifest': True,
    'allow_rust_file_writes': True,
    'allow_rust_libreqos_apply': True,
    'append_transaction_journal': True,
    'allow_transaction_journal_writes': True,
    'include_rehearsal_journal_entries': False,
    'allow_dry_run_journal_entries': False,
    'execute_rollback': False,
    'allow_rust_rollback_file_writes': False,
    'rollback_authority': 'preview',
    'collector_authority_mode': 'rust_validated_python_transport',
    'collector_output_authority': 'rust_validate_all',
    'require_collector_rust_validation': True,
    'collector_authority_require_python_fallback': True,
    'run_cycle_rust_shadow_report_enabled': True,
})
text = json.dumps(cfg, indent=2) + '\n'
fd, tmp = tempfile.mkstemp(prefix=path.name + '.', dir=str(path.parent))
with os.fdopen(fd, 'w') as f:
    f.write(text)
os.replace(tmp, path)
print('config patched for Rust full apply authority mode')
LQOSYNC_PROMOTE_PY

if [ -d "$INSTALL_DIR" ]; then
  log "Validating config after patch..."
  (cd "$INSTALL_DIR" && CONFIG_PATH="$CONFIG_PATH" python3 scripts/validate_config_example.py >/dev/null)
  (cd "$INSTALL_DIR" && CONFIG_PATH="$CONFIG_PATH" python3 scripts/doctor.py "$CONFIG_PATH" || true)
fi

if as_bool "$RUN_RUST_AUTHORITY_PREFLIGHT"; then
  [ -x "$INSTALL_DIR/scripts/rust-full-authority-preflight.sh" ] || fail "preflight script missing or not executable: $INSTALL_DIR/scripts/rust-full-authority-preflight.sh"
  log "Running Rust authority preflight and writing supervisor stamp..."
  if ! CONFIG_PATH="$CONFIG_PATH" LQOSYNC_INSTALL_DIR="$INSTALL_DIR" LQOSYNC_CORE_BIN="$CORE_BIN" "$INSTALL_DIR/scripts/rust-full-authority-preflight.sh" --write-stamp; then
    cp -a "$BACKUP_DIR/config.json.before-rust-full-authority" "$CONFIG_PATH"
    fail "Rust authority preflight failed after patch; restored previous config from backup"
  fi
fi

if as_bool "$RUN_RUST_AUTHORITY_WATCHDOG"; then
  [ -x "$INSTALL_DIR/scripts/rust-authority-watchdog.sh" ] || fail "watchdog script missing or not executable: $INSTALL_DIR/scripts/rust-authority-watchdog.sh"
  log "Running Rust authority watchdog verification..."
  CONFIG_PATH="$CONFIG_PATH" LQOSYNC_INSTALL_DIR="$INSTALL_DIR" "$INSTALL_DIR/scripts/rust-authority-watchdog.sh" || fail "Rust authority watchdog verification failed after promotion"
fi

log "Rust full apply authority mode is enabled."
log "Active boundaries:"
log "  - Rust owns validation and sync-plan blocker enforcement."
log "  - Rust owns atomic ShapedDevices.csv/network.json writes."
log "  - Rust owns LibreQoS.py external apply execution."
log "  - Python still owns WebUI and scheduler shell. RouterOS transport remains Python-compatible, but collector output must pass Rust validation before mutation."
log "  - backup_before_apply=true and file_drift_policy=block."

if as_bool "$RESTART_SERVICE"; then
  log "Restarting $SERVICE_NAME as requested..."
  systemctl restart "$SERVICE_NAME"
else
  log "Service not restarted. Run Dry Run first, then restart manually: sudo systemctl restart $SERVICE_NAME"
fi
