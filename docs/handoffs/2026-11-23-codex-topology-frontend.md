# Codex: Topology Studio frontend handoff

Status: ready after the shared plan and G0 contract commit.
Last agent: Codex. Baseline: `d38113e` on `lqosync3`.
Project: `/home/pi/LQoSync3.0`. Backend owner: Claude Code.

## Read first

Read `brain/Home.md`, the latest `brain/HANDOFF.md` entry, and the accepted design.
Then read [the API contract](../frontend/TOPOLOGY_STUDIO_API.md) and [the tandem plan](../superpowers/plans/2026-11-23-topology-studio-tandem.md).
Read [the backend handoff](2026-11-23-claude-topology-backend.md) before each integration point.

## Your scope

Implement F0, F1, F2, F3, and I0 from the tandem plan.
Own the Leptos UI, canvas layout and controls, source editor, typed client, validation view, and browser checks.
Keep shared Rust contract, fixture, gateway, and core-service changes with Claude Code.

Keep the accepted topology cards and Sankey composition.
Use icon actions and fixed docks inside the canvas. Keep every configuration and help panel within the canvas bounds.
The information icon alone opens the workflow.

Reuse the current theme state, router form components, edit transport, layout revision, and service counts.

Use the backend-owned fixture while core service work continues.
Do not invent a production route, request shape, success state, rate calculation, or subscriber identity.
If a contract field is missing, record the exact needed change in this handoff.

Wait for the backend contract commit before its consumer change.

## Frontend requirements

- Keep saved topology, draft, candidate, and apply evidence separate.

- Ignore stale responses from another router or draft epoch.

- Keep secret input outside ordinary draft state and browser storage.

- Preserve drafts after conflicts, failures, preview expiry, and view changes.

- Save access profiles before source policy. Wait for the loaded config digest.

- Use the same request key when recovery repeats the same operation.

- Keep preview optional for a valid policy save.

- Let Load saved topology read cached state without mutation.

- Show Saved from the service receipt and Applied from matching runtime evidence.

- Keep unsupported capabilities unavailable with a clear reason.

## Worktree and integration

Use `codex/topology-studio-ui` from the agreed G0 contract commit in a separate worktree.
Record the exact branch and directory here when implementation starts.
Reserve fixture port `19581` before browser checks. Do not run the full check script with the backend lane concurrently.

Integrate accepted backend and frontend commits onto `lqosync3` once their dependency gates pass.
Run I0 on one exact integration commit. Record skipped checks separately.
Production rollout remains separate from source integration.

## Return packet

Record task ID, base/head commits, worktree, owned files, consumed contract commit, fixture cases, and exact check results.
Include screenshots, remaining gaps, the next backend dependency, and an absolute timestamp with timezone.
The planning artifacts do not start an external Claude Code session automatically.

## F0 worktree reservation

Base: `a0e6035f8fb7a92039c6e791857ee5d35333015f`.
Branch: `codex/topology-studio-ui`.
Worktree: `/home/pi/LQoSync3.0-worktrees/codex-topology-studio-ui`.
Task: F0 only. Backend files and the shared fixture remain with Claude Code.
Browser checks reserve port `19581` with `/tmp/lqosync-browser-19581.lock` during each run.
Baseline `cargo test --locked -p web-ui`: 72 passed, 0 failed.

## F0 return packet — 2026-11-23T23:08:19+08:00 (Asia/Manila)

Status: complete. The changes remain on `codex/topology-studio-ui`. No merge, push, or deploy occurred.
Worktree: `/home/pi/LQoSync3.0-worktrees/codex-topology-studio-ui`.
Base: `a0e6035f8fb7a92039c6e791857ee5d35333015f`. Contract consumed: `f7e3977`.
Code commits: `4b5b424`, `2918086`, and `1faa0c0`.

F0 adds fixed icon tools, separate zoom and appearance docks, optional help, and bounded dialogs to the real canvas.
It preserves current card data, IDs, layout requests, flows, and policy edit paths.
The application uses one theme preference and one cookie update path.

