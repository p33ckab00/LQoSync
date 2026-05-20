#!/usr/bin/env bash
set -euo pipefail

# Build a non-destructive recovery bundle before or after Rust-authoritative
# mutation. This copies config/state/managed LibreQoS files and captures basic
# service/self-test metadata. It never restores or deletes anything.

INSTALL_DIR="${LQOSYNC_INSTALL_DIR:-/opt/LQoSync}"
CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"
CORE_BIN="${LQOSYNC_CORE_BIN:-$(command -v lqosync-core 2>/dev/null || true)}"
TS="$(date +%Y%m%d_%H%M%S)"
BUNDLE_ROOT="${RUST_AUTHORITY_RECOVERY_BUNDLE_DIR:-$INSTALL_DIR/state/rust_authority_recovery}"
BUNDLE_DIR="$BUNDLE_ROOT/$TS"
mkdir -p "$BUNDLE_DIR"

log() { echo "[rust-authority-recovery-bundle] $*"; }
copy_if_exists() {
  local src="$1" dst="$2"
  if [ -e "$src" ]; then
    mkdir -p "$(dirname "$dst")"
    cp -a "$src" "$dst"
    log "copied|$src|$dst"
  else
    log "missing|$src"
  fi
}

copy_if_exists "$CONFIG_PATH" "$BUNDLE_DIR/config.json"

python3 - "$CONFIG_PATH" "$BUNDLE_DIR" <<'PY'
from __future__ import annotations
import json, pathlib, shutil, sys
config_path = pathlib.Path(sys.argv[1])
bundle = pathlib.Path(sys.argv[2])
if not config_path.exists():
    raise SystemExit(0)
cfg=json.loads(config_path.read_text())
paths=cfg.get('paths') or {}
for key, rel in [
    ('shaped_devices_csv','libreqos/ShapedDevices.csv'),
    ('network_json','libreqos/network.json'),
    ('runtime_state','state/runtime_state.json'),
    ('policy_state','state/policy_state.json'),
    ('audit_log','logs/audit.log'),
]:
    value=paths.get(key)
    if not value:
        continue
    src=pathlib.Path(str(value))
    dst=bundle/rel
    if src.exists():
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)
        print(f"[rust-authority-recovery-bundle] copied|{src}|{dst}")
PY

if [ -d "$INSTALL_DIR/.git" ]; then
  (cd "$INSTALL_DIR" && git rev-parse HEAD > "$BUNDLE_DIR/git_head.txt" 2>/dev/null || true)
  (cd "$INSTALL_DIR" && git status --short > "$BUNDLE_DIR/git_status_short.txt" 2>/dev/null || true)
fi

if command -v systemctl >/dev/null 2>&1; then
  systemctl status lqosync --no-pager > "$BUNDLE_DIR/systemctl_lqosync.txt" 2>&1 || true
  systemctl status lqosync-core --no-pager > "$BUNDLE_DIR/systemctl_lqosync_core.txt" 2>&1 || true
fi

if [ -n "$CORE_BIN" ] && [ -x "$CORE_BIN" ]; then
  "$CORE_BIN" > "$BUNDLE_DIR/lqosync_core_noop.txt" 2>&1 || true
  "$CORE_BIN" <<'JSON' > "$BUNDLE_DIR/lqosync_core_self_test.json" 2>&1 || true
{"version":"1","op":"self-test","payload":{}}
JSON
fi

cat > "$BUNDLE_DIR/README.txt" <<EOF2
LQoSync Rust authority recovery bundle
Created: $(date -Is)
Config: $CONFIG_PATH
Install: $INSTALL_DIR

This directory is a non-destructive snapshot. It is intended for operator review
before restoring any file manually. Do not blindly copy files back onto a live
LibreQoS host without checking timestamps and current service state.
EOF2

python3 - "$BUNDLE_DIR" <<'PY'
from __future__ import annotations
import hashlib, json, pathlib, sys
root=pathlib.Path(sys.argv[1])
items=[]
for path in sorted(p for p in root.rglob('*') if p.is_file()):
    data=path.read_bytes()
    items.append({"path": str(path.relative_to(root)), "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()})
(root/'MANIFEST.json').write_text(json.dumps({"schema":"lqosync.rust_authority_recovery_bundle.v1","files":items}, indent=2)+"\n")
PY

log "bundle-ready|$BUNDLE_DIR"
