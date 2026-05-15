# Compact Information Architecture and Documentation Consolidation

LQoSync v2.61 reduces redundant operator surfaces while preserving all important data and compatibility routes.

## Purpose

The project had grown into many powerful pages that sometimes repeated similar information: health data in Dashboard and Health Trends, logs split between Services/Journals and Logs/Backups, reports repeating Dashboard cards, and long-form guidance duplicated between About, Setup & Repair, README, and docs files.

The v2.61 direction is:

- Dashboard is the single live operator status page.
- Config Center is the settings and policy home.
- Operations Center is the home for services, journals, LibreQoS apply history, app logs, audit events, and backups.
- Reports Center is for exports and report snapshots, not another Dashboard.
- Documentation Center is the searchable source of truth for guides.
- About is lightweight project/version/disclosure information.

## New compact sidebar model

```text
Main
├─ Dashboard
├─ Shaped Devices
├─ Network Layout
├─ Dry-run Preview
├─ Lifecycle

Settings
├─ Config Center
├─ Users

Operations
├─ Operations Center
├─ Reports
├─ Updates

Help
├─ Documentation
├─ Setup Wizard
├─ Setup / Repair
├─ About
```

## Operations Center

Operations Center consolidates:

- Service Status
- Restart Groups
- Journal Viewer
- LibreQoS Apply History
- Last Cycle Timeline
- App Logs
- Audit Events
- Backups

Compatibility routes remain:

```text
/services → /operations?tab=services or journals
/logs → /operations?tab=logs
/health → Dashboard health section
```

## Reports Center

Reports Center is intentionally compact. It provides export bundles and snapshot previews. Use Dashboard for live status and Operations Center for live logs.

## Documentation model

Documentation is consolidated so GitHub and WebUI use the same content model:

- `docs/content/*.md` is the reusable source content.
- `docs/docs_manifest.json` is the documentation index and ordering map.
- `FULL_DOCUMENTATION.md` is the consolidated long-form manual for GitHub/offline reading.
- `README.md` stays compact and links to the full docs.
- WebUI Documentation Center searches local docs content.
- Setup & Repair should diagnose and link to docs instead of repeating full manuals.

## Operator guidance

Use these pages by intent:

- Need live status? Open Dashboard.
- Need logs/services/backups? Open Operations Center.
- Need to change behavior? Open Config Center.
- Need to preview impact? Run Dry Run.
- Need audit/export? Open Reports.
- Need help? Open Documentation Center.