Owned files: `crates/web-ui/src/app.rs`, `main.rs`, `topology.rs`, `topology_canvas.rs`, `topology_studio_help.rs`, and `crates/web-ui/style.css`.
Browser files: `studio-shell-check.cjs`, `studio-shell-edge-check.cjs`, and the layout-control update in `topology-canvas-check.cjs`.
No backend, shared type, fixture, manifest, or lock file changed.

The independent review found five important issues. The edge check reproduced all five before the corrections.
The fixes cover landscape space, the phone Reload target, initial read state, incomplete WAN help, and standalone appearance icons.
A final edge check also caught hidden save feedback in the short view. The correction preserves the status overlay.
A minor keyboard-tooltip issue remains on Reload and the editor header Close. Both controls have accessible names.

Checks:

- `cargo test --locked --workspace --no-fail-fast`: 2023 passed, 0 failed, 13 ignored.
- Workspace Clippy and Wasm Clippy: passed with `-D warnings`.
- `cargo fmt --all --check` and `git diff --check`: passed.
- Full `STAGES=browser tools/frontend/check-all.sh`: 20 checks passed. `pulse-check` remained the established exclusion.
- Fixture and Wasm builds: passed. Full browser logs: `/tmp/lqosync-checks.4eogJE/`.
- Focused shell, edge, canvas, and topology checks: passed. Logs: `/tmp/lqosync-checks.Gb5P8j/`.

The shell uses the normal synthetic topology fixture and current service endpoints.
The B0 source-policy scenarios remain for F1–F3. This result does not claim new source-editor or discovery behavior.
Screenshots and checksums: [F0 evidence](../frontend/evidence/topology-studio-f0/README.md).
The optional design inspector is unavailable. Playwright supplied the rendered checks.

Next action: start F1 with the published B0 types and fixture. Keep new backend work in the backend lane.
Integration and I0 remain separate tasks. Keep the frontend branch and worktree for the next slice.

## F1 return packet — 2026-11-24T00:03:19+08:00 (Asia/Manila)

Status: complete. Base: `18ec855`. Code commits: `19397ac`, `9e18dd6`.
Branch: `codex/topology-studio-ui`. Worktree: `/home/pi/LQoSync3.0-worktrees/codex-topology-studio-ui`.
Contract: B0 `f7e3977`, from G0 `a0e6035`.
No merge, push, deploy, backend edit, contract edit, or fixture edit occurred.

The canvas action and card editor open the bounded router modal.
It reads `RouterInventory`, `StudioCapabilities`, and `RouterSourceConfig` as cached state.
It keeps router-scoped drafts, base revisions, profile metadata, and a local edit epoch.
Secrets use a separate shared form. Source and profile switches keep non-secret drafts.

The editor has typed filters, member sets, node grouping, parent selection, speed-plan maps, rate order, capacity, and explicit WAN write grants.
It keeps unsupported saved fields and operators visible without making them selectable.
The default profile uses the known router username. The v1 view omits explicit-profile usernames, so those require entry before an edit.
Check the B1 rule for partial and complete profile updates before F3 persistence. No contract assumption changed in F1.

Shared components: `NewRouter`, `EditableSection`, `CredentialFields`, `SavedNotice`, and the current router-edit transport.
Their Studio mode uses icons and bounded credential confirmation. Ordinary Routers pages retain their defaults.
The explicit unsaved connection test uses `StudioJob::DraftTest` and the operation result.
It saves no router, handles queued status, clears its secret, and invalidates changed connection results.
Registration saves the router disabled and returns to Connection inside the canvas.
Enable and Disable remain on Routers until a later canvas task.

The independent review found an access secret that survived Add router cancellation.
A render check found a collapsed phone selector. The new edge check reproduced both defects before the correction.
The corrections clear the secret before registration and give the selector and form independent scroll regions.
Minor deferred issue: the binding UI uses a per-row count of 128. F3 must enforce the advertised total per policy before save.

Checks:

