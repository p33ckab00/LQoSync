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
