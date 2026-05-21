#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
fail=0
check_file() { [ -f "$1" ] || { echo "MISSING: $1" >&2; fail=1; }; }
check_contains() { local f="$1" p="$2" label="$3"; check_file "$f"; if [ -f "$f" ] && grep -q "$p" "$f"; then echo "ok|$label|$f"; else echo "MISSING[$label]: $f lacks $p" >&2; fail=1; fi; }
for f in scripts/rust-authority-journal-audit.sh scripts/rust-authority-rollback-drill.sh scripts/rust-set-and-forget-readiness.sh scripts/verify-rust-set-and-forget-candidate.sh docs/RUST_CORE_V780_SET_AND_FORGET_CANDIDATE.md; do check_file "$f"; done
check_contains VERSION '2.151.0' version
check_contains config.json.example "rust_set_and_forget_candidate_enabled" config-flag
check_contains engine/config_loader.py "rust_set_and_forget_candidate_enabled" loader-default
check_contains engine/run_cycle.py "rust_set_and_forget_gate_failed" runtime-gate
check_contains scripts/promote-rust-full-authoritative-safe.sh "rust-set-and-forget-readiness.sh" promotion-readiness
check_contains docs/docs_manifest.json "scheduler_authority" docs-manifest
if [ "$fail" -ne 0 ]; then echo "FAIL: Rust set-and-forget candidate verification failed" >&2; exit 1; fi
echo "PASS: Rust set-and-forget candidate wiring verified"