- Studio model: 7 tests passed, including all three named F1 tests.
- Workspace: 2030 passed, 0 failed, 13 ignored.
- Native and Wasm Clippy with `-D warnings`, format, and whitespace: passed.
- Full browser gate: 22 checks passed. `pulse-check` remained the established exclusion. Fixture and Wasm builds passed.
- Final browser logs: `/tmp/lqosync-checks.8ZAiCt/`.
- Normal and legacy B0 cases: covered. The browser supplies one queued response override to exercise polling.
- Actual embedded registration, credential confirmation, secret clearing, source/profile retention, exact rates, 1440/768/390 bounds, and System themes: covered.

Evidence: [F1 screenshots and checksums](../frontend/evidence/topology-studio-f1/README.md).
The optional design inspector is unavailable. Playwright supplied the rendered checks.

Next action: F2 discovery snapshots and candidate preview with B0 fixtures, then B3 at G3.
F3 owns profile/policy persistence, CAS errors, request recovery, saved-load behavior, and runtime feedback.
Keep the branch and worktree. Integration and I0 remain separate tasks.

## F2 return packet — 2026-11-24T06:51:39+08:00 (Asia/Manila)

Status: complete. Base: `f58b924`. Code commits: `93d5a44`, `d546293`. Test correction: `b5325fc`.

Branch: `codex/topology-studio-ui`. Worktree: `/home/pi/LQoSync3.0-worktrees/codex-topology-studio-ui`.
Contract: B0 `f7e3977`, from G0 `a0e6035`.

No backend, contract, fixture, manifest, or lock file changed.
No merge, push, or deploy occurred.

The new typed client reads Studio errors and runs explicit read jobs through the operation endpoint.
It bounds request size and the job poll loop. A lost reply queries the original operation without automatic resubmission.
Manual recovery checks that operation first. An absent operation permits the original key and input to repeat.

The actor and CSRF token scope each request. A late 401 cannot end a newer session.
The draft connection test now uses the same typed client.

The source form exposes cached table evidence, age, completeness, and an explicit refresh.
Collection uses saved access configuration and the source types in the draft.
Evaluation binds the actor, router, configuration digest, base revision, snapshot, and draft epoch.
An old response, changed input, or expired candidate cannot replace the current preview.

The current tab keeps only non-secret drafts through same-actor reauthentication.
Explicit logout and another actor clear that memory.

Candidate cards and count-based flows share the current canvas and layout engine.
Saved positions remain intact. Saved card interactions become inert during preview.
The details modal shows full issue counts, paged issues and records, truncation, unknown values, and provenance.

Load saved topology sends a cached GET without a policy or layout write.
It retains the source draft. Studio closes the parent card modal before it opens. The user can then inspect the canvas preview.

Owned files: `topology_studio_api.rs`, `topology_studio_preview.rs`, Studio model/editor, app memory, router forms, canvas, CSS, help, and module registration.
Browser files: `studio-preview-check.cjs` and `studio-preview-auth-check.cjs`.

Checks:

- Studio model: 14 tests passed. All named F2 model cases passed.

- Workspace: 2037 passed, 0 failed, 13 ignored.

- Workspace native/Wasm Clippy passed with `-D warnings` before the final modal correction.

- After that correction, native/Wasm Clippy for `web-ui`, format, and whitespace passed.

- Final browser gate: 24 checks and both builds passed. `pulse-check` remained the established exclusion.

- Logs: `/tmp/lqosync-checks.T2xkuC/`. Workspace test log: `/tmp/lqosync-f2-workspace-tests.log`.

- Regression checks cover delayed results, partial reads, expiry, paged totals, lost replies, manual recovery, actor changes, and late old-session 401 replies.

- The parent-card-modal test failed before its correction, then passed in the final browser gate.

- An earlier run read the old inventory layout immediately after resize. The check now waits for its expected hidden header.

The independent reviewer could not run because of a usage limit. A separate author review supplied the fallback.
This review is weaker than an independent review. It found one important card-modal defect, now corrected.

Minor deferred issue: an older snapshot selection can hide the locally collected choice if cached metadata omits it.
F0/F1 minor items remain in their return packets.

Evidence: [F2 screenshots and checksums](../frontend/evidence/topology-studio-f2/README.md).

