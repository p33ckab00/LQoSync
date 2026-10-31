---
type: session
created: 2026-10-31
tags: [session, rust, core]
status: done
related: ["[[ADR-0005-topology-follows-v3]]", "[[ADR-0006-v3-guards-and-node-types]]", "[[ADR-0007-fresh-design-not-a-port]]", "[[v3-node-handling]]"]
---

# 2026-10-31 — Plan 1: the lqosync3 core library

## Goal

Build the pure core of lqosync3 from `docs/superpowers/plans/2026-10-31-lqosync3-core.md`, then refresh the graph and review the code.

## Changes

- Added the crate at the repository root: `Cargo.toml`, `src/` (config, speed, model, state, sources, topology, cycle, validate, writer), `tests/`, `tools/v3sim/check.sh`.
- Added nine synthetic scenarios in `tests/scenarios/`.
- Added `.graphifyignore` and refreshed `graphify-out/` and `40-graph/` (523 nodes, 30 communities).
- Fixed eight review findings with a failing test first (commit `529fa36`).

## Results

- `cargo test`: 88 unit tests, 1 config document test, 1 scenario test with nine cases. All pass.
- The v3 3.0.7 compiler accepts all nine scenario outputs. Each node keeps its `lq3-` id.
- A fresh reviewer found 0 Critical and 6 Important items. The fixes cover them and two items moved up from Minor.

## Open items

- [ ] Write Plan 2 (runtime).
- [ ] Fix the deferred minor items in [[HANDOFF]].
- [ ] Test `compile_mode` and an empty Parent Node on a v3 test shaper.

## Links

- [[2026-10-31-updatecsv-dissection]]
