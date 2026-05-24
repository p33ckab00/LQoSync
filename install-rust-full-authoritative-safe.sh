#!/usr/bin/env bash
set -euo pipefail

# One-command Rust full apply authority installer.
# Installs production-safe, builds/installs Rust core + daemon, verifies self-test,
# then promotes config so Rust owns validation, sync-plan gating, atomic file
# writes, transaction journal, and LibreQoS.py external apply execution.
# It retires the Python backend service and keeps lqosync-core as the only
# backend runtime unless a legacy override is explicitly requested.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVICE_START_POLICY="${LQOSYNC_SERVICE_START_POLICY:-enable_only}"
RESTART_AFTER_PROMOTION="${RESTART_AFTER_PROMOTION:-false}"
STRICT_RUST="${STRICT_RUST:-true}"

if [ "${EUID:-$(id -u)}" -ne 0 ]; then
  echo "Run as root: sudo bash install-rust-full-authoritative-safe.sh" >&2
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
  bash scripts/promote-rust-full-authoritative-safe.sh
)

echo "[LQoSync Rust Full Authority] Complete. Rust backend service policy: $SERVICE_START_POLICY"
echo "[LQoSync Rust Full Authority] Recommended next step: run verification checks, then start/restart lqosync-core manually if needed."
