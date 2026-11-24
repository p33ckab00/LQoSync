#!/usr/bin/env bash
# Topology Studio service smoke test: HTTP -> broker -> core service API -> fake RouterOS.
# TEST EVIDENCE ONLY. It uses a synthetic router and a test runtime. It is not production acceptance.
#
# Harness (owner decision, task B5): the real broker and HTTP code (examples/studio_gateway)
# and the real Runner and service API (examples/studio_core), in their own processes. The core
# reads secret files owned by the current user, not root. No router-edit, sync-now, or signal
# reaches another process. Every file is under one temporary root. It never reads a production
# config and never contacts a real router: the only router is 127.0.0.1.
#
# Usage: bash tools/integration/studio-service-smoke.sh
# Environment: SMOKE_ROUTER_PORT (default 19601), SMOKE_WEB_PORT (default 19602), KEEP=1 keeps the root.
set -euo pipefail

REPO=$(cd "$(dirname "$0")/../.." && pwd)
ROOT=$(mktemp -d "${TMPDIR:-/tmp}/lqosync3-studio-smoke.XXXXXX")
ROOT=$(realpath "$ROOT")
RPORT=${SMOKE_ROUTER_PORT:-19601}
WPORT=${SMOKE_WEB_PORT:-19602}
B="http://127.0.0.1:$WPORT"
PIDS=()
FAILED=0
EVIDENCE="$ROOT/evidence.txt"

