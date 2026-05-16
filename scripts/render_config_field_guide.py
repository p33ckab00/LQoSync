#!/usr/bin/env python3
"""Render docs/content/config_field_guide.md from the shared config guide registry."""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from engine.config_guide import render_config_field_guide_markdown  # noqa: E402


def main() -> int:
    target = ROOT / "docs" / "content" / "config_field_guide.md"
    target.write_text(render_config_field_guide_markdown(), encoding="utf-8")
    print(target)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
