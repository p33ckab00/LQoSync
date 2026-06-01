#!/usr/bin/env bash
set -Eeuo pipefail

# LQoSync one-line installer
# Target branch layout:
#   install.sh
#   src/updatecsv.py
#   templates/config.json
#   docs/README.md
#
# One-line install:
#   curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/one-line-cli/install.sh | sudo bash
#
# Recommended testing-production example:
#   curl -fsSL https://raw.githubusercontent.com/p33ckab00/LQoSync/one-line-cli/install.sh | sudo RESERVE_CORES=2 MAX_COUNT=6 ROOT_DOWN=620 ROOT_UP=620 bash

APP_DIR="${APP_DIR:-/opt/libreqos/src}"
SERVICE_NAME="${SERVICE_NAME:-lqosync-updatecsv}"
REPO_RAW_BASE="${REPO_RAW_BASE:-https://raw.githubusercontent.com/p33ckab00/LQoSync/one-line-cli}"
CONFIG_MODE="${CONFIG_MODE:-merge}"          # merge | replace
FORCE_NO_PLAN_NODE="${FORCE_NO_PLAN_NODE:-1}"
CREATE_SERVICE_IF_MISSING="${CREATE_SERVICE_IF_MISSING:-1}"
DRY_RUN="${DRY_RUN:-0}"
AUTO_ENABLE="${AUTO_ENABLE:-0}"
AUTO_START="${AUTO_START:-0}"

UPDATECSV_REMOTE_PATH="${UPDATECSV_REMOTE_PATH:-src/updatecsv.py}"
CONFIG_REMOTE_PATH="${CONFIG_REMOTE_PATH:-templates/config.json}"
STATE_FILE_NAME="${STATE_FILE_NAME:-logical_core_assignments.json}"

log() { printf '\033[1;32m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mWARN:\033[0m %s\n' "$*" >&2; }
fail() { printf '\033[1;31mERROR:\033[0m %s\n' "$*" >&2; exit 1; }

need_root() {
  [[ "${EUID}" -eq 0 ]] || fail "Run as root. Example: curl -fsSL <url> | sudo bash"
}

install_pkg_if_missing() {
  local bin="$1" pkg="$2"
  if ! command -v "${bin}" >/dev/null 2>&1; then
    log "Installing dependency: ${pkg}"
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install -y "${pkg}"
  fi
}

download_url() {
  local url="$1" out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "${url}" -o "${out}"
  else
    wget -qO "${out}" "${url}"
  fi
}

service_exists() {
  systemctl list-unit-files "${SERVICE_NAME}.service" >/dev/null 2>&1 || [[ -f "/etc/systemd/system/${SERVICE_NAME}.service" ]]
}

create_service_if_missing() {
  if service_exists; then
    return 0
  fi
  if [[ "${CREATE_SERVICE_IF_MISSING}" != "1" ]]; then
    warn "${SERVICE_NAME}.service not found. File install will continue; set CREATE_SERVICE_IF_MISSING=1 to create a Python systemd service."
    return 0
  fi

  log "Creating systemd service: ${SERVICE_NAME}.service"
  cat > "/etc/systemd/system/${SERVICE_NAME}.service" <<EOF
[Unit]
Description=LQoSync MikroTik to LibreQoS Sync
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=${APP_DIR}
ExecStart=/usr/bin/python3 ${APP_DIR}/updatecsv.py
Restart=always
RestartSec=10
User=root

[Install]
WantedBy=multi-user.target
EOF
  systemctl daemon-reload
  if [[ "${AUTO_ENABLE}" == "1" ]]; then
    systemctl enable "${SERVICE_NAME}.service"
  else
    log "Service created but not enabled yet. Enable after editing config: systemctl enable ${SERVICE_NAME}"
  fi
}