inside() { # refuse any path outside the temporary root
  case "$(realpath -m "$1")" in "$ROOT"/*) ;; *) echo "refuse: $1 is outside $ROOT" >&2; exit 1 ;; esac
}
cleanup() {
  for p in "${PIDS[@]}"; do kill "$p" 2>/dev/null || true; done
  wait 2>/dev/null || true
  if [ "$FAILED" = 0 ] && [ "${KEEP:-0}" != 1 ]; then rm -rf "$ROOT"; else echo "evidence kept in $ROOT"; fi
}
trap cleanup EXIT
note() { echo "$*" | tee -a "$EVIDENCE"; }
check() { # check <name> <condition>
  if eval "$2"; then note "PASS $1"; else note "FAIL $1"; note "  last answer: $(head -c 400 "$ROOT/body" 2>/dev/null)"; FAILED=1; fi
}
json() { python3 -c "import json,sys; d=json.load(open(sys.argv[1])); print(eval(sys.argv[2]))" "$ROOT/body" "$1"; }

for port in "$RPORT" "$WPORT"; do
  if ss -ltn | grep -q ":$port "; then echo "port $port is busy" >&2; exit 1; fi
done
note "TEST EVIDENCE — synthetic router and test runtime; not production acceptance"
note "root $ROOT, repo $(git -C "$REPO" rev-parse --short HEAD), $(date -Is)"

cd "$REPO"
cargo build --locked -q -j2 --example studio_core --example fake_routeros
cargo build --locked -q -j2 -p web-gateway --example studio_gateway
BIN="$REPO/target/debug/examples"

STATE="$ROOT/state"; LQOS="$ROOT/lqos"; CONFIG="$ROOT/lqosync3.toml"; SECRET="$ROOT/router.password"
for p in "$STATE" "$LQOS" "$CONFIG" "$SECRET" "$ROOT/dpkg-status"; do inside "$p"; done
mkdir -p "$STATE" "$LQOS/state"
ROUTER_PASSWORD="smoke-$(od -An -N8 -tx1 /dev/urandom | tr -d ' \n')"
WEB_PASSWORD="smoke-web-$(od -An -N8 -tx1 /dev/urandom | tr -d ' \n')"
umask 077
printf '%s\n' "$ROUTER_PASSWORD" > "$SECRET"
printf 'Package: libreqos\nStatus: install ok installed\nVersion: 2.2.1\n' > "$ROOT/dpkg-status"
cat > "$CONFIG" <<EOF
[runtime]
state_dir = "$STATE"
mode = "shadow"
[libreqos]
target = "v2_pair"
directory = "$LQOS"
state_directory = "$LQOS/state"
dpkg_status = "$ROOT/dpkg-status"
[wan_monitor]
enabled = false
[[routers]]
name = "Smoke-01"
address = "127.0.0.1"
port = $RPORT
tls = false
tls_verify = false
username = "smoke"
password_file = "$SECRET"
[routers.pppoe]
enabled = true
per_plan_node = true
EOF

"$BIN/fake_routeros" "$RPORT" "$ROUTER_PASSWORD" > "$ROOT/router.log" 2>&1 & PIDS+=($!)
SMOKE_ROOT="$ROOT" "$BIN/studio_core" "$CONFIG" > "$ROOT/core.log" 2>&1 & PIDS+=($!)
SMOKE_ROOT="$ROOT" SMOKE_STATE="$STATE" SMOKE_PORT="$WPORT" SMOKE_PASSWORD="$WEB_PASSWORD" \
  "$BIN/studio_gateway" > "$ROOT/gateway.log" 2>&1 & PIDS+=($!)
for _ in $(seq 1 120); do
  [ -S "$STATE/api.sock" ] && [ -f "$STATE/source-studio.json" ] && curl -s -o /dev/null "$B/api/v1/session" && break
  sleep 0.5
done
check "core and gateway started" '[ -f "$STATE/source-studio.json" ]'

J="$ROOT/cookies"; O=(-H "Origin: $B" -H 'Content-Type: application/json')
req() { curl -s -o "$ROOT/body" -w '%{http_code}' -b "$J" -c "$J" "$@"; }
post() { req "${O[@]}" -H "x-csrf-token: $CSRF" -d "$2" "$B$1"; }
op() { # op <key>: poll the operation until it ends
  for _ in $(seq 1 120); do
    req "$B/api/v1/topology/operations/$1" > /dev/null
    case "$(json 'd["state"]')" in queued|running|reviewing) sleep 0.5 ;; *) return ;; esac
  done
}
tree_hash() { (cd "$STATE" && find topology-manager -type f -print0 2>/dev/null | sort -z | xargs -0 sha256sum 2>/dev/null | sha256sum); }

req "${O[@]}" -d "{\"username\":\"smoke-operator\",\"password\":\"$WEB_PASSWORD\"}" "$B/api/v1/session" > /dev/null
CSRF=$(json 'd["csrf"]')
req "$B/api/v1/topology/source-capabilities" > "$ROOT/status"
V=$(json '"policy_save" in d["features"]')
check "capabilities advertise policy_save" '[ "$(cat "$ROOT/status")" = 200 ] && [ "$V" = True ]'
req "$B/api/v1/routers" > /dev/null
RID=$(json 'd["routers"][0]["id"]')

DRAFT="{\"key\":\"smoke-draft-0000000001\",\"job\":{\"kind\":\"draft_test\",\"required_sources\":[\"ppp\"],\"connection\":{\"address\":\"127.0.0.1\",\"port\":$RPORT,\"tls\":false,\"tls_verify\":false,\"username\":\"smoke\",\"password\":\"$ROUTER_PASSWORD\",\"ca_pem\":null}}}"
check "draft test accepted" '[ "$(post /api/v1/topology/source-jobs "$DRAFT")" = 202 ]'
op smoke-draft-0000000001
V=$(json '",".join(c["state"] for c in d["source_result"]["checks"])')
check "draft test passes every stage that runs (TLS off)" '[ "$V" = passed,not_run,passed,passed,passed ]'
check "draft secret absent from the operation" '! grep -q "$ROUTER_PASSWORD" "$ROOT/body"'

req "$B/api/v1/routers/$RID/source-config" > "$ROOT/status"
check "source config is served from the cache" '[ "$(cat "$ROOT/status")" = 200 ]'
DIGEST=$(json 'd["loaded_config_digest"]')

check "topology initialize accepted" '[ "$(post /api/v1/topology/edits "{\"key\":\"smoke-init-00000000001\",\"expected_revision\":0,\"edit\":{\"kind\":\"topology.initialize\"}}")" = 202 ]'
op smoke-init-00000000001
V=$(json 'd["state"]')
check "topology initialized" '[ "$V" = saved ]'

post /api/v1/topology/source-jobs "{\"key\":\"smoke-collect-000000001\",\"job\":{\"kind\":\"collect\",\"router_id\":\"$RID\",\"expected_config_digest\":\"$DIGEST\",\"sources\":[\"ppp\",\"wan\"]}}" > /dev/null
op smoke-collect-000000001
SNAP=$(json 'd["source_result"]["id"]')
V=$(json '",".join(t["state"] for t in d["source_result"]["tables"] if t["table"].startswith("ppp"))')
check "collection gives complete PPP tables" '[ "$V" = complete,complete,complete ]'

RATES='[{"kind":"subscriber_override"},{"kind":"account_comment","format":"legacy_configured"},{"kind":"active_comment","format":"legacy_configured"},{"kind":"profile_comment","format":"legacy_configured"},{"kind":"profile_rate_limit"},{"kind":"profile_name"},{"kind":"settings"}]'
POLICY="{\"id\":null,\"name\":\"PPPoE\",\"enabled\":true,\"priority\":10,\"source\":\"ppp\",\"read_access_ref\":\"default\",\"members\":\"joined\",\"filter\":{\"kind\":\"all\",\"rules\":[]},\"group\":{\"kind\":\"by_field\",\"field\":\"account.profile\"},\"identity\":\"registry\",\"rates\":$RATES,\"parent\":{\"kind\":\"router\"},\"capacity\":{\"kind\":\"from_plans\",\"down_factor\":\"1\",\"up_factor\":\"1\",\"minimum_down_bps\":\"0\",\"minimum_up_bps\":\"0\"},\"writes\":[]}"

BEFORE=$(tree_hash)
post /api/v1/topology/source-jobs "{\"key\":\"smoke-evaluate-00000001\",\"job\":{\"kind\":\"evaluate\",\"router_id\":\"$RID\",\"expected_revision\":\"1\",\"expected_config_digest\":\"$DIGEST\",\"snapshot_id\":\"$SNAP\",\"policies\":[$POLICY]}}" > /dev/null
op smoke-evaluate-00000001
CAND=$(json 'd["source_result"]["id"]')
req "$B/api/v1/topology/source-candidates/$CAND" > "$ROOT/status"
V=$(json '"%s %s" % (d.get("complete"), d.get("publishable"))')
check "candidate summary is complete and publishable" '[ "$(cat "$ROOT/status")" = 200 ] && [ "$V" = "True True" ]'
req "$B/api/v1/topology/source-candidates/$CAND/records?limit=10" > /dev/null
V=$(json '",".join("%s=%s" % (r["record_ref"], r["down_bps"]) for r in d["items"])')
check "candidate has the active PPP record at the profile rate" '[ "$V" = ppp-name/v1/alice=15000000 ]'
check "evaluation changes no durable topology state" '[ "$BEFORE" = "$(tree_hash)" ]'

post /api/v1/topology/edits "{\"key\":\"smoke-save-00000000001\",\"expected_revision\":1,\"edit\":{\"kind\":\"router.sources_set\",\"router_id\":\"$RID\",\"expected_config_digest\":\"$DIGEST\",\"policies\":[$POLICY]}}" > /dev/null
op smoke-save-00000000001
V=$(json '"%s %s" % (d["state"], (d.get("receipt") or {}).get("revision"))')
check "policy save gives a saved receipt" '[ "$V" = "saved 2" ]'
check "the registry holds the policy set" 'grep -q "source_policies" "$STATE/topology-manager/registry.json"'

post /api/v1/topology/reads '{"key":"smoke-refresh-000000001","job":"refresh"}' > /dev/null
op smoke-refresh-000000001
req "$B/api/v1/topology" > /dev/null
V=$(json '"%s %s" % ((d.get("observed") or {}).get("state"), (d.get("observed") or {}).get("revision"))')
check "the current engine prepares the saved policy" '[ "$V" = "ok 2" ]'

post /api/v1/topology/edits "{\"key\":\"smoke-wan-000000000001\",\"expected_revision\":2,\"edit\":{\"kind\":\"wan.candidate_set\",\"router_id\":\"$RID\",\"expected_config_digest\":\"$DIGEST\",\"interface\":\"pppoe-out1\",\"decision\":\"pin\"}}" > /dev/null
op smoke-wan-000000000001
V=$(json 'd["state"]')
check "WAN decision saved" '[ "$V" = saved ]'
post /api/v1/topology/source-jobs "{\"key\":\"smoke-collect-000000002\",\"job\":{\"kind\":\"collect\",\"router_id\":\"$RID\",\"expected_config_digest\":\"$DIGEST\",\"sources\":[\"wan\"]}}" > /dev/null
op smoke-collect-000000002
req "$B/api/v1/routers/$RID/source-config" > /dev/null
V=$(json '",".join("%s=%s" % (w["interface"], w["decision"]) for w in d["wan_candidates"] if w["decision"])')
check "WAN evidence shows the saved decision" '[ "$V" = pppoe-out1=pin ]'

check "the fake router saw no write or other command" '! grep -q "^REFUSED" "$ROOT/router.log"'
check "no password in any log" '! grep -rq -e "$ROUTER_PASSWORD" -e "$WEB_PASSWORD" "$ROOT"/*.log'
note "not covered: publication by the topology writer (activation needs live-test gates); production binaries as root"
[ "$FAILED" = 0 ] && note "RESULT PASS" || note "RESULT FAIL"
cat "$EVIDENCE" > "$REPO/target/studio-service-smoke.txt" 2>/dev/null || true
exit "$FAILED"
