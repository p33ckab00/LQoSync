# Coordinated worker library integration

Base: `3d4e973`. Branch: `codex/coordinated-worker`.
Worktree: `/home/pi/lqosync-coordinated-worker`.
The operator authorized continued implementation with no destructive live MikroTik changes.
Use code and local mock protocol tests only. Do not contact live routers or restart services.

## Scope and rulings

- Add an explicit library-only coordinated open beside the unchanged exclusive offline open.
- Keep the console shared lease. Require the existing shadow slot for coordinated execution, so previews and other coordinated writers cannot overlap.
- Bind the coordinated lease to the canonical root, settings revision, exact batch, and policy epoch.
- Attach the coordination guard to approval. Pause or settings changes invalidate it permanently before subsequent verifier or command work.
- Preserve durable writer intent, exact row checks, fresh sources, and unknown-outcome handling.
- New coordinated sessions cannot resume an old journal or replace a marker. Recovery needs separate reviewed approval.
- Do not add a scheduler, CLI, HTTP activation, automatic mode, or recovery grant.
- The coordinated library can send commands only when an explicit caller opens an exact approved batch. This task uses synthetic loopback peers only.
- External RouterOS writers remain outside process locks. No test proves uninterrupted traffic.
- Preserve unrelated frontend planning work in the main checkout.

## Steps

1. Add failing tests for console coexistence, writer exclusion, exact root/revision binding, pause, and durable marker preservation.
2. Add a coordinated lease and cancellation guard. Reuse the writer journal and verifier instead of another execution engine.
3. Exercise actual worker protocol requests against a synthetic local peer. Check known results, rollback, lost replies, cancellation, and pause during awaited verification.
4. Obtain one fresh whole-branch review. Fix concrete defects with regression tests.
5. Run format, the locked test suite, strict all-target Clippy, and documentation checks.
6. Record the stage and pending lab acceptance. Commit, preserve unrelated edits, merge, and push after the checks pass.

Acceptance: coordinated mock execution can coexist with a console; competing work and revoked grants fail closed; existing offline exclusions still pass.
Production activation and isolated RouterOS acceptance remain pending.

## Completion evidence

- Coordinated open coexists with a console. Wrong root, approval, policy, busy slot, prior journal, and prior marker reject without replacement.
- Pause during awaited capture or confirmation prevents any mock connection or command. Resume does not revive the old grant.
- Coordinated lease drop revokes detached approval and preserves the recovery fence.
- The existing protocol test now covers offline and coordinated execution, verified rollback, lost replies, and cancelled in-flight requests.
- Fresh review found missing epoch updates in ordinary settings handlers. The new regression failed before the fix and passed afterward.
- All nine settings assignments now use the common replacement method. This revokes policy-bound work without waiting for middleware refresh.
- The implementation moves batch ownership into the existing executor. It does not clone the full baseline for coordinated open.
- Format passed. The locked suite passed: 314 tests, zero failures, 12 ignored. Strict all-target Clippy passed.
- Documentation links and diff checks passed. No UI layout changed. No new browser or performance benchmark ran.
- No live router access, settings change, service restart, production activation, or dependency change occurred.
- Runtime activation, distributed controls, recovery approval, and isolated RouterOS acceptance remain incomplete.
