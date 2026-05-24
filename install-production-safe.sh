#!/usr/bin/env bash
set -euo pipefail

# Production-safe LQoSync installer wrapper.
# Goal: install/upgrade without replacing live LibreQoS files and without starting
# scheduler-capable runtime until the operator has reviewed config and run dry-run.
#
# Defaults are intentionally conservative:
#   - preserve existing /opt/libreqos/src/config.json, ShapedDevices.csv, network.json
#   - create timestamped backups before any installer action
#   - retire the Python backend service and keep Rust as the only backend runtime
#   - build/install the Rust core daemon and enable it without forcing an immediate restart by default
#
# Common safe command:
#   sudo bash install-production-safe.sh
#
# Start service manually after review:
#   sudo systemctl start lqosync-core
# Then open:
#   http://<server-ip>:9202

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${LQOSYNC_INSTALL_DIR:-/opt/LQoSync}"
LIBREQOS_SRC_DIR="${LIBREQOS_SRC_DIR:-${LIBREQOS_SRC:-/opt/libreqos/src}}"
SERVICE_NAME="${LQOSYNC_SERVICE_NAME:-lqosync}"
CORE_SERVICE_NAME="${LQOSYNC_CORE_SERVICE_NAME:-lqosync-core}"
INIT_POLICY="${LQOSYNC_INIT_POLICY:-preserve_existing}"
SERVICE_START_POLICY="${LQOSYNC_SERVICE_START_POLICY:-enable_only}"
INSTALL_RUST_CORE="${INSTALL_RUST_CORE:-true}"
INSTALL_RUST_CORE_DAEMON="${INSTALL_RUST_CORE_DAEMON:-true}"
RUN_PRECHECKS="${RUN_PRECHECKS:-true}"
RUN_POSTCHECKS="${RUN_POSTCHECKS:-true}"
STRICT_RUST="${STRICT_RUST:-true}"
TS="$(date +%Y%m%d_%H%M%S)"
BACKUP_ROOT="${LQOSYNC_BACKUP_ROOT:-/root/lqosync_production_install_backups}"
BACKUP_DIR="$BACKUP_ROOT/$TS"

log() { echo "[LQoSync Production Install] $*"; }
warn() { echo "[LQoSync Production Install] WARNING: $*"; }
fail() { echo "[LQoSync Production Install] ERROR: $*" >&2; exit 1; }

as_bool() {
  case "$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|y|on) return 0 ;;
    0|false|no|n|off|'') return 1 ;;
    *) return 1 ;;
  esac
}

ensure_rust_build_toolchain() {
  log "Ensuring Rust build toolchain is available..."
  apt-get update -qq
  DEBIAN_FRONTEND=noninteractive apt-get install -y curl build-essential pkg-config libssl-dev

  export PATH="/root/.cargo/bin:$PATH"
  if ! command -v cargo >/dev/null 2>&1 || ! cargo --version 2>/dev/null | grep -Eq 'cargo 1\.(8[0-9]|9[0-9]|[1-9][0-9]{2})'; then
    log "Installing/updating Rust stable toolchain with rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    if [ -f /root/.cargo/env ]; then
      # shellcheck disable=SC1091
      . /root/.cargo/env
    fi
    rustup default stable
    rustup update stable
  fi
  export PATH="/root/.cargo/bin:$PATH"
  hash -r || true
  log "cargo: $(command -v cargo) $(cargo --version 2>/dev/null || true)"
  log "rustc: $(command -v rustc) $(rustc --version 2>/dev/null || true)"
}

if [ "${EUID:-$(id -u)}" -ne 0 ]; then
  fail "Run as root: sudo bash install-production-safe.sh"
fi

case "$INIT_POLICY" in
  preserve_existing|create_missing_only|smart_confirm|overwrite_with_backup) ;;
  *) fail "Invalid LQOSYNC_INIT_POLICY=$INIT_POLICY" ;;
esac

case "$SERVICE_START_POLICY" in
  enable_only|restart|leave_stopped) ;;
  *) fail "Invalid LQOSYNC_SERVICE_START_POLICY=$SERVICE_START_POLICY" ;;
esac

mkdir -p "$BACKUP_DIR/libreqos_src"
log "Backup directory: $BACKUP_DIR"

backup_file() {
  local src="$1"
  local dst="$2"
  if [ -e "$src" ]; then
    mkdir -p "$(dirname "$dst")"
    cp -a "$src" "$dst"
    log "Backed up: $src"
  else
    log "Not present, skipped backup: $src"
  fi
}

backup_file "$LIBREQOS_SRC_DIR/config.json" "$BACKUP_DIR/libreqos_src/config.json"
backup_file "$LIBREQOS_SRC_DIR/ShapedDevices.csv" "$BACKUP_DIR/libreqos_src/ShapedDevices.csv"
backup_file "$LIBREQOS_SRC_DIR/network.json" "$BACKUP_DIR/libreqos_src/network.json"
backup_file "$INSTALL_DIR/users.json" "$BACKUP_DIR/users.json"
backup_file "$INSTALL_DIR/.env" "$BACKUP_DIR/.env"
backup_file "$INSTALL_DIR/state" "$BACKUP_DIR/state"
backup_file "/etc/systemd/system/${SERVICE_NAME}.service" "$BACKUP_DIR/${SERVICE_NAME}.service"
backup_file "/etc/systemd/system/${CORE_SERVICE_NAME}.service" "$BACKUP_DIR/${CORE_SERVICE_NAME}.service"
backup_file "/etc/sudoers.d/lqosync" "$BACKUP_DIR/sudoers.lqosync"

