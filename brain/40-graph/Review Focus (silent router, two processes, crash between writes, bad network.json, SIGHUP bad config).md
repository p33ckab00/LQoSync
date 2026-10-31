---
source_file: "docs/superpowers/plans/2026-10-31-lqosync3-runtime.md"
type: "concept"
community: "Router Reads 2"
location: "L23-29"
tags:
  - graphify/concept
  - graphify/EXTRACTED
  - community/Router_Reads_2
---

# Review Focus (silent router, two processes, crash between writes, bad network.json, SIGHUP bad config)

## Connections
- [[Deferred minor items from Plan 2 review]] - `conceptually_related_to` [INFERRED]
- [[Mock RouterOS server tests (testsmock_router.rs)]] - `references` [EXTRACTED]
- [[Write order state.json first, then LibreQoS pair]] - `references` [EXTRACTED]
- [[fileslock (try_lock, 'another lqosync3 is running')]] - `references` [EXTRACTED]
- [[lqosync3 Runtime Implementation Plan (Plan 2)]] - `references` [EXTRACTED]
- [[run loop with tokioselect! on SIGUSR1, SIGHUP, SIGTERMSIGINT]] - `references` [EXTRACTED]

#graphify/concept #graphify/EXTRACTED #community/Router_Reads_2