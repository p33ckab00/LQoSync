---
source_file: "docs/superpowers/plans/2026-10-31-lqosync3-runtime.md"
type: "concept"
community: "Plan 2 Runtime"
location: "L218"
tags:
  - graphify/concept
  - graphify/INFERRED
  - community/Plan_2_Runtime
---

# main run loop (tokio::select! on sleep and signals)

## Connections
- [[CycleSummarynext_sleep]] - `calls` [INFERRED]
- [[Signals (SIGHUP reload, USR1 cycle now)]] - `implements` [INFERRED]
- [[lqosync3.service systemd unit]] - `references` [EXTRACTED]
- [[runnerRunnercycle (one full cycle)]] - `calls` [INFERRED]

#graphify/concept #graphify/INFERRED #community/Plan_2_Runtime