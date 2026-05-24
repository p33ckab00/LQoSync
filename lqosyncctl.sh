#!/usr/bin/env bash
set -euo pipefail

# LQoSync one-line operator control script.
# Remote examples:
#   curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- install
#   curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- update
#   curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- uninstall
#   curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- adopt
#   curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- check
#   curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- verify

COMMAND="${1:-help}"
REPO_URL="${LQOSYNC_REPO_URL:-https://github.com/p33ckab00/LQoSync.git}"
BRANCH="${LQOSYNC_BRANCH:-lqosync-in-rust}"
INSTALL_DIR="${LQOSYNC_INSTALL_DIR:-/opt/LQoSync}"
LIBREQOS_SRC="${LIBREQOS_SRC:-/opt/libreqos/src}"
SERVICE_WEB="${LQOSYNC_SERVICE_NAME:-lqosync}"
SERVICE_CORE="${LQOSYNC_CORE_SERVICE_NAME:-lqosync-core}"
TS="$(date +%Y%m%d_%H%M%S)"
BACKUP_ROOT="${LQOSYNC_BACKUP_ROOT:-/root/lqosync_one_line_backups}"

log() { echo "[LQoSync One-Line] $*"; }
warn() { echo "[LQoSync One-Line] WARNING: $*"; }
fail() { echo "[LQoSync One-Line] ERROR: $*" >&2; exit 1; }

need_root() {
  if [ "${EUID:-$(id -u)}" -ne 0 ]; then
    fail "Run with sudo/root, e.g. curl ... | sudo bash -s -- $COMMAND"
  fi
}

usage() {
  cat <<USAGE
LQoSync one-line control

Commands:
  install   Fresh/adopt install from GitHub branch $BRANCH, preserve LibreQoS files, build Rust, and enable the Rust backend service without auto-start surprise.
  update    Update existing /opt/LQoSync from GitHub branch $BRANCH, preserve LibreQoS files, rebuild Rust, and verify the Rust-only backend runtime.
  uninstall Remove the LQoSync service/runtime integration with safe backups; keep LibreQoS working files unless explicit uninstall env vars say otherwise.
  adopt     Re-apply the lqosync runtime user, ownership, ACLs, and managed-file permissions across /opt/LQoSync and /opt/libreqos/src.
  check     Read-only status check: branch, services, ports, Rust/Cargo, config summary.
  verify    Run package + Rust authority verification scripts.
  start     Start lqosync-core.
  stop      Stop lqosync-core and any stale legacy Python service.
  restart   Restart lqosync-core.
  repair    Re-run install safely without Git update.

One-line examples:
  curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- install
  curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- update
  curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- uninstall
  curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- adopt
  curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- check
  curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync-in-rust/lqosyncctl.sh | sudo bash -s -- verify
USAGE
}

backup_live_files() {
  local dir="$BACKUP_ROOT/$TS"
  mkdir -p "$dir/libreqos_src"
  cp -a "$LIBREQOS_SRC/config.json" "$dir/libreqos_src/config.json" 2>/dev/null || true
  cp -a "$LIBREQOS_SRC/ShapedDevices.csv" "$dir/libreqos_src/ShapedDevices.csv" 2>/dev/null || true
  cp -a "$LIBREQOS_SRC/network.json" "$dir/libreqos_src/network.json" 2>/dev/null || true
  cp -a "$INSTALL_DIR/users.json" "$dir/users.json" 2>/dev/null || true
  cp -a "$INSTALL_DIR/.env" "$dir/.env" 2>/dev/null || true
  cp -a "$INSTALL_DIR/state" "$dir/state" 2>/dev/null || true
  cp -a "/etc/systemd/system/${SERVICE_WEB}.service" "$dir/${SERVICE_WEB}.service" 2>/dev/null || true
  cp -a "/etc/systemd/system/${SERVICE_CORE}.service" "$dir/${SERVICE_CORE}.service" 2>/dev/null || true
  log "Backup saved: $dir"
}

