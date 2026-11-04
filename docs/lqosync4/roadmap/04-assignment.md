# Phase 4 — WAN assignment planner and dry-run

**Entry:** W01 snapshots and subscriber identities. **Exit:** The operator can inspect a deterministic logical assignment plan and diff. Exact RouterOS object diffs follow the G2 writable-object contract in phase 5. This phase does not apply RouterOS changes.

| ID / owner | Depends | Implementation instruction | Acceptance evidence |
|---|---|---|---|
| A01 / Core backend | W01 | Define stable subscriber/upstream IDs, capacity, eligibility, percentage targets, fixed counts and manual pins. Keep a valid assignment stable. Make policy precedence and tie-breaking deterministic. Exclude known-down WANs; preserve an explicit unknown state. | Same input gives same plan; add/remove/restart/over-capacity/unknown/down/manual cases explain each move. WAN-01…03. |
| A02 / Middle layer | A01 | Persist policy and assignment versions. Expose paged/filterable allocations, assignments, discovery and history reads plus a preview that lists every affected subscriber, current/desired WAN and reason. Reject a preview if its input version changed. Do not claim an exact RouterOS object diff before G2. Do not apply here. | 1k/10k plan timing, bounded memory, stable paging, stale-plan rejection and audit-ready reason for each move. UI-04…06, WAN-05, PERF-02, PERF-07. |
| A03 / Frontend | A02 | Build `/load-balancer` summary and four shareable tabs. Use allocation cards, grouped searchable assignment table, discovery and history. Show the logical pending plan, reason and affected count. Manual move/pin controls must stop at preview until phase 5 can show the exact object diff and apply. | Desktop/mobile/keyboard review, URL filters, no hidden row actions, no success badge for a preview. UI-05, UI-08…14. |

Use [the assignment route map](../docs/ROUTES_AND_COMPONENTS.md) and [visual system](../docs/UI_VISUAL_SYSTEM.md). A percentage target is a subscriber distribution target; it does not prove traffic balance. The planner does not own packet routing; MikroTik core does.
