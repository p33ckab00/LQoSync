#!/bin/bash
# Dev only. Compile one ShapedDevices.csv + network.json pair with the LibreQoS v3
# Python binding, offline. All socket connections are blocked with strace.
# Use: LQOSYNC3_V3_BINDING=<dir> tools/v3sim/check.sh <pair-dir>
# <dir> must hold liblqos_python.so and lqos.conf (a v3 config to use as a template).
set -euo pipefail
PAIR=$(realpath "$1")
if [ -z "${LQOSYNC3_V3_BINDING:-}" ]; then echo "skip: no v3 binding"; exit 0; fi
BIND=$(realpath "$LQOSYNC3_V3_BINDING")
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
RUN="$ROOT/target/v3sim/$(basename "$PAIR")-$(date +%s%N)"   # a new folder for each run; nothing is deleted
mkdir -p "$RUN/src" "$RUN/state"
cp "$PAIR/ShapedDevices.csv" "$PAIR/network.json" "$RUN/src/"
sed -E "s#^lqos_directory = .*#lqos_directory = \"$RUN/src\"#; s#^state_directory = .*#state_directory = \"$RUN/state\"#; s#^source = .*#source = \"shaped_devices\"#" \
  "$BIND/lqos.conf" > "$RUN/lqos.conf"
cat > "$RUN/harness.py" <<'PY'
import os, sys
sys.path.insert(0, os.environ["SIM_MOD"])
import liblqos_python as m
try:
    print("RESULT:", m.write_compiled_topology_from_legacy_artifacts("shaped_devices", "full"))
except Exception as e:
    print("ERROR:", type(e).__name__, e)
    sys.exit(1)
PY
env -i PATH=/usr/bin:/bin LQOS_CONFIG="$RUN/lqos.conf" SIM_MOD="$BIND" timeout 60 \
  strace -f -qq -o "$RUN/strace.log" -e trace=connect,openat,rename,renameat2,unlink,unlinkat,mkdir \
  -e inject=connect:error=EACCES python3 -I "$RUN/harness.py"
# Fail if the binding wrote outside the run folder.
if grep -E 'O_WRONLY|O_RDWR|O_CREAT|rename|unlink|mkdir' "$RUN/strace.log" | grep -v -E "$RUN|/dev/null|/proc/|ENOENT" | grep -q .; then
  echo "FAIL: write outside $RUN"; exit 1
fi
python3 -I - "$RUN/state/topology/topology_canonical_state.json" <<'PY'
import json, sys
nodes = json.load(open(sys.argv[1]))["nodes"]
bad = [n["node_name"] for n in nodes if not n["node_id"].startswith("lq3-")]
print(f"nodes: {len(nodes)}, without lq3- id: {bad}")
sys.exit(1 if bad else 0)
PY