backup_permission_snapshot() {
  if [ -x "$INSTALL_DIR/scripts/snapshot-runtime-permissions.sh" ]; then
    bash "$INSTALL_DIR/scripts/snapshot-runtime-permissions.sh"
    return
  fi

  local snapshot_root="${LQOSYNC_PERMISSION_SNAPSHOT_ROOT:-/root/lqosync_permission_snapshots}"
  local force="${LQOSYNC_FORCE_PERMISSION_SNAPSHOT:-false}"
  case "$(printf '%s' "$force" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|y|on) force=true ;;
    *) force=false ;;
  esac

  if [ "$force" != "true" ]; then
    if [ -f "$snapshot_root/latest.path" ]; then
      local existing=""
      read -r existing < "$snapshot_root/latest.path" || true
      if [ -n "$existing" ] && [ -f "$existing/metadata.tsv" ]; then
        log "Existing original permission snapshot kept: $existing"
        return
      fi
    fi
    if [ -f "$snapshot_root/latest/metadata.tsv" ]; then
      log "Existing original permission snapshot kept: $snapshot_root/latest"
      return
    fi
  fi

  local dir="$snapshot_root/$TS"
  local metadata_tmp="$dir/metadata.tsv.tmp"
  local metadata="$dir/metadata.tsv"
  local acl_restore="$dir/acl.restore"
  mkdir -p "$dir"
  chmod 700 "$snapshot_root" "$dir" 2>/dev/null || true
  : > "$metadata_tmp"

  record_missing() { printf 'missing\t\t\t\t%s\n' "$1" >> "$metadata_tmp"; }
  record_one() {
    local p="$1"
    if [ -e "$p" ] || [ -L "$p" ]; then
      find "$p" -maxdepth 0 -printf '%y\t%m\t%U\t%G\t%p\n' >> "$metadata_tmp"
    else
      record_missing "$p"
    fi
  }
  record_tree() {
    local p="$1"
    if [ -d "$p" ]; then
      find "$p" -xdev -printf '%y\t%m\t%U\t%G\t%p\n' >> "$metadata_tmp"
    else
      record_missing "$p"
    fi
  }

  record_tree "$INSTALL_DIR"
  record_one "$LIBREQOS_SRC"
  record_one "$LIBREQOS_SRC/config.json"
  record_one "$LIBREQOS_SRC/ShapedDevices.csv"
  record_one "$LIBREQOS_SRC/network.json"
  record_one /var/log/lqosync.log

  {
    printf '#type\tmode\tuid\tgid\tpath\n'
    awk -F '\t' 'NF >= 5 && !seen[$5]++ { print }' "$metadata_tmp"
  } > "$metadata"
  rm -f "$metadata_tmp"

  : > "$acl_restore"
  if command -v getfacl >/dev/null 2>&1; then
    [ -d "$INSTALL_DIR" ] && getfacl -R -p "$INSTALL_DIR" >> "$acl_restore" 2>/dev/null || true
    for p in "$LIBREQOS_SRC" "$LIBREQOS_SRC/config.json" "$LIBREQOS_SRC/ShapedDevices.csv" "$LIBREQOS_SRC/network.json" /var/log/lqosync.log; do
      if [ -e "$p" ] || [ -L "$p" ]; then
        getfacl -p "$p" >> "$acl_restore" 2>/dev/null || true
      fi
    done
  fi

  cat > "$dir/manifest.env" <<EOF
created_at=$TS
install_dir=$INSTALL_DIR
libreqos_src_dir=$LIBREQOS_SRC
metadata=$metadata
acl_restore=$acl_restore
EOF

  ln -sfn "$dir" "$snapshot_root/latest"
  printf '%s\n' "$dir" > "$snapshot_root/latest.path"
  log "Original permission snapshot saved: $dir"
}

install_packages() {
  apt-get update -qq
  DEBIAN_FRONTEND=noninteractive apt-get install -y git curl rsync sudo acl python3 python3-venv python3-pip build-essential pkg-config libssl-dev
}

