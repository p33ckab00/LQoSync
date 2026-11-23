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