The optional design inspector is unavailable. Playwright supplied the rendered checks.

Next action: F3 profile/policy persistence and save recovery, after review of the B4/B5 behavior.
Check explicit-profile username/update semantics and the combined advertised limit for `PlanBinding` before save.
B3/G3 integration and I0 remain separate tasks. This packet claims B0 fixture behavior only.
Keep the branch and worktree.

## F3 return packet — 2026-11-24

Timestamp: 2026-11-24T12:19:57+08:00 (Asia/Manila).

Owner: Codex. Branch: `codex/topology-studio-ui`.
Worktree: `/home/pi/LQoSync3.0-worktrees/codex-topology-studio-ui`.

Base: `3ee06d0`. Contract adoption: `3f86730` from backend `4ec8a0a`.
Implementation: `5812e74`. Review corrections: `5de4afa`. Final stabilization: `41ffe5a`.

Access configuration saves separately. The UI waits for the loaded digest before policy save.
Unknown credential outcomes block further writes until explicit recovery.
Secrets remain outside retained drafts. Cancel and Escape clear replacement secrets.
The unchanged digest of a secret-only save cannot prove credential pickup after a failed reload signal.

Policy save works without preview and uses the exact expected registry revision.
Refusals preserve draft text. Recovery reads the original operation before a retry with the same key and payload.
A check after that read prevents resubmission after a router switch or draft change.
Saved-ID recovery preserves later local edits and never repeats a known saved write.

Rebase uses three versions: saved baseline, local draft, and current cached state.
It preserves remote additions and requires explicit choices for edits to the same entries.
Different cached revisions cannot authorize save.
The API omits explicit-profile usernames. The form requires confirmation after a configuration change.
Remove source policy changes only the draft until Save.

Saved receipts remain separate from fresh runtime acceptance for the same revision.
Load saved topology uses cached reads without a policy, layout, router, or output write.
Router-specific acknowledgments remain attached to the router that received the save during navigation.
Controls remain inside the canvas and its bounded dialogs. The current themes remain available.

Owned files: Studio API/model/save/editor, router forms, canvas integration, and module registration.
Browser checks: `studio-apply-check.cjs`, `studio-apply-edge-check.cjs`, and `studio-apply-review-check.cjs`.

Checks:

- Workspace after the main corrections: 2047 passed, 0 failed, 13 ignored.
- Workspace native Clippy and Wasm Clippy passed.
- Final ID/reload changes: all 96 web-ui tests and native/Wasm Clippy passed.
- Those tests include all 24 Studio model cases.
- On `41ffe5a`, all 27 browser checks and both builds passed.
- The established `pulse-check` exclusion remains. Format and whitespace checks passed.

- Logs: `/tmp/lqosync-checks.yKJtHH/`. Final web-ui tests: `/tmp/lqosync-f3-final-ui-tests.log`.

An independent review found two Critical and six Important defects in `5812e74`.
One correction pass supplied native and browser RED/GREEN proof.
The corrected code did not receive a second independent review.
The incorrect new-profile help also received a correction.

Minor deferred issue: username whitespace and byte-count validation can differ from backend validation.
The backend refuses invalid values. Earlier F0/F2 minor items remain in their packets.
Evidence: [F3 screenshots and checksums](../frontend/evidence/topology-studio-f3/README.md).
Playwright supplied rendered checks. The optional design inspector is unavailable.

These checks use B0 fixtures plus the accepted rate amendment.
Browser routes supply metadata, revision, access-reply, and runtime-evidence transitions.
They do not prove real service integration or output publication.
No backend implementation or shared fixture change belongs to this frontend slice.

Next: I0 with backend packet `86b9aa4`, after F3 capture.
B5 is complete in the backend worktree. Its owner reports smoke 19/19 and 2084 workspace tests passed.
The newest backend refuses non-default `read_access_ref` with `unsupported_in_v1`.
Align that restriction during I0. Consume the additive throughput endpoint and owner Sankey reference after integration.

No merge, push, deploy, production service change, or live router I/O occurred.
Keep the branch and worktree.
