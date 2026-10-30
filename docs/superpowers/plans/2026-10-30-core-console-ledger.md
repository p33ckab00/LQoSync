# Core console integration ledger

Base: `b51af71`. Branch: `codex/core-console`. Worktree: `/tmp/lqosync-core-console`.
Scope: scheduler health/reconciliation preview, operator controls, and visible automatic-enable prerequisites.
Ruling: automatic enable must reject until isolated RouterOS acceptance and integrated writer lease/verifier proof exist. An HTTP form cannot assert that proof.
Ruling: do not replace the existing console shared lease or bypass maintenance fencing. Live worker execution remains blocked, not silently enabled.
Ruling: compute preview outside the App lock from immutable snapshots; accept only unchanged settings, read epochs, and inventory identities.
Ruling: pause clears preview authority and does not stop ordinary collection/publication; collection pause remains the existing auto_stage setting.

Ruling: Task 4 ships in two stages. This stage covers preview controls and scheduling. Mutation workers, journal controls, live verification, and shared/exclusive lease coordination remain pending.
Review finding: a faster health interval accelerated ordinary publication. Separate publication timing now preserves the ordinary interval.
Regression: incomplete source evidence previously discarded useful health diagnostics. Preserve health reasons, clear the plan and cleanup, and show unknown counts.
Persistence: save settings in the blocking pool under the existing owned App guard. Do not block the async executor on fsync.
Regression: cancellation of a caller stranded the preview busy flag. A detached owner now completes bookkeeping and retains all supersession checks.
Performance: retain only revision, health, observation, and cleanup in the prior snapshot. Do not copy the previous decision plan or clone the snapshot twice.
Fresh reviewer rechecked both changes and found no remaining material defect.

Final checks: 287 tests passed, zero failures, 12 ignored. Eight console tests and 11 scheduler tests passed.
Format, strict Clippy, JavaScript syntax, diff, and documentation link checks passed. Five-width Chromium checks passed.
No live router reads/writes, production settings, shaping outputs, or service restart occurred.
