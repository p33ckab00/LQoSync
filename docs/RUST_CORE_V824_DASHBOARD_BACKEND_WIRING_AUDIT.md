# v8.2.4 Dashboard Backend Wiring Audit

This release adds a read-only Dashboard backend wiring audit for the local LQoSync appliance journey.

## Purpose

Operators need to know whether Dashboard cards are backed by real backend providers or are only waiting for a first dry-run/sync cycle. The Dashboard now exposes this in two places:

- Dashboard card: **Dashboard Backend Wiring**
- JSON endpoint: `/api/dashboard/modules`

## Boundary

This audit is read-only. It does not run MikroTik collectors, write `ShapedDevices.csv`, write `network.json`, run LibreQoS, or mutate scheduler state.

## Canonical architecture

- Rust daemon: backend authority, scheduler authority, apply authority, journal, watchdog, readiness gates.
- Python Flask: WebUI shell only.
- Dashboard modules: operator views connected to backend state/API helpers.

## Module wiring checked

- Operator health summary
- Production readiness
- Source health and performance
- Rust scheduler authority
- Rust backend authority
- Flask WebUI shell
- Client change summary
- Policy decision
- Smart insights
- Smart lifecycle
- Generated files and drift policy
- Services snapshot
- Git status
- Setup Wizard banner

`idle` means wired but no dry-run/sync output exists yet.
`warn` means the backend provider is reachable but a required runtime condition is missing, such as an inactive service or missing file.
`fail` means the module has config/schema errors that require action.
