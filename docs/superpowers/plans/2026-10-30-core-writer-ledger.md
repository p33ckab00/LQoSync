# Core writer execution ledger

Base: `909b4b8`. Branch: `codex/core-writer`. Worktree: `/tmp/lqosync-core-writer`.
Scope: guarded writer library, scoped production transport, recovery and synthetic tests. No live caller or writes.
Ruling: use the existing exclusive offline maintenance lease for this stage. Never upgrade the console shared lease. Scheduler coordination remains Task 4.
Ruling: transport verification must bind settings, source identity, health and authorization to an exact context before every send. The integration supplies that verifier; this stage does not invent live enable approval.
Ruling: rollback restores removed hosts with an actual new returned ID, recorded durably. It cannot restore an original deleted RouterOS ID.
Ruling: bounded plans have at most 20 mutations. Blocked planner decisions cannot enter an executable batch.

RED: missing writer module, then missing production transport. GREEN: initial engine and real-wire tests passed.
RED/GREEN: reopened verified journals must not clear fences without fresh evidence; malformed unrelated owned rows reject.
Review found missing binding-core baselines and a verifier gap after the awaited target read. Both failed regressions before correction.
The final verifier now runs after that read. The regression asserts RequestNotSent, zero commands, and a null pending intent.
RED/GREEN: expired new batches reject before marker creation. Approval lifetime is bound to batch creation and limited to 300 seconds.
Focused checks: 20 writer tests passed, including exact 20-operation forward/rollback and failed intent persistence.
Final full checks: 278 tests passed, zero failures, 12 manual tests ignored. No live caller, reads, writes, settings changes, or restart.

Fresh review used the executing-plans final-review requirement. No reviewer ran live reads or writes.
All three review findings passed follow-up inspection after regression fixes.
Final completion-window boundaries pass; reopened and failed-later verification cannot reuse earlier confidence.

The initial wallclock expiry test failed early under I/O pressure, then the full run reproduced the intended finish assertion failure. Replaced timing-dependent proof with deterministic completion-window boundaries.
RED/GREEN: a later failed forward re-verification must revoke earlier completion confidence. Clear runtime and saved verification before observing again.

Final verification: 21 writer tests and one completion-window unit test passed.
Format, strict Clippy, diff, documentation links, and Bash syntax passed. Fresh final review found no outstanding defect.
No live router reads/writes, configuration changes, shaping output changes, or restart occurred.
