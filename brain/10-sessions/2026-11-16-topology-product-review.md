---
type: session
created: 2026-11-16
tags: [topology, review, reliability, performance, ste]
status: review-complete
related: ["[[HANDOFF]]", "[[2026-11-16-topology-manager-adoption]]"]
---

# Topology product review

The owner requests a documentation review against the current product and a design optimized for performance, reliability, and integrity.
The owner requires the STE skill for later implementation text.

Development worktree: `/home/pi/LQoSync3.0`, branch `lqosync3`, commit `0ad8b8d`.
The separate `/opt/LQoSync3.0` checkout uses branch `lqosyncv3`, commit `0e12561`.
The active service uses `/usr/local/bin/lqosync3`. Neither checkout proves that executable's source revision.
Installed LibreQoS control and dataplane packages report `3.0.7.202611141944`.
Prior unrelated changes remain intact.

## Review results

- Current cycles can use stored subscriber rows and grace retention. Manager-mode active output requires fresh observations instead.
- Current identity depends on circuit names or MAC-related keys in several paths. A registry needs verified source references and multiple current endpoints.
- Current collection has no WAN discovery role.
- Reuse the current pair writer, receipts, native verifier, health reporting, and TUI caches.
- Independent file renames do not form an atomic pair transaction.
- The approved publication hold can leave old native mappings active. Report that state without a false freshness claim.
- Source-specific formulas and migrated effective settings need parity proof.
- Local plan ownership and current RouterOS speed reads need an explicit base-rate authority during migration.
- The scheduler contract still needs target, overlap, overnight-day, and DST decisions.

## Artifacts

- `docs/reports/2026-11-16-topology-product-review.md`: evidence, migration differences, and performance constraints.
- `docs/design/TOPOLOGY_MANAGER_IMPLEMENTATION.md`: proposed core records, module boundaries, ownership, and proof gates.
- `docs/superpowers/plans/2026-11-16-topology-foundation.md`: isolated pure rate/hierarchy tasks with explicit test cases.
- Updated the discussion record, initial TUI layout, and source research report with links and clarified evidence limits.

The foundation plan has not executed. No product source, live configuration, router, service, or package changed.
The owner answered the scheduler target question: subscriber policies and WAN/group capacities.
Record both targets. Keep configured true WAN capacity and ancestor ceilings as hard limits.
Overlap and time-boundary rules remain open. No live schedules were activated.

## Checks

Focused checks passed: 37 cycle, 25 source, 15 topology, 11 validation, 5 native verification, 13 TUI loader, and 11 receipt tests.
Total: 117 passed, zero failures.
The initial receipt library filter matched zero tests. The integration test command supplied the actual evidence.
Commands and results are in the review report.

Documentation whitespace, local links, source-path references, and approval-record markers passed.
Ran the ASD-STE100 heuristic checker and revised ordinary prose findings.
Technical names and list-related heuristic findings remain. The result is not certified compliance.

## Next work

Use the foundation plan for an isolated implementation slice.
Keep live export integration separate until migration, fresh-data, pair interruption, and native acceptance checks pass.
Settle scheduler targets and conflicts before enabling scheduler evaluation.
Use STE for documentation, comments, errors, and UI labels. Preserve code identifiers and commands exactly.
