#!/usr/bin/env bash
set -euo pipefail

# One-command safe Rust-authoritative install wrapper.
# It installs LQoSync conservatively, requires Rust core build/self-test, installs
# the Rust daemon, then promotes config to Rust authoritative gate mode.
# It does not start/restart the main lqosync service unless explicitly requested.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVICE_START_POLICY="${LQOSYNC_SERVICE_START_POLICY:-enable_only}"
RESTART_AFTER_PROMOTION="${RESTART_AFTER_PROMOTION:-false}"
STRICT_RUST="${STRICT_RUST:-true}"

if [ "${EUID:-$(id -u)}" -ne 0 ]; then
  echo "Run as root: sudo bash install-rust-authoritative-safe.sh" >&2
  exit 1
fi

(
  cd "$SCRIPT_DIR"
  INSTALL_RUST_CORE=true \
  INSTALL_RUST_CORE_DAEMON=true \
  STRICT_RUST="$STRICT_RUST" \
  LQOSYNC_SERVICE_START_POLICY="$SERVICE_START_POLICY" \
  bash install-production-safe.sh
)

(
  cd "$SCRIPT_DIR"
  RESTART_SERVICE="$RESTART_AFTER_PROMOTION" \
  bash scripts/promote-rust-authoritative-safe.sh
)

echo "[LQoSync Rust Authority] Complete. Main service start policy: $SERVICE_START_POLICY"
echo "[LQoSync Rust Authority] Recommended next step: run Dry Run, review diff, then start/restart service manually."
