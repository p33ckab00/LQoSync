#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
fail=0
check_file() { [ -f "$1" ] || { echo "MISSING: $1" >&2; fail=1; }; }
check_contains() {
  local file="$1" pattern="$2" label="$3"
  check_file "$file"
  if [ -f "$file" ] && grep -q "$pattern" "$file"; then
    echo "ok|$label|$file"
  else
    echo "MISSING[$label]: $file lacks $pattern" >&2
    fail=1
  fi
}
check_file docs/OPERATOR_TROUBLESHOOTING.md
check_file docs/RUST_CORE_V821_OPERATOR_TROUBLESHOOTING_DOCS.md
check_contains docs/OPERATOR_TROUBLESHOOTING.md "Rust core requested but cargo is not installed" "missing-cargo-error"
check_contains docs/OPERATOR_TROUBLESHOOTING.md "lock file version 4" "cargo-lock-v4-error"
check_contains docs/OPERATOR_TROUBLESHOOTING.md "git push rejected" "git-fetch-first-error"
check_contains docs/OPERATOR_TROUBLESHOOTING.md "interactive rebase in progress" "rebase-conflict-error"
check_contains docs/OPERATOR_TROUBLESHOOTING.md "LQOSYNC_SERVICE_START_POLICY=enable_only" "enable-only-policy"
check_contains docs/OPERATOR_TROUBLESHOOTING.md "Python Flask = WebUI shell only" "canonical-boundary"
check_contains docs/DOCUMENTATION_INDEX.md "OPERATOR_TROUBLESHOOTING" "docs-index-link"
check_contains docs/docs_manifest.json "operator.troubleshooting" "docs-manifest-entry"
python3 - <<'PY'
import json
json.load(open('docs/docs_manifest.json'))
print('ok|json|docs/docs_manifest.json')
PY
if [ "$fail" -ne 0 ]; then
  echo "FAIL: operator troubleshooting documentation alignment failed" >&2
  exit 1
fi
echo "PASS: operator troubleshooting documentation alignment verified"
