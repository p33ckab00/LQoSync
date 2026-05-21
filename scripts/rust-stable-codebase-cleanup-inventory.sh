#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
echo "== LQoSync stable Rust cleanup inventory =="
echo "runtime_role=flask_webui_shell_only"
echo "backend_authority=rust_full_authoritative"
echo

echo "== Python compatibility shell modules still imported =="
python3 - <<'PY'
import pathlib
root=pathlib.Path('.')
mods=[]
for p in root.rglob('*.py'):
    if '.git' in p.parts or '__pycache__' in p.parts:
        continue
    text=p.read_text(errors='ignore')
    if any(x in text for x in ['from applier', 'from builders', 'from collectors', 'from rules', 'from parsers', 'from engine']):
        mods.append(str(p))
for m in sorted(mods):
    print('in_use|' + m)
PY

echo

echo "== Legacy duplicate working-tree candidates on this host =="
for p in /home/pi/lqosync_docker /home/pi/lqosync /opt/lqosync /opt/lqos_docker; do
  if [ -e "$p" ]; then
    echo "candidate|$p|exists|archive with guarded stale-codebase cleanup only"
  else
    echo "candidate|$p|missing"
  fi
done

echo

echo "== Protected paths =="
for p in /opt/LQoSync /opt/libreqos /usr/local/bin/lqosync-core /etc/systemd/system/lqosync-core.service /run/lqosync-core.sock; do
  echo "protected|$p"
done
