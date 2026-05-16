# Policy Tree Desktop Alignment Hotfix

v2.70.14-rc1 fixes a desktop-only Policy Center presentation regression.

## What changed

- Restores horizontal icon + label alignment in the desktop Policy Center tree.
- Keeps the existing mobile Policy Center card layout unchanged.
- Scopes the Field Guide stacked text styling away from the normal Policy Center navigation tree.
- Extends UI Wiring Audit so the desktop tree alignment rule cannot silently disappear again.

## Safety

This is a UI-only fix. It does not change policy values, config writes, routes, mobile behavior, generated files, scheduler timing, or LibreQoS apply mechanics.
