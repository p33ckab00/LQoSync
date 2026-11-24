---
type: session
created: 2026-11-24
tags: [topology, frontend, studio, f2]
status: complete
related: ["[[2026-11-23-topology-studio-f1]]", "[[2026-11-23-topology-studio-tandem-plan]]"]
---

# Topology Studio F2: read-only discovery preview

Timestamp: 2026-11-24T06:51:39+08:00 (Asia/Manila).

Base: `f58b924`. Contract: `f7e3977`. Code commits: `93d5a44`, `d546293`. Test correction: `b5325fc`.
Branch: `codex/topology-studio-ui`.
Worktree: `/home/pi/LQoSync3.0-worktrees/codex-topology-studio-ui`.

## Result

The source modal reads cached evidence and explicitly starts discovery.
The typed client handles jobs, a bounded poll loop, request recovery, and scoped session errors.
Candidate cards, diff markers, and subscriber-count flows appear over the saved canvas.
The bounded modal shows paged issues and records, complete counts, source provenance, and freshness.
Load saved topology reads cached state and preserves the draft.

The actor, router, digest, revision, snapshot, and draft epoch bind each candidate.
Old replies and expired candidates cannot replace a newer draft.
The current tab keeps non-secret drafts through reauthentication by the same actor.
Explicit logout and a different actor clear them. Credentials remain outside draft memory.

## Verification

The named model cases and browser entry checks failed before implementation, then passed.
Later regressions cover phone controls, late old-session 401 replies, and manual read recovery.
The separate final author review found a parent card modal that covered the preview canvas.
Its regression test failed before the correction, then passed.
The independent reviewer could not run because of a usage limit. Author review is weaker than independent review.

Studio model: 14 passed. Workspace: 2037 passed, 0 failed, 13 ignored.
Workspace native/Wasm Clippy passed before the final modal correction.
That correction also passed native/Wasm Clippy for `web-ui`, format, whitespace, and the full browser gate.

The final browser gate passed 24 checks and both builds.

An earlier run read the old inventory layout immediately after resize.
The check now waits for the expected hidden header before its assertion. The gate skipped `pulse-check`, the established exclusion.
Logs: `/tmp/lqosync-checks.T2xkuC/`.

## Decisions and open work

- Keep preview separate from saved policy and applied output. F3 owns profile and policy persistence.

- Use app memory for non-secret draft recovery. Store no source draft or access secret in browser storage.

- Use the current layout engine and preserve saved positions. Count-based widths do not claim throughput.

- Keep the minor snapshot-selector issue for later work. Explicit discovery refresh restores a collected choice.

- Check explicit-profile username/update semantics and the combined plan-binding limit before F3 save.

- B3/G3 integration and I0 remain unverified. The browser uses only B0 fixtures.

No live router or production service changed. No merge, push, or deploy occurred.
Next action: F3. Keep the frontend branch and worktree.

## Links

- [[2026-11-23-topology-studio-f1]]

- [[2026-11-23-topology-studio-tandem-plan]]

- [Frontend handoff](../../docs/handoffs/2026-11-23-codex-topology-frontend.md)

- [Evidence](../../docs/frontend/evidence/topology-studio-f2/README.md)
