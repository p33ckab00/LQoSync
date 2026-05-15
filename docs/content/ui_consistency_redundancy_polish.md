# UI Consistency and Redundancy Polish

LQoSync v2.64 is a UI/UX consolidation release. It does not change MikroTik collectors, policy decisions, generated files, scheduler behavior, or LibreQoS apply behavior.

## Purpose

The project now follows a compact operator model:

- Dashboard is the live status cockpit.
- Config Center is the settings, policy, and notification home.
- Operations Center owns services, journals, apply logs, app logs, audit events, and backups.
- Reports Center is for exports and snapshots.
- Lifecycle is for per-client investigation.
- Documentation Center is the manual and search surface.

## What changed

- Added reusable UI helpers for page maps, compact chips, table toolbars, pagination, mobile table cards, icon buttons, and empty states.
- Added dashboard section shortcuts so operators can jump to Health, Sources, Timeline, Policy, Operations, and Reports.
- Standardized Operations Center Apply History and Audit Events with pagination and row-limit controls.
- Improved mobile table behavior for audit events.
- Kept compatibility routes and existing workflows intact.

## Operator guidance

Use Dashboard for a quick answer to “is the system healthy?”

Use Operations Center when you need evidence: services, journals, logs, audit trail, apply history, and backups.

Use Reports Center when you need an exportable snapshot for audit or support.

Use Documentation Center instead of duplicated page-level manuals.
