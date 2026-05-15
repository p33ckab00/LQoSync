# Router Insight + Multi-Router UX

LQoSync originally introduced a standalone `/routers` Router Overview page in v2.69. In v2.69.1 this was de-duplicated: router insight now lives inside `Config Center → Routers`, where router settings already exist.

## Current behavior

- `/config?tab=routers` is the main Router Insight and router settings page.
- `/routers` is kept only as a compatibility alias and redirects to `Config Center → Routers`.
- `/api/routers/overview` remains as the read-only JSON API.

## Purpose

Router Insight answers:

- which MikroTik routers are configured
- which routers are enabled or disabled
- which PPPoE/DHCP/Hotspot sources are enabled per router
- how many generated rows appear to belong to each router
- which routers have collector/source warnings from the last run

## Non-goals

Router Insight does not contact MikroTik directly, write files, change config, run the scheduler, or apply LibreQoS. It summarizes existing config and last known runtime data.
