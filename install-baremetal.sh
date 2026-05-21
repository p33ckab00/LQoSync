#!/usr/bin/env bash
# Convenience wrapper for bare-metal Ubuntu/Debian installation.
# Historical behavior follows install.sh. For live production, prefer:
#   sudo bash install-production-safe.sh
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec bash "$SCRIPT_DIR/install.sh" "$@"
