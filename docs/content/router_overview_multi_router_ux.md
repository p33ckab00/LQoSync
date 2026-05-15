# Router Overview + Multi-Router UX Polish

LQoSync v2.69 adds a compact, read-only Router Overview page for operators managing one or more MikroTik routers.

## Purpose

The Router Overview page answers:

- Which MikroTik routers are configured?
- Which routers are enabled or disabled?
- Which sources are enabled per router: PPPoE, DHCP, or Hotspot?
- How many generated ShapedDevices rows appear to belong to each router?
- Which routers have collector/source warnings from the last run?
- Which page should the operator open next: Config Center, Dry Run, or Operations Center?

## Page location

```text
/routers
```

Read-only API:

```text
/api/routers/overview
```

## What it does not do

Router Overview does not contact MikroTik directly, write files, change config, run the scheduler, or apply LibreQoS. It summarizes existing config and last known runtime data.

## Recommended workflow

1. Open Router Overview.
2. Check router status cards.
3. If a router has warnings, open Dry Run or Operations Center.
4. If a source is disabled or missing, open Config Center.
5. After changes, run Dry Run before enabling scheduler or auto-apply.

## Multi-router clarity

Routers can be root-level or child routers when `parent_node` is configured for deep/custom hierarchy. Router Overview shows this role so operators can quickly verify whether a router is acting as a root or nested router in the topology.
