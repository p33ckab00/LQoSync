# Setup & Repair Center

Setup & Repair is a diagnostics and repair guidance surface. It should not duplicate the full manual.

## Purpose

- inspect the current system
- show pass/fail/warn checks
- compute readiness score
- recommend the next action
- provide copy-ready SSH repair commands
- link to documentation sections for deeper explanations

## Not the purpose

Setup & Repair should not become a second About/manual page. Long installation, update, uninstall, and troubleshooting explanations belong in Documentation / About.

## Source of truth

Use `docs/content/*.md` and `docs/docs_manifest.json` as the documentation source. Setup & Repair should reference these sections by key.
