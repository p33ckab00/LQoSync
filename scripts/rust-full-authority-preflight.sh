#!/usr/bin/env bash
set -euo pipefail

# Rust full-authority live preflight.
# This is non-mutating by default. With --write-stamp it writes a preflight
# stamp used by the Python scheduler shell to prove Rust authority was checked
# recently before production mutation is allowed.

WRITE_STAMP=false
if [ "${1:-}" = "--write-stamp" ]; then
  WRITE_STAMP=true
fi

INSTALL_DIR="${LQOSYNC_INSTALL_DIR:-/opt/LQoSync}"
CONFIG_PATH="${CONFIG_PATH:-/opt/libreqos/src/config.json}"
CORE_BIN="${LQOSYNC_CORE_BIN:-$(command -v lqosync-core 2>/dev/null || true)}"
STAMP_PATH="${RUST_AUTHORITY_PREFLIGHT_STAMP:-$INSTALL_DIR/state/rust_authority_preflight.json}"
TS_EPOCH="$(date +%s)"
TS_ISO="$(date -Is)"
FAIL=0
WARN=0
TMP_JSON="$(mktemp)"
trap 'rm -f "$TMP_JSON"' EXIT

log() { echo "[rust-authority-preflight] $*"; }
warn() { echo "[rust-authority-preflight] WARN: $*"; WARN=$((WARN+1)); }
fail_check() { echo "[rust-authority-preflight] FAIL: $*" >&2; FAIL=$((FAIL+1)); }

check_file() {
  local path="$1" label="$2"
  if [ -f "$path" ]; then
    log "ok|$label|$path"
  else
    fail_check "missing $label: $path"
  fi
}

check_dir() {
  local path="$1" label="$2"
  if [ -d "$path" ]; then
    log "ok|$label|$path"
  else
    fail_check "missing $label: $path"
  fi
}

check_file "$CONFIG_PATH" "config"
check_dir "$INSTALL_DIR" "install-dir"

if [ -z "$CORE_BIN" ]; then
  fail_check "lqosync-core not found in PATH; set LQOSYNC_CORE_BIN=/path/to/lqosync-core"
elif [ ! -x "$CORE_BIN" ]; then
  fail_check "lqosync-core is not executable: $CORE_BIN"
else
  log "ok|rust-core-bin|$CORE_BIN"
fi

if [ -f "$CONFIG_PATH" ]; then
python3 - "$CONFIG_PATH" > "$TMP_JSON" <<'PY'
from __future__ import annotations
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
cfg = json.loads(path.read_text())
rc = cfg.get("rust_core", {}) or {}
paths = cfg.get("paths", {}) or {}
lib = cfg.get("libreqos", {}) or {}
checks = []
def add(name, ok, detail=""):
    checks.append({"name": name, "ok": bool(ok), "detail": detail})
add("full_rust_backend_authority", rc.get("full_rust_backend_authority") is True, str(rc.get("full_rust_backend_authority")))
add("python_mutation_fallback_disabled", rc.get("python_mutation_fallback") is False, str(rc.get("python_mutation_fallback")))
add("transaction_authority", rc.get("transaction_authority") == "rust_full_authoritative", str(rc.get("transaction_authority")))
add("execute_apply_manifest", rc.get("execute_apply_manifest") is True, str(rc.get("execute_apply_manifest")))
add("allow_rust_file_writes", rc.get("allow_rust_file_writes") is True, str(rc.get("allow_rust_file_writes")))
add("allow_rust_libreqos_apply", rc.get("allow_rust_libreqos_apply") is True, str(rc.get("allow_rust_libreqos_apply")))
add("append_transaction_journal", rc.get("append_transaction_journal") is True, str(rc.get("append_transaction_journal")))
add("allow_transaction_journal_writes", rc.get("allow_transaction_journal_writes") is True, str(rc.get("allow_transaction_journal_writes")))
add("collector_output_authority", rc.get("collector_output_authority") == "rust_validate_all", str(rc.get("collector_output_authority")))
for key in ("shaped_devices_csv", "network_json", "runtime_state"):
    value = str(paths.get(key) or "")
    add(f"paths.{key}", bool(value and pathlib.Path(value).is_absolute()), value)
cmd = str(lib.get("cmd") or "")
wd = str(lib.get("working_dir") or "")
add("libreqos.cmd", bool(cmd), cmd)
add("libreqos.working_dir", bool(wd and pathlib.Path(wd).is_absolute()), wd)
print(json.dumps({"checks": checks, "paths": paths, "libreqos": lib, "rust_core": rc}, indent=2))
PY
  if ! python3 - "$TMP_JSON" <<'PY'
