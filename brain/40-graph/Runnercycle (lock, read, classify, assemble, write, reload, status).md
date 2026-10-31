---
source_file: "docs/superpowers/plans/2026-10-31-lqosync3-runtime.md"
type: "code"
community: "Router Reads 2"
location: "L189-197"
tags:
  - graphify/code
  - graphify/EXTRACTED
  - community/Router_Reads_2
---

# Runner::cycle (lock, read, classify, assemble, write, reload, status)

## Connections
- [[Write order state.json first, then LibreQoS pair]] - `implements` [EXTRACTED]
- [[filesbackup with runtime.keep_backups = 10]] - `calls` [EXTRACTED]
- [[filesload_state (state.json or seed)]] - `calls` [EXTRACTED]
- [[fileslock (try_lock, 'another lqosync3 is running')]] - `calls` [EXTRACTED]
- [[filesread_live  Live (static rows, preserve network)]] - `calls` [EXTRACTED]
- [[filesrun_reload (kill on timeout, last 20 stderr lines; pending retry)]] - `calls` [EXTRACTED]
- [[fileswrite_if_changed (atomic write)]] - `calls` [INFERRED]
- [[lqosync3 Runtime Implementation Plan (Plan 2)]] - `references` [EXTRACTED]
- [[read_snapshot  read_router (timeouts, retries, session reuse)]] - `calls` [EXTRACTED]
- [[routerstateclassify]] - `calls` [EXTRACTED]
- [[run loop with tokioselect! on SIGUSR1, SIGHUP, SIGTERMSIGINT]] - `calls` [EXTRACTED]
- [[statusStatus  RouterStatus]] - `calls` [EXTRACTED]

#graphify/code #graphify/EXTRACTED #community/Router_Reads_2