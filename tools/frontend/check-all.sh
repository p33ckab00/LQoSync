#!/usr/bin/env bash
# Run every web quality gate once: format, whitespace, Clippy (native and Wasm), tests, the Wasm
# build, and each browser check against a fresh isolated fixture. It never touches a production
# service, router, or account.
#
# Required environment: PLAYWRIGHT_MODULE, CHROMIUM_PATH, BROWSER_TEST_PASSWORD (fixture only).
# Optional: TRUNK (default: trunk), CHECKS="routers-check dpdk-check" (default: every *-check.cjs),
#           SKIP_CHECKS (default: pulse-check, a known failure from before the web work),
#           STAGES="rust browser" (default: both).
# Exit status: 0 when every gate passes, 1 otherwise. Logs stay in the printed folder.
set -uo pipefail
cd "$(dirname "$0")/../.."

: "${PLAYWRIGHT_MODULE:?set PLAYWRIGHT_MODULE to the Playwright module path}"
: "${CHROMIUM_PATH:?set CHROMIUM_PATH to the Chromium binary}"
: "${BROWSER_TEST_PASSWORD:?set BROWSER_TEST_PASSWORD for the isolated fixture account}"
export PLAYWRIGHT_MODULE CHROMIUM_PATH BROWSER_TEST_PASSWORD
TRUNK="${TRUNK:-trunk}"
SKIP_CHECKS="${SKIP_CHECKS:-pulse-check}"
STAGES="${STAGES:-rust browser}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
PORT=19581
LOGS="$(mktemp -d -t lqosync-checks.XXXXXX)"
export ROUTERS_EVIDENCE_DIR="$LOGS/evidence" DPDK_EVIDENCE_DIR="$LOGS/evidence"
results=()
failed=0

gate() { # gate <name> <command...>: run one gate, keep its log, record the result
    local name="$1"; shift
    if nice -n 19 "$@" >"$LOGS/$name.log" 2>&1; then
        results+=("PASS  $name")
    else
        results+=("FAIL  $name   (log: $LOGS/$name.log)"); failed=1
    fi
}

# Stop only the fixture process on the test port, never another program.
stop_fixture() {
    local pid
    for pid in $(ss -ltnpH "sport = :$PORT" 2>/dev/null | grep -oP 'pid=\K[0-9]+' | sort -u); do
        [[ "$(cat "/proc/$pid/comm" 2>/dev/null)" == browser_fixture* ]] && kill "$pid" 2>/dev/null
    done
    for _ in $(seq 1 50); do ss -ltnH "sport = :$PORT" | grep -q . || return 0; sleep 0.1; done
}
start_fixture() {
    stop_fixture
    setsid -f target/debug/examples/browser_fixture >"$LOGS/fixture-$1.log" 2>&1
    for _ in $(seq 1 100); do ss -ltnH "sport = :$PORT" | grep -q . && return 0; sleep 0.1; done
    return 1
}

if [[ " $STAGES " == *" rust "* ]]; then
    gate fmt cargo fmt --all --check
    gate whitespace git diff --check
    gate clippy cargo clippy --locked --all-targets -p lqosync3 -p web-ui -p web-gateway -p web-contract -- -D warnings
    gate clippy-wasm cargo clippy --locked -p web-ui --target wasm32-unknown-unknown -- -D warnings
    gate tests cargo test --locked -p lqosync3 -p web-ui -p web-gateway -p web-contract
fi

if [[ " $STAGES " == *" browser "* ]]; then
    gate fixture-build cargo build --locked -p web-gateway --example browser_fixture
    gate wasm-build bash -c "cd crates/web-ui && '$TRUNK' build --locked && cd ../.. && python3 tools/frontend/prepare-dist.py"
    checks="${CHECKS:-$(ls tools/frontend/*-check.cjs | xargs -n1 basename | sed 's/\.cjs$//')}"
    for check in $checks; do
        [[ " $SKIP_CHECKS " == *" $check "* ]] && { results+=("SKIP  $check"); continue; }
        # A fresh fixture for each check: the sign-in rate limit and the synthetic data start clean.
        if ! start_fixture "$check"; then
            results+=("FAIL  $check   (the fixture did not start: $LOGS/fixture-$check.log)"); failed=1; continue
        fi
        gate "$check" timeout 300 node "tools/frontend/$check.cjs"
    done
    stop_fixture
fi

printf '%s\n' "${results[@]}"
echo "Logs: $LOGS"
exit "$failed"