import json, sys
report = json.load(open(sys.argv[1]))
failed = [c for c in report["checks"] if not c["ok"]]
for c in report["checks"]:
    prefix = "ok" if c["ok"] else "fail"
    print(f"[rust-authority-preflight] {prefix}|{c['name']}|{c['detail']}")
raise SystemExit(1 if failed else 0)
PY
  then
    FAIL=$((FAIL+1))
  fi
fi

# Filesystem checks are intentionally conservative. A missing file may be a fresh
# install, but parent directories must exist and be writable for Rust authority.
if [ -f "$TMP_JSON" ]; then
  while IFS='|' read -r label path; do
    [ -n "$path" ] || continue
    parent="$(dirname "$path")"
    if [ -d "$parent" ]; then
      log "ok|parent-dir-$label|$parent"
      if [ -w "$parent" ]; then
        log "ok|parent-writable-$label|$parent"
      else
        warn "parent directory not writable by current user for $label: $parent"
      fi
    else
      fail_check "missing parent directory for $label: $parent"
    fi
  done < <(python3 - "$TMP_JSON" <<'PY'
import json, sys
r=json.load(open(sys.argv[1])); p=r.get('paths') or {}
for k in ('shaped_devices_csv','network_json','runtime_state'):
    if p.get(k): print(f"{k}|{p.get(k)}")
PY
)
fi

SELF_TEST_STATUS="not_run"
SELF_TEST_JSON="{}"
if [ -n "$CORE_BIN" ] && [ -x "$CORE_BIN" ]; then
  if SELF_TEST_JSON="$($CORE_BIN <<'JSON'
{"version":"1","op":"self-test","payload":{}}
JSON
)"; then
    if SELF_TEST_JSON="$SELF_TEST_JSON" python3 - <<'PY'
import json, os, sys
data=json.loads(os.environ.get('SELF_TEST_JSON') or '{}')
ops=set(((data.get('result') or {}).get('operations')) or [])
required={'execute-apply-transaction','build-apply-manifest','evaluate-sync-plan','append-transaction-journal','build-rollback-manifest'}
missing=sorted(required-ops)
ok=bool(data.get('ok')) and (data.get('result') or {}).get('status')=='ok' and not missing
print('ok' if ok else 'failed')
if missing:
    print('missing operations: '+', '.join(missing), file=sys.stderr)
raise SystemExit(0 if ok else 1)
PY
    then
      SELF_TEST_STATUS="ok"
      log "ok|rust-self-test|required operations present"
    else
      SELF_TEST_STATUS="failed"
      fail_check "Rust self-test missing required authority operations"
    fi
  else
    SELF_TEST_STATUS="failed"
    fail_check "lqosync-core self-test command failed"
  fi
fi

if command -v systemctl >/dev/null 2>&1; then
  if systemctl is-active --quiet lqosync-core 2>/dev/null; then
    log "ok|lqosync-core.service|active"
  else
    warn "lqosync-core.service is not active; daemon preference may fall back to CLI"
  fi
fi

STATUS="pass"
if [ "$FAIL" -ne 0 ]; then
  STATUS="fail"
fi

if [ "$WRITE_STAMP" = true ]; then
  mkdir -p "$(dirname "$STAMP_PATH")"
  GIT_HEAD="unknown"
  if [ -d "$INSTALL_DIR/.git" ]; then
    GIT_HEAD="$(cd "$INSTALL_DIR" && git rev-parse --short HEAD 2>/dev/null || echo unknown)"
  fi
  python3 - "$STAMP_PATH" "$STATUS" "$FAIL" "$WARN" "$TS_EPOCH" "$TS_ISO" "$CONFIG_PATH" "$CORE_BIN" "$SELF_TEST_STATUS" "$GIT_HEAD" <<'PY'
from __future__ import annotations
import json, os, pathlib, sys
stamp = pathlib.Path(sys.argv[1])
data = {
    "schema": "lqosync.rust_authority_preflight.v1",
    "status": sys.argv[2],
    "failures": int(sys.argv[3]),
    "warnings": int(sys.argv[4]),
    "created_epoch": int(sys.argv[5]),
    "created_at": sys.argv[6],
    "config_path": sys.argv[7],
    "core_bin": sys.argv[8],
    "self_test_status": sys.argv[9],
    "git_head": sys.argv[10],
    "operator": os.environ.get("SUDO_USER") or os.environ.get("USER") or "unknown",
}
stamp.write_text(json.dumps(data, indent=2) + "\n")
print(f"[rust-authority-preflight] wrote-stamp|{stamp}")
PY
fi

if [ "$FAIL" -ne 0 ]; then
  echo "FAIL: Rust full-authority preflight failed ($FAIL failures, $WARN warnings)" >&2
  exit 1
fi

echo "PASS: Rust full-authority preflight passed ($WARN warnings)"