ensure_acl_available() {
  if command -v setfacl >/dev/null 2>&1 && command -v getfacl >/dev/null 2>&1; then
    return
  fi
  if command -v apt-get >/dev/null 2>&1; then
    log "Installing ACL tooling for permission adoption..."
    apt-get update -qq
    DEBIAN_FRONTEND=noninteractive apt-get install -y acl
    return
  fi
  fail "setfacl/getfacl are missing. Install the acl package before running adopt."
}

ensure_rustup_cargo() {
  export PATH="/root/.cargo/bin:$PATH"
  if ! command -v cargo >/dev/null 2>&1 || ! cargo --version 2>/dev/null | grep -Eq 'cargo 1\.(8[0-9]|9[0-9]|[1-9][0-9]{2})'; then
    log "Installing/updating Rust stable toolchain with rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1091
    source /root/.cargo/env
    rustup default stable
    rustup update stable
  fi
  export PATH="/root/.cargo/bin:$PATH"
  hash -r || true
  log "cargo: $(command -v cargo) $(cargo --version 2>/dev/null || true)"
  log "rustc: $(command -v rustc) $(rustc --version 2>/dev/null || true)"
}

safe_git() {
  git config --global --add safe.directory "$INSTALL_DIR" 2>/dev/null || true
}

ensure_source() {
  safe_git
  if [ -d "$INSTALL_DIR/.git" ]; then
    log "Updating Git source: $INSTALL_DIR ($BRANCH)"
    git -C "$INSTALL_DIR" fetch origin "$BRANCH"
    git -C "$INSTALL_DIR" switch "$BRANCH" 2>/dev/null || git -C "$INSTALL_DIR" checkout "$BRANCH"
    git -C "$INSTALL_DIR" pull --ff-only origin "$BRANCH"
    return
  fi
  if [ -e "$INSTALL_DIR" ]; then
    local legacy="${INSTALL_DIR}.legacy.$TS"
    warn "Existing non-Git install found. Moving to $legacy"
    mv "$INSTALL_DIR" "$legacy"
  fi
  log "Cloning $REPO_URL branch $BRANCH to $INSTALL_DIR"
  git clone --branch "$BRANCH" --single-branch "$REPO_URL" "$INSTALL_DIR"
  safe_git
}

run_stable_install() {
  cd "$INSTALL_DIR"
  export PATH="/root/.cargo/bin:$PATH"
  export LQOSYNC_INIT_POLICY="${LQOSYNC_INIT_POLICY:-preserve_existing}"
  export LQOSYNC_SERVICE_START_POLICY="${LQOSYNC_SERVICE_START_POLICY:-enable_only}"
  bash install-rust-stable-safe.sh
}

run_permission_adoption() {
  [ -x "$INSTALL_DIR/scripts/adopt-runtime-permissions.sh" ] || fail "Missing executable permission adoption script: $INSTALL_DIR/scripts/adopt-runtime-permissions.sh"
  bash "$INSTALL_DIR/scripts/adopt-runtime-permissions.sh"
}

run_verify() {
  cd "$INSTALL_DIR"
  export PATH="/root/.cargo/bin:$PATH"
  local scripts=(
    scripts/verify-full-rust-daemon-boundary.sh
    scripts/verify-rust-scheduler-authority.sh
    scripts/verify-rust-sync-plan-gate-import-hardening.sh
    scripts/verify-dashboard-backend-wiring.sh
    scripts/verify-one-line-operations.sh
  )
  for s in "${scripts[@]}"; do
    [ -x "$s" ] && bash "$s" || true
  done
  python3 scripts/release_check.py
  python3 scripts/regression_check.py
  python3 scripts/stable_release_check.py
}

