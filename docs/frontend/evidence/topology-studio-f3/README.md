# Topology Studio F3 evidence

Timestamp: 2026-11-24T12:19:57+08:00 (Asia/Manila).

Base: `3ee06d0`. Branch: `codex/topology-studio-ui`.
Contract adoption: `3f86730` from backend `4ec8a0a`.
Code commits: `5812e74`, `5de4afa`, `41ffe5a`.

The real Leptos/Wasm UI uses the isolated B0 browser fixture.
Browser routes supply metadata changes, revision changes, access replies, and runtime evidence.
These records do not come from a production router.

## Result

Access configuration saves before policy. The UI waits for its loaded digest.
Unknown access outcomes require explicit recovery. Cancel and Escape clear secrets.

Policy save works without preview. Refusals preserve the draft.
Recovery reads the operation before any retry with the original key and payload.
A live check prevents stale resubmission after an edit or router switch.

Rebase preserves remote entries and requires choices for edits to the same entries.
Saved IDs attach to stable local rows without loss of later edits.
A saved receipt does not prove runtime acceptance.
The UI requires fresh publication and observation evidence for the same revision.
Load saved topology reads cached state without mutation.

## Images

| Files | Content |
| --- | --- |
| `studio-save-{1440,768,390}.png` | Source save and its evidence within the canvas |
| `studio-save-dark.png` | Save with the dark theme |
| `studio-save-credential-confirm.png` | Bounded credential confirmation |
| `studio-save-conflict-{1440,768,390}.png` | Explicit choices for conflicting saved and local values |

`SHA256SUMS` covers the eight PNG files.

## Verification

Workspace tests after the main corrections: 2047 passed, 0 failed, 13 ignored.
Workspace native Clippy and Wasm Clippy passed.
After the final ID/reload changes, all 96 web-ui tests and native/Wasm Clippy passed.
Those tests include all 24 Studio model cases.

On `41ffe5a`, the full browser gate passed 27 checks and both builds.
The established `pulse-check` exclusion remains.
Logs: `/tmp/lqosync-checks.yKJtHH/`. Format and whitespace checks passed.

An independent review found two Critical and six Important defects in the initial implementation.
One correction pass supplied native and browser RED/GREEN proof.
The corrected code did not receive a second independent review.
The final scenarios cover delayed replies, router switches, unknown access outcomes, and reload failures.
They also cover assigned IDs, three-way merge, stale revisions, removal, and secret cancellation.

Minor deferred issue: username whitespace and byte-count validation can differ from the backend.
The backend refuses invalid values. Earlier F0/F2 minor items remain in their packets.
The optional design inspector is unavailable. Playwright supplies rendered checks.

I0 must test real service integration and runtime publication.
The latest backend permits only the default read access profile in v1.
Align that restriction during I0. Throughput and the owner Sankey reference remain later work.
