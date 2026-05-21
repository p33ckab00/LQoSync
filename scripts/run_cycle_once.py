#!/usr/bin/env python3
"""Run one LQoSync cycle for Rust scheduler authority.

This script is intentionally small. It gives the Rust scheduler daemon a stable
command target while Rust remains the authority for scheduler decision, lock,
preflight/watchdog, transaction journal, file writes, and LibreQoS apply.
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from engine.run_cycle import run_cycle  # noqa: E402


def main() -> int:
    mode = sys.argv[1] if len(sys.argv) > 1 else "scheduled"
    config_path = os.environ.get("CONFIG_PATH") or "/opt/libreqos/src/config.json"
    result = run_cycle(mode=mode, config_path=config_path)
    print(json.dumps(result.to_dict(), ensure_ascii=False))
    return 0 if not result.errors else 2


if __name__ == "__main__":
    raise SystemExit(main())
