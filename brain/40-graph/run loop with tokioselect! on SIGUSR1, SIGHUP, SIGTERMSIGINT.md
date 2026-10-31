---
source_file: "docs/superpowers/plans/2026-10-31-lqosync3-runtime.md"
type: "concept"
community: "Router Reads 2"
location: "L218"
tags:
  - graphify/concept
  - graphify/EXTRACTED
  - community/Router_Reads_2
---

# run loop with tokio::select! on SIGUSR1, SIGHUP, SIGTERM/SIGINT

## Connections
- [[Process run loop, SIGUSR1, SIGHUP, SIGTERMSIGINT, once, check, diff, defaults]] - `implements` [INFERRED]
- [[Review Focus (silent router, two processes, crash between writes, bad network.json, SIGHUP bad config)]] - `references` [EXTRACTED]
- [[Runnercycle (lock, read, classify, assemble, write, reload, status)]] - `calls` [EXTRACTED]
- [[lqosync3 Runtime Implementation Plan (Plan 2)]] - `references` [EXTRACTED]

#graphify/concept #graphify/EXTRACTED #community/Router_Reads_2