if as_bool "$RUN_PRECHECKS"; then
  log "Running non-mutating package prechecks..."
  bash -n "$SCRIPT_DIR/install.sh"
  bash -n "$SCRIPT_DIR/install-from-github.sh"
  bash -n "$SCRIPT_DIR/upgrade.sh"
  bash -n "$SCRIPT_DIR/uninstall.sh"
  for f in "$SCRIPT_DIR"/scripts/*.sh; do
    [ -f "$f" ] || continue
    bash -n "$f"
  done
  python3 - "$SCRIPT_DIR" <<'PY'
import ast
import pathlib
import sys
root = pathlib.Path(sys.argv[1])
errors = []
for path in root.rglob("*.py"):
    if any(part in {"venv", ".git", "__pycache__"} for part in path.parts):
        continue
    try:
        ast.parse(path.read_text(encoding="utf-8", errors="ignore"), filename=str(path))
    except SyntaxError as exc:
        errors.append(f"{path}: {exc}")
if errors:
    print("\n".join(errors))
    raise SystemExit(1)
PY
  python3 "$SCRIPT_DIR/scripts/validate_config_example.py"
fi

log "Installing with conservative live-system settings..."
log "Init policy: $INIT_POLICY"
log "Service start policy: $SERVICE_START_POLICY"
(
  cd "$SCRIPT_DIR"
  LQOSYNC_INIT_POLICY="$INIT_POLICY" \
  LQOSYNC_SERVICE_START_POLICY="$SERVICE_START_POLICY" \
  LQOSYNC_INSTALL_MODE=baremetal \
  LIBREQOS_SRC_DIR="$LIBREQOS_SRC_DIR" \
  bash install.sh
)

if as_bool "$INSTALL_RUST_CORE_DAEMON"; then
  INSTALL_RUST_CORE=true
fi

if as_bool "$INSTALL_RUST_CORE"; then
  ensure_rust_build_toolchain
  if ! command -v cargo >/dev/null 2>&1; then
    msg="Rust core requested but cargo is not installed. Install Rust/cargo first or rerun without INSTALL_RUST_CORE=true."
    if as_bool "$STRICT_RUST"; then
      fail "$msg"
    fi
    warn "$msg"
  else
    log "Building and installing optional Rust core..."
    (
      cd "$SCRIPT_DIR"
      bash scripts/build-rust-core.sh
      bash scripts/install-rust-core.sh
      if as_bool "$INSTALL_RUST_CORE_DAEMON"; then
        LQOSYNC_CORE_SERVICE_START_POLICY="$SERVICE_START_POLICY" \
        LQOSYNC_SERVICE_START_POLICY="$SERVICE_START_POLICY" \
        bash scripts/install-rust-core-daemon.sh
      fi
    )
  fi
else
  log "Rust core install skipped. Use INSTALL_RUST_CORE=true to opt in."
fi

if as_bool "$RUN_POSTCHECKS"; then
  log "Running post-install checks from installed directory..."
  if [ -d "$INSTALL_DIR" ]; then
    (
      cd "$INSTALL_DIR"
      python3 scripts/release_check.py
      CONFIG_PATH="$LIBREQOS_SRC_DIR/config.json" python3 scripts/doctor.py "$LIBREQOS_SRC_DIR/config.json" || true
    )
  else
    warn "Installed directory not found after install: $INSTALL_DIR"
  fi
fi

cat > "$BACKUP_DIR/production_install_summary.json" <<JSON
{
  "timestamp": "$TS",
  "install_dir": "$INSTALL_DIR",
  "libreqos_src_dir": "$LIBREQOS_SRC_DIR",
  "legacy_service_name": "$SERVICE_NAME",
  "core_service_name": "$CORE_SERVICE_NAME",
  "runtime_service_name": "$CORE_SERVICE_NAME",
  "init_policy": "$INIT_POLICY",
  "service_start_policy": "$SERVICE_START_POLICY",
  "install_rust_core": "$INSTALL_RUST_CORE",
  "install_rust_core_daemon": "$INSTALL_RUST_CORE_DAEMON",
  "backup_dir": "$BACKUP_DIR",
  "safe_defaults": {
    "preserve_live_libreqos_files": true,
    "service_not_started_by_default": true,
    "rust_only_backend_runtime": true
  }
}
JSON

log "Production-safe install wrapper complete."
log "Backup: $BACKUP_DIR"
if [ "$SERVICE_START_POLICY" = "enable_only" ] || [ "$SERVICE_START_POLICY" = "leave_stopped" ]; then
  log "Next: review config, run CLI/verification checks, then start with: sudo systemctl start $CORE_SERVICE_NAME"
  log "Rust web console: http://$(hostname -I | awk '{print $1}'):9202"
else
  log "Service was restarted by requested policy. Verify: sudo systemctl status $CORE_SERVICE_NAME --no-pager"
  log "Rust web console: http://$(hostname -I | awk '{print $1}'):9202"
fi
