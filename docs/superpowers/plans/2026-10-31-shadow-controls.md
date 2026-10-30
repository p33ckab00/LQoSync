# Shadow coordination and recovery views

Base: `6921171`. Branch: `codex/shadow-controls`.
Worktree: `/home/pi/lqosync-shadow-controls`.
The operator authorized code and mock tests. No live restart or router change.
Use the existing NOC layout and modal policy forms. Keep automatic mode rejected.

## Scope and rulings

- Coordinate one read-only shadow computation per canonical data root across consoles.
- Pause and settings changes invalidate outstanding tickets, including after awaited work.
- A shadow ticket has no mutation capability. Keep offline writer and maintenance exclusion unchanged.
- Inspect saved writer journals without opening an executor, saving a journal, or changing a marker.
- Keep known results separate from saved verification. Neither proves current actual membership.
- Show recovery approval as locked. Expose no resume, rollback, marker removal, or approval action.
- Add a separate GET-only recovery viewer for use when the normal console startup gate blocks recovery.
  It must not create an App, hold a write lease, load credentials, collect routers, or bypass normal startup.
- Bound file reads and enumeration. Reject symlinks and untrusted marker paths.
- Preserve unrelated frontend planning edits in the main checkout.

## Implementation and proof

1. Add generation-bound shadow tickets and a process fence. Test competing consoles, pause, settings drift, cancellation, and restart release.
2. Extract pure journal validation. Add bounded read-only reports for pending, historical verification, corrupt input, and missing markers/files.
3. Add pause control, recovery page, and loopback-only GET viewer. Test token/revision checks and rejected POST mutation paths.
4. Run format, the locked Rust suite, strict Clippy, documentation links, and isolated browser checks.
5. Obtain a fresh review, fix concrete findings, capture results in Brain, then commit and push.

Real RouterOS acceptance and online mutation grants remain pending. No mock result substitutes for lab proof.

## Completion evidence

- Process exclusion, ticket invalidation, cancellation, killed-process release, pause persistence, and rejected automatic mode passed.
- Journal tests preserve bytes and markers. Lost replies, corrupt history, missing files, symlinks, FIFOs, and oversized input reject safely.
- Fresh review found persisted-policy drift and failed UTF-8 read accounting. Both regression tests failed before fixes and passed afterward.
- Final format passed. The locked suite passed: 308 tests, zero failures, 12 ignored. Strict all-target Clippy passed.
- Chromium passed at widths 1440, 1100, 720, 390, and 320 with synthetic records. POST mutation paths rejected.
- Documentation links and diff checks passed. The language checker received a manual review for technical terms and list false positives.
- Actual online mutation coordination and recovery grants remain outside this completed shadow stage.
- No live restart or router access occurred.
