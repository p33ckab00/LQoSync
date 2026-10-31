---
source_file: "docs/superpowers/plans/2026-10-31-lqosync3-runtime.md"
type: "concept"
community: "MikroTik Session 4"
location: "L195"
tags:
  - graphify/concept
  - graphify/EXTRACTED
  - community/MikroTik_Session_4
---

# runner::Runner::cycle (one full cycle)

## Connections
- [[CycleSummarynext_sleep]] - `shares_data_with` [EXTRACTED]
- [[cycleassemble (pure core)]] - `calls` [EXTRACTED]
- [[filesbackup (keep_backups)]] - `calls` [EXTRACTED]
- [[filesload_state (state.json or seed)]] - `calls` [EXTRACTED]
- [[fileslock (lqosync3.lock)]] - `calls` [EXTRACTED]
- [[filesread_live  Live]] - `calls` [EXTRACTED]
- [[filesrun_reload]] - `calls` [EXTRACTED]
- [[fileswrite_if_changed (atomic write)]] - `calls` [INFERRED]
- [[main run loop (tokioselect! on sleep and signals)]] - `calls` [INFERRED]
- [[routerosread_router (connect, retries)]] - `calls` [INFERRED]
- [[routerstateclassify]] - `calls` [EXTRACTED]
- [[statusStatus  RouterStatus]] - `shares_data_with` [EXTRACTED]

#graphify/concept #graphify/EXTRACTED #community/MikroTik_Session_4