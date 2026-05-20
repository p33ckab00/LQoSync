#!/usr/bin/env bash
set -euo pipefail

# Safely promote an existing LQoSync config to Rust authoritative gate mode.
# This does not remove Python fallback and does not enable Rust file writes or
# Rust LibreQoS.py execution. Rust becomes authoritative for validation and
# sync-plan blocker enforcement, fail-closed, after lqosync-core self-test passes.

CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"
INSTALL_DIR="${LQOSYNC_INSTALL_DIR:-/opt/LQoSync}"
CORE_BIN="${LQOSYNC_CORE_BIN:-$(command -v lqosync-core 2>/dev/null || true)}"
SERVICE_NAME="${LQOSYNC_SERVICE_NAME:-lqosync}"
RESTART_SERVICE="${RESTART_SERVICE:-false}"
TS="$(date +%Y%m%d_%H%M%S)"
BACKUP_DIR="${LQOSYNC_RUST_AUTH_BACKUP_DIR:-/root/lqosync_rust_authority_backups/$TS}"

log() { echo "[LQoSync Rust Authority] $*"; }
fail() { echo "[LQoSync Rust Authority] ERROR: $*" >&2; exit 1; }
as_bool() {
  case "$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|y|on) return 0 ;;
    *) return 1 ;;
  esac
}

if [ "${EUID:-$(id -u)}" -ne 0 ]; then
  fail "Run as root: sudo bash scripts/promote-rust-authoritative-safe.sh"
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
if not ok:
    print(json.dumps(data, indent=2)[:4000], file=sys.stderr)
    raise SystemExit(1)
print('self-test ok')
LQOSYNC_SELF_TEST_PY

mkdir -p "$BACKUP_DIR"
cp -a "$CONFIG_PATH" "$BACKUP_DIR/config.json.before-rust-authority"
log "Backed up config: $BACKUP_DIR/config.json.before-rust-authority"

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
    'transaction_authority': 'preview',
    'execute_apply_manifest': False,
    'allow_rust_file_writes': False,
    'allow_rust_libreqos_apply': False,
    'append_transaction_journal': False,
    'allow_transaction_journal_writes': False,
    'include_rehearsal_journal_entries': False,
    'allow_dry_run_journal_entries': False,
    'execute_rollback': False,
    'allow_rust_rollback_file_writes': False,
    'rollback_authority': 'preview',
    'collector_authority_mode': 'python_authoritative',
    'collector_authority_require_python_fallback': True,
})
text = json.dumps(cfg, indent=2) + '\n'
fd, tmp = tempfile.mkstemp(prefix=path.name + '.', dir=str(path.parent))
with os.fdopen(fd, 'w') as f:
    f.write(text)
os.replace(tmp, path)
print('config patched for Rust authoritative gate mode')
LQOSYNC_PROMOTE_PY

if [ -d "$INSTALL_DIR" ]; then
  log "Validating config after patch..."
  (cd "$INSTALL_DIR" && CONFIG_PATH="$CONFIG_PATH" python3 scripts/validate_config_example.py >/dev/null)
  (cd "$INSTALL_DIR" && CONFIG_PATH="$CONFIG_PATH" python3 scripts/doctor.py "$CONFIG_PATH" || true)
fi

log "Rust authoritative gate mode is enabled."
log "Active safety boundaries:"
log "  - Rust validation is enforced."
log "  - Rust sync-plan blockers are fail-closed."
log "  - Python fallback is preserved."
log "  - Rust file writes are NOT enabled."
log "  - Rust LibreQoS.py external apply is NOT enabled."
log "  - backup_before_apply=true and file_drift_policy=block."

if as_bool "$RESTART_SERVICE"; then
  log "Restarting $SERVICE_NAME as requested..."
  systemctl restart "$SERVICE_NAME"
else
  log "Service not restarted. Restart manually after review: sudo systemctl restart $SERVICE_NAME"
fi
