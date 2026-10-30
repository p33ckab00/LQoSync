# Distributed pause and recovery assessment

Base: `6500d83`. Branch: `codex/distributed-controls`.
Worktree: `/home/pi/lqosync-distributed-controls`.
Use code and synthetic local tests only. No live router access, restart, or activation.

## Rulings

- Persist a revision-bound control generation per canonical data directory.
- Serialize control and settings saves with a separate nonblocking process lock.
- Revoke the old generation before settings IO. Failed or interrupted saves remain paused.
- Successful saves publish a new generation. Restart or stale in-memory configuration cannot clear a durable pause.
- Capture the generation in shadow and coordinated tickets. Recheck it at existing verification boundaries.
- Missing or invalid state blocks coordinated writes. Shadow-only synthetic roots can retain a missing-state sentinel.
- Add a pure recovery assessment for exact journal bytes, known history, complete fresh evidence, and next operation scope.
- Unknown outcomes remain blocked even if current rows appear to match a possible outcome.
- Recovery assessment is not execution approval. No persisted worker guard manifest or isolated RouterOS acceptance exists.
  Therefore resume, rollback, approval, marker removal, and live activation remain unavailable.
- Preserve existing offline fences and unrelated frontend edits.

## Proof

1. Observe failing cross-process pause/restart and recovery assessment tests.
2. Implement durable controls, settings integration, and exact read-only recovery assessment.
3. Test competing saves, corrupt/missing controls, interrupted settings IO, stale consoles, changed evidence, expiry, unknown replies, and request cancellation.
4. Obtain one fresh review. Fix concrete defects with regression proof.
5. Run format, locked Rust tests, strict all-target Clippy, and documentation checks.
6. Capture results, commit, merge with unrelated edits preserved, and push after verification.

## Completion evidence

- Random control generations revoke old tickets and grants across updated processes. Actual child-process pause passed.
- Failed saves, Paused mode, missing initialized state, and ordinary stale saves preserve inhibit. Explicit Preview resume rejects changed persisted settings.
- Corrupt state and old-byte restoration cannot revive observed authority. Competing settings saves do not enter settings IO.
- The pure recovery assessment binds exact journal bytes and validates all known history against complete fresh evidence. Unknown outcomes remain blocked.
- Fresh review found non-sticky Paused mode, missing-state initialization, and shadow ticket revival. Regressions failed before fixes and passed afterward.
- Final format passed. The locked suite passed: 326 tests, zero failures, 12 ignored. Strict all-target Clippy passed.
- Five-width Chromium checks passed with a synthetic pending journal, exact IP/list display, no horizontal overflow, 14px body text, and locked actions.
- Documentation paths, links, and diff checks passed. The language heuristic received manual review.
- No live router access or restart occurred. Runtime activation, persisted worker guard reconstruction, recovery execution approval, and RouterOS lab acceptance remain incomplete.