merge_config() {
  export APP_DIR TMP_DIR CONFIG_MODE FORCE_NO_PLAN_NODE STATE_FILE_NAME
  python3 - <<'PYCONF'
import json
import os
from pathlib import Path

app_dir = Path(os.environ["APP_DIR"])
tmp_dir = Path(os.environ["TMP_DIR"])
config_path = app_dir / "config.json"
template_path = tmp_dir / "config.template.json"
mode = os.environ.get("CONFIG_MODE", "merge").strip().lower()
force_no_plan = os.environ.get("FORCE_NO_PLAN_NODE", "1") == "1"


def load(path: Path, default):
    try:
        if path.exists():
            return json.loads(path.read_text(encoding="utf-8"))
    except Exception as e:
        raise SystemExit(f"Failed to read {path}: {e}")
    return default


def deep_merge_keep_existing(existing, template):
    """Conservative merge: keep live secrets/router list; import new logical-core network block."""
    if not isinstance(existing, dict):
        existing = {}
    final = dict(existing)
    final["network"] = template.get("network", {})
    for section in ("runtime", "libreqos", "bandwidth_defaults"):
        final.setdefault(section, template.get(section, {}))

    final.setdefault("telegram", {})
    for key, value in template.get("telegram", {}).items():
        final["telegram"].setdefault(key, value)

    if not final.get("routers"):
        final["routers"] = template.get("routers", [])
    return final


template = load(template_path, {})
existing = load(config_path, {})

if mode == "replace" or not existing:
    final = template
else:
    final = deep_merge_keep_existing(existing, template)

network = final.setdefault("network", {})
network["mode"] = "logical_core_root"
logical = network.setdefault("logical_roots", {})
logical["enabled"] = True
logical["count_from_cpu"] = True
logical["state_file"] = os.environ.get("STATE_FILE_NAME", logical.get("state_file", "logical_core_assignments.json"))
logical["distribution"] = "sticky_least_loaded"
logical["rebalance"] = True

numeric_overrides = {
    "RESERVE_CORES": (logical, "reserve_cores"),
    "MIN_COUNT": (logical, "min_count"),
    "MAX_COUNT": (logical, "max_count"),
    "FALLBACK_COUNT": (logical, "fallback_count"),
    "REBALANCE_THRESHOLD": (logical, "rebalance_threshold_clients"),
    "MAX_MOVES": (logical, "max_rebalance_moves_per_cycle"),
    "ROOT_DOWN": (network, "root_download_mbps"),
    "ROOT_UP": (network, "root_upload_mbps"),
}
for env_name, (obj, key) in numeric_overrides.items():
    raw = os.environ.get(env_name, "").strip()
    if not raw:
        continue
    try:
        obj[key] = int(raw)
    except ValueError:
        try:
            obj[key] = float(raw)
        except ValueError:
            raise SystemExit(f"Invalid numeric value for {env_name}: {raw}")

if os.environ.get("LCORE_PREFIX", "").strip():
    logical["prefix"] = os.environ["LCORE_PREFIX"].strip()
if os.environ.get("ROOT_NAME", "").strip():
    network["root_name"] = os.environ["ROOT_NAME"].strip()

if force_no_plan:
    for router in final.get("routers", []):
        if isinstance(router, dict):
            router.setdefault("pppoe", {})["per_plan_node"] = False

config_path.write_text(json.dumps(final, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print(f"Config written: {config_path}")
PYCONF
}

print_bucket_plan() {
  cd "${APP_DIR}"
  python3 - <<'PYPLAN'
import json, os
from pathlib import Path
cfg = json.loads(Path("config.json").read_text(encoding="utf-8"))
net = cfg.get("network", {})
logical = net.get("logical_roots", {})
cores = os.cpu_count() or int(logical.get("fallback_count", 4))
reserve = int(logical.get("reserve_cores", 2))
count = max(1, cores - reserve)
count = max(int(logical.get("min_count", 1)), count)
count = min(int(logical.get("max_count", count)), count)
print(f"CPU logical cores : {cores}")
print(f"Reserved cores    : {reserve}")
print(f"Logical buckets   : {count}")
print(f"Bucket prefix     : {logical.get('prefix', 'LCORE')}")
print(f"Root node         : {net.get('root_name', 'LibreQoS-ROOT')}")
print(f"Root bandwidth    : {net.get('root_download_mbps')}/{net.get('root_upload_mbps')} Mbps")
print(f"Distribution      : {logical.get('distribution')}")
print(f"State file        : {logical.get('state_file')}")
PYPLAN
}

main() {
  need_root
  install_pkg_if_missing python3 python3
  if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install -y curl
  fi

  [[ -d "${APP_DIR}" ]] || fail "APP_DIR does not exist: ${APP_DIR}. Set APP_DIR=/path/to/libreqos/src"

  TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
  BACKUP_DIR="${BACKUP_DIR:-/opt/libreqos/backups/logical-core-root-${TIMESTAMP}}"
  TMP_DIR="$(mktemp -d)"
  export TMP_DIR
  trap 'rm -rf "${TMP_DIR}"' EXIT

  log "Repo raw base: ${REPO_RAW_BASE}"
  log "Downloading files"
  download_url "${REPO_RAW_BASE}/${UPDATECSV_REMOTE_PATH}" "${TMP_DIR}/updatecsv.py"
  download_url "${REPO_RAW_BASE}/${CONFIG_REMOTE_PATH}" "${TMP_DIR}/config.template.json"

  log "Validating downloads"
  python3 -m py_compile "${TMP_DIR}/updatecsv.py"
  python3 -m json.tool "${TMP_DIR}/config.template.json" >/dev/null

  OLD_OWNER="0:0"
  if [[ -e "${APP_DIR}/updatecsv.py" ]]; then
    OLD_OWNER="$(stat -c '%u:%g' "${APP_DIR}/updatecsv.py")"
  elif [[ -e "${APP_DIR}/config.json" ]]; then
    OLD_OWNER="$(stat -c '%u:%g' "${APP_DIR}/config.json")"
  elif id pi >/dev/null 2>&1; then
    OLD_OWNER="pi:pi"
  fi

  log "Backup directory: ${BACKUP_DIR}"
  mkdir -p "${BACKUP_DIR}"
  for f in updatecsv.py config.json ShapedDevices.csv network.json "${STATE_FILE_NAME}"; do
    [[ -e "${APP_DIR}/${f}" ]] && cp -a "${APP_DIR}/${f}" "${BACKUP_DIR}/${f}.bak"
  done

  if [[ "${DRY_RUN}" == "1" ]]; then
    log "DRY_RUN=1 complete. No files changed."
    exit 0
  fi

  create_service_if_missing

  if service_exists; then
    log "Stopping service: ${SERVICE_NAME}"
    systemctl stop "${SERVICE_NAME}" || true
  fi

  log "Installing updatecsv.py"
  install -m 0755 "${TMP_DIR}/updatecsv.py" "${APP_DIR}/updatecsv.py"
  chown "${OLD_OWNER}" "${APP_DIR}/updatecsv.py" || true

  log "Installing/merging config.json (${CONFIG_MODE})"
  merge_config
  python3 -m json.tool "${APP_DIR}/config.json" >/dev/null
  chown "${OLD_OWNER}" "${APP_DIR}/config.json" || true
  chmod 600 "${APP_DIR}/config.json" || true

  log "Final syntax check"
  cd "${APP_DIR}"
  python3 -m py_compile updatecsv.py

  log "Logical-core bucket plan"
  print_bucket_plan

  if service_exists; then
    systemctl daemon-reload
    if [[ "${AUTO_ENABLE}" == "1" ]]; then
      log "Enabling service: ${SERVICE_NAME}"
      systemctl enable "${SERVICE_NAME}"
    fi
    if [[ "${AUTO_START}" == "1" ]]; then
      log "Starting/restarting service: ${SERVICE_NAME}"
      systemctl restart "${SERVICE_NAME}"
      systemctl --no-pager --full status "${SERVICE_NAME}" || true
    else
      log "Installed but service was not started. Edit config first, then run: systemctl start ${SERVICE_NAME}"
    fi
  else
    warn "Service not found. Manual test run: cd ${APP_DIR} && python3 updatecsv.py"
  fi

  echo
  log "DONE: one-line CLI files installed"
  echo "Installed files:"
  echo "  ${APP_DIR}/updatecsv.py"
  echo "  ${APP_DIR}/config.json"
  echo "Backup saved at: ${BACKUP_DIR}"
  echo
  echo "Next steps:"
  echo "  1) Edit config: sudo nano ${APP_DIR}/config.json"
  echo "  2) Optional edit script: sudo nano ${APP_DIR}/updatecsv.py"
  echo "  3) Validate: sudo python3 -m json.tool ${APP_DIR}/config.json >/dev/null && sudo python3 -m py_compile ${APP_DIR}/updatecsv.py"
  echo "  4) Test once: cd ${APP_DIR} && sudo python3 updatecsv.py"
  echo "  5) Enable/start: sudo systemctl enable --now ${SERVICE_NAME}"
  echo "Logs: sudo journalctl -u ${SERVICE_NAME} -f"
}

main "$@"