run_check() {
  echo "== LQoSync source =="
  if [ -d "$INSTALL_DIR/.git" ]; then
    safe_git
    git -C "$INSTALL_DIR" branch --show-current || true
    git -C "$INSTALL_DIR" log --oneline -1 || true
  else
    echo "not git-managed: $INSTALL_DIR"
  fi
  echo
  echo "== VERSION =="
  cat "$INSTALL_DIR/VERSION" 2>/dev/null || true
  echo
  echo "== Rust/Cargo =="
  export PATH="/root/.cargo/bin:$PATH"
  command -v cargo || true; cargo --version 2>/dev/null || true
  command -v rustc || true; rustc --version 2>/dev/null || true
  echo
  echo "== Services =="
  systemctl status "$SERVICE_CORE" --no-pager -l 2>/dev/null || true
  if systemctl list-unit-files 2>/dev/null | grep -q "^${SERVICE_WEB}\\.service"; then
    echo
    echo "== Retired legacy service =="
    systemctl status "$SERVICE_WEB" --no-pager -l 2>/dev/null || true
  fi
  echo
  echo "== Ports =="
  ss -ltnp | grep -E ':9202|:9203|:80|:443' || true
  echo
  echo "== Config summary =="
  python3 - <<PY 2>/dev/null || true
import json
p='$LIBREQOS_SRC/config.json'
c=json.load(open(p))
print('config=',p)
print('scheduler.enabled=', c.get('scheduler',{}).get('enabled'))
print('scheduler.engine=', c.get('scheduler',{}).get('engine'))
print('auto_apply=', c.get('app',{}).get('auto_apply'))
print('rust.enabled=', c.get('rust_core',{}).get('enabled'))
print('rust.full_authority=', c.get('rust_core',{}).get('full_rust_backend_authority'))
print('python_mutation_fallback=', c.get('rust_core',{}).get('python_mutation_fallback'))
print('python_backend_runtime_fallback_disabled=', c.get('rust_core',{}).get('python_backend_runtime_fallback_disabled'))
print('python_backend_service_removed=', c.get('rust_core',{}).get('python_backend_service_removed'))
PY
}

start_services() {
  systemctl daemon-reload
  systemctl start "$SERVICE_CORE" 2>/dev/null || true
  if systemctl list-unit-files 2>/dev/null | grep -q "^${SERVICE_WEB}\\.service"; then
    warn "Legacy Python backend service is still installed: $SERVICE_WEB"
  fi
  run_check
}
stop_services() {
  systemctl stop "$SERVICE_WEB" 2>/dev/null || true
  systemctl stop "$SERVICE_CORE" 2>/dev/null || true
  systemctl stop lqos_shaped_sync 2>/dev/null || true
}
restart_services() {
  systemctl daemon-reload
  systemctl restart "$SERVICE_CORE" 2>/dev/null || true
  if systemctl list-unit-files 2>/dev/null | grep -q "^${SERVICE_WEB}\\.service"; then
    warn "Legacy Python backend service remains installed: $SERVICE_WEB"
  fi
  run_check
}

case "$COMMAND" in
  install|update)
    need_root
    stop_services
    backup_live_files
    install_packages
    backup_permission_snapshot
    ensure_rustup_cargo
    ensure_source
    run_stable_install
    run_permission_adoption
    run_verify
    log "Complete. Start Rust backend with: sudo /opt/LQoSync/lqosyncctl.sh start"
    ;;
  repair)
    need_root
    backup_live_files
    install_packages
    backup_permission_snapshot
    ensure_rustup_cargo
    run_stable_install
    run_permission_adoption
    run_verify
    ;;
  uninstall)
    need_root
    backup_live_files
    [ -x "$INSTALL_DIR/uninstall.sh" ] || fail "Missing uninstall helper: $INSTALL_DIR/uninstall.sh"
    bash "$INSTALL_DIR/uninstall.sh"
    ;;
  adopt)
    need_root
    ensure_acl_available
    run_permission_adoption
    ;;
  check)
    need_root
    run_check
    ;;
  verify)
    need_root
    ensure_rustup_cargo
    run_verify
    ;;
  start)
    need_root
    start_services
    ;;
  stop)
    need_root
    stop_services
    ;;
  restart)
    need_root
    restart_services
    ;;
  help|-h|--help)
    usage
    ;;
  *)
    usage
    fail "Unknown command: $COMMAND"
    ;;
esac
