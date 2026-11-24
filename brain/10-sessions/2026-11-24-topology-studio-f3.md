---
type: session
created: 2026-11-24
tags: [topology, frontend, studio, f3]
status: complete
related: ["[[2026-11-24-topology-studio-f2]]", "[[topology-studio-save-recovery]]"]
---

# Topology Studio F3: save and recovery

Timestamp: 2026-11-24T12:11:57+08:00 (Asia/Manila).

Goal: complete F3 from the shared tandem plan. Keep I0 separate.
Base: `3ee06d0`. Contract adoption: `3f86730` from backend `4ec8a0a`.
Code commits: `5812e74`, `5de4afa`, `41ffe5a`.

Branch: `codex/topology-studio-ui`.
Worktree: `/home/pi/LQoSync3.0-worktrees/codex-topology-studio-ui`.

## Changes

Access configuration saves separately from source policy.
The UI waits for the loaded configuration digest before policy save.
Unknown access outcomes require explicit recovery. Secrets stay outside retained drafts.
Cancel and Escape clear replacement secrets and intent.

Policy save needs no preview. It uses the exact registry revision.
Recovery reads the original operation before any retry with the same key and payload.
A live check after lookup prevents stale resubmission after draft or router changes.

Saved policy IDs attach to stable local tokens without loss of later edits.
Rebase preserves other operators' entries and requires choices for changes to the same entries.
Mixed cached revisions cannot authorize save. Remove source policy edits the draft before save.

Saved receipts remain separate from fresh runtime acceptance for the same revision.
Load saved topology reads cached state without mutation.
All controls remain within the canvas or its bounded dialogs. Themes use the current settings.

Owned files: Studio API/model/save/editor, router forms, canvas integration, and module registration.
The browser gate includes three new save checks.

## Verification

The independent reviewer found two Critical and six Important defects in `5812e74`.
One consolidated correction pass supplied native and browser RED/GREEN proof.
The corrected code did not receive a second independent review.
The incorrect new-profile help also received a correction.

Workspace tests after the main corrections: 2047 passed, 0 failed, 13 ignored.
Workspace native Clippy and Wasm Clippy passed.
After the final ID/reload changes, all 96 web-ui tests and native/Wasm Clippy passed.

All 24 Studio model cases form part of those 96 tests.
Final browser gate on `41ffe5a`: 27 checks and both builds passed.
The established `pulse-check` exclusion remains. Logs: `/tmp/lqosync-checks.yKJtHH/`.
Format and whitespace checks passed before capture.

Minor deferred issue: username whitespace and byte-count validation can differ from the backend.
The backend refuses invalid values. Earlier F0/F2 minor items remain in their packets.
The optional design inspector is unavailable. Playwright supplies rendered evidence.

## Next action

Run I0 with backend packet `86b9aa4` after F3 capture.
B5 is complete in that separate worktree. Its owner reports smoke 19/19 and 2084 tests passed.
These frontend checks use B0 fixtures and the accepted rate amendment.
Browser overrides model metadata, revision, and runtime evidence transitions.
They do not prove real service integration or output publication.

The latest backend permits only the default read access profile in v1.
Align the UI with that restriction during I0. Then address the throughput endpoint and owner Sankey reference.
No merge, push, deploy, production service change, or live router I/O occurred.

## Links

- [[topology-studio-save-recovery]]
- [[2026-11-23-topology-studio-tandem-plan]]
- `docs/handoffs/2026-11-23-codex-topology-frontend.md`
- `docs/frontend/evidence/topology-studio-f3/README.md`
