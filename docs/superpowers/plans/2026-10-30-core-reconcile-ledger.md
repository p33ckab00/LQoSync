# Core reconciliation execution ledger

Scope: Task 2 only, pure planning and tests. Base: `e96de2a`.
Branch: `codex/core-reconcile`. Worktree: `/tmp/lqosync-core-reconcile`.
Ruling: consume a caller-supplied fresh session observation and recompute source normalization from the same inventories. No new collection or writer caller.
Ruling: reuse dynamic target scoring with a health eligibility filter for pending assignments. Preserve the original assign API and behavior.
Ruling: permit a structurally valid inactive source default only for a freshly confirmed unhealthy WAN. Preserve all structural checks and strict healthy destination validation.
Ruling: whole-empty scans and incomplete evidence never authorize cleanup. Cross-core and IP-change transitions remain explicit blocked work until a safe multi-operation writer exists.

RED: missing reconciliation module. GREEN: initial three arrival/failure/pin tests passed.
RED: recent reads extended old completed-sample eligibility. GREEN: State::status_at now bounds completed-sample age under its own policy lifetime.
Review: fixed repeated source-router index construction and bound health to the canonical settings revision and WAN identity. Follow-up found no material issue.
Focused checks: 10 reconciliation tests and 11 health/CLI checks passed. Final full suite: 256 tests passed, zero failures, 12 ignored.
Ruling: an IP change is Blocked until the multi-operation writer can justify its transition. Proposed assignments remain separate from observed membership and publication.

RED then GREEN: preserve eligible assignments when session metadata first appears; support manual-mode cleanup through current selection provenance.
RED: a current down interface was accepted under still-fresh old health. Added a strict current destination interface guard. Final full checks: 256 tests passed, zero failures, 12 ignored.
Cleanup receipts include the selected source. The caller must discard cleanup evidence after any input error.

All 13 reconciliation tests passed. Format, strict Clippy, diff, links, and Bash syntax passed.
Final reviews found no material defect. No live reads, writes, settings changes, or restarts occurred.
