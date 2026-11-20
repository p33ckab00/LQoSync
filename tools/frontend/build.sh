#!/usr/bin/env bash
# Build the web services and the Rust browser application.
set -euo pipefail
cd "$(dirname "$0")/../.."
export BINARYEN_CORES="${BINARYEN_CORES:-2}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_PROFILE_RELEASE_LTO=thin
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=4
cargo build --locked --release -p web-gateway --bins
(cd crates/web-ui && "${TRUNK:-trunk}" build --release --locked)
python3 tools/frontend/prepare-dist.py
