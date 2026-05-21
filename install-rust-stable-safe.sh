#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

echo "[LQoSync Stable] Installing Rust backend stable release..."
export LQOSYNC_INIT_POLICY="${LQOSYNC_INIT_POLICY:-preserve_existing}"
export LQOSYNC_SERVICE_START_POLICY="${LQOSYNC_SERVICE_START_POLICY:-enable_only}"
export INSTALL_RUST_CORE="${INSTALL_RUST_CORE:-true}"
export INSTALL_RUST_CORE_DAEMON="${INSTALL_RUST_CORE_DAEMON:-true}"

if [ -x ./install-rust-full-authoritative-safe.sh ]; then
  bash ./install-rust-full-authoritative-safe.sh
else
  bash ./install-production-safe.sh
fi

if [ -x ./scripts/promote-rust-full-authoritative-safe.sh ]; then
  bash ./scripts/promote-rust-full-authoritative-safe.sh
fi

bash ./scripts/verify-rust-stable-release-cleanup.sh
python3 ./scripts/release_check.py
python3 ./scripts/regression_check.py
python3 ./scripts/stable_release_check.py

echo "[LQoSync Stable] Complete. Start after review: sudo systemctl start lqosync"
