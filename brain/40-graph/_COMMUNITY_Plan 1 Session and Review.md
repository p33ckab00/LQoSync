---
type: community
cohesion: 0.14
members: 26
---

# Plan 1 Session and Review

**Cohesion:** 0.14 - loosely connected
**Members:** 26 nodes

## Members
- [[2026-10-31 Plan 1 lqosync3 core library session]] - document - brain/10-sessions/2026-10-31-plan1-core-build.md
- [[ADR-0007 Fresh design, not a port]] - document - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Cycle assembly rules]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[CycleOutput {rows, nodes, state, report}]] - code - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Delivery in five plans (core, runtime, v3, TUI, install)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Plan 1 global constraints]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Plan 1 review fixes (commit 529fa36)]] - concept - brain/10-sessions/2026-10-31-plan1-core-build.md
- [[Plan 1 review focus (edge cases)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Plan 2 runtime (next action)]] - concept - brain/HANDOFF.md
- [[Removal guard fingerprint (FNV-1a 64 of removed names)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Router state (online  failed  unreachable  offline)]] - concept - docs/CONFIGURATION.md
- [[RouterResult (OnlineFailedUnreachableOffline)]] - code - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[RouterSnapshot (six RouterOS tables)]] - code - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Scenario tests (replace golden parity tests)]] - concept - brain/20-decisions/ADR-0007-fresh-design-not-a-port.md
- [[Scenario tests (synthetic fixtures)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[State seed (seed_state  seed_nodes)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[Task 3 Model, state file, and seed]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Task 6 Cycle assembly and validation]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Task 8 Scenario tests and v3 check]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[Validation]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[cycleassemble() pure core function]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[lqosync3 Core Implementation Plan (Plan 1)]] - document - docs/superpowers/plans/2026-10-31-lqosync3-core.md
- [[lqosync3 core crate (config, speed, model, state, sources, topology, cycle, validate, writer)]] - code - brain/10-sessions/2026-10-31-plan1-core-build.md
- [[lqosync3 core design spec]] - document - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[state.json state file]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md
- [[toolsv3simcheck.sh (v3 compiler check)]] - code - docs/superpowers/specs/2026-10-31-lqosync3-core-design.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Plan_1_Session_and_Review
SORT file.name ASC
```

## Connections to other communities
- 13 edges to [[_COMMUNITY_DHCP Discovery and Exclusions]]
- 12 edges to [[_COMMUNITY_Scenario Test Harness]]
- 10 edges to [[_COMMUNITY_Early Sessions and ADRs]]
- 8 edges to [[_COMMUNITY_Router State Decisions]]
- 2 edges to [[_COMMUNITY_Terminal UI]]

## Top bridge nodes
- [[ADR-0007 Fresh design, not a port]] - degree 14, connects to 5 communities
- [[lqosync3 core design spec]] - degree 32, connects to 4 communities
- [[2026-10-31 Plan 1 lqosync3 core library session]] - degree 12, connects to 2 communities
- [[Cycle assembly rules]] - degree 8, connects to 2 communities
- [[Delivery in five plans (core, runtime, v3, TUI, install)]] - degree 5, connects to 2 communities