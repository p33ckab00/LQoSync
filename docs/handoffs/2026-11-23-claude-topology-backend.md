# Claude Code: Topology Studio backend handoff

Status: ready for backend implementation after the shared plan commit.
Last agent: Codex. Baseline: `d38113e` on `lqosync3`.
Project: `/home/pi/LQoSync3.0`. Frontend owner: Codex. Backend owner: Claude Code.

## Read first

1. Read the project `AGENTS.md` instructions supplied by the owner.
2. Read `brain/Home.md` and the latest entry in `brain/HANDOFF.md`.
3. Read [the product design](../frontend/ROUTER_SOURCE_CONFIGURATION.md).

4. Read [the API contract](../frontend/TOPOLOGY_STUDIO_API.md).
5. Read [the tandem plan](../superpowers/plans/2026-11-23-topology-studio-tandem.md).

Use the current files as evidence. Older handoff entries contain stale branch and deployment facts.
The owner requested this division of work. Do not replace it with an independent frontend rewrite.

## Your scope

Implement B0, B1, B2, B3, B4, and B5 from the tandem plan.
Own shared Rust types, gateway HTTP, broker IPC, service commands, discovery, preview, policy storage, and publisher integration.
Own the browser fixture and its synthetic source-policy cases.
Provide `tools/integration/studio-service-smoke.sh` with real gateway/broker/core binaries and isolated fake RouterOS/runtime endpoints.
Keep every smoke-test config and output under its temporary root. Mark its runtime responses as test evidence.

Keep application UI edits with Codex after B0 mechanical compatibility changes finish.

Start with B0. Publish a compiling contract commit before the UI consumes new APIs.
Keep unavailable production capabilities false until the matching backend task passes.
Send the contract commit, fixture cases, and checks through this handoff file.

Do not infer that a file update automatically starts the frontend agent.

## Backend requirements

- Keep all RouterOS I/O in the core service.

- Preserve the disabled registered-router rule. Add a separate explicit draft test.

- Keep draft secrets out of logs, receipts, snapshots, and reports.

- Distinguish complete-empty tables from failed or partial tables.

- Evaluate the draft against frozen evidence and the complete current topology.

- Keep preview free of durable ID, grace, routing, policy, and output mutations.

- Save policy through the current edit receipt and publisher path.

- Check the config digest and registry revision separately.

- Preserve the distinction between legacy policy `None` and explicit policy `Some([])`.

- Preserve stable identities, ancestor limits, shared budgets, and one writer.

- Do not enable RouterOS write scopes or change live ownership as a side effect.

## Worktree and shared-file rules

Before parallel edits, commit the plan package and complete G0.
Use branch `claude/topology-studio-backend` in a separate worktree from the agreed G0 commit.
Record the exact branch and absolute worktree path here when work starts.

Do not edit the frontend lane's files after B0 without an explicit ownership handoff.
Only one worker uses browser fixture port `19581` at a time.
Do not force-push, deploy, restart production services, or run live collection as a test of this plan.

## Required return packet

Record this packet after each backend milestone:

| Item | Required value |
| --- | --- |
| Task | B0, B1, B2, B3, B4, or B5 |
| Branch/worktree | Exact branch and absolute directory |
| Base/head | Full commit IDs |
| Contract version | `1` and the contract commit |
| Files | Owned changed paths |
| API state | Implemented routes and advertised features |
| Fixture state | Supported scenario names |
| Checks | Exact commands, result, log location |
| Migration | Schema, backup, compatibility, and rollback method |
| Open work | Remaining task IDs and known limits |
| Next frontend step | Matching F task or required contract correction |
| Time | Absolute timestamp with timezone |

Use STE prose and exact technical names.
Do not report a task complete from a mock response or an unexecuted test command.

## Return packet: B0 (2026-11-23T17:38:53+08:00, Asia/Manila)

| Item | Value |
| --- | --- |
| Task | B0. Gate G0 is complete on the backend side. |
| Branch/worktree | `lqosync3` in `/home/pi/LQoSync3.0`. B0 precedes the lane worktrees, so it is on the integration branch. |
| Base/head | Base `50830e0d` (plan package). Contract commit `f7e3977e4164ad4da533f8deeeb8ff5a6be27ddb`. |
| Contract version | `1`, commit `f7e3977e`. Create `codex/topology-studio-ui` and `claude/topology-studio-backend` from the commit that adds this packet. |
| Files | `crates/web-contract/src/{source_studio,lib,topology,routers}.rs`, `crates/web-contract/tests/studio_contract.rs`, `crates/web-contract/tests/fixtures/studio/*.json`, `crates/web-gateway/examples/browser_fixture.rs`, `crates/web-gateway/src/topology_jobs.rs`, `src/topology_manager/edits.rs`, `src/topology_manager/edits_tests.rs`, `docs/frontend/TOPOLOGY_STUDIO_API.md`. |
| API state | Types only. The production gateway has no new route. It advertises no feature, and each new route returns `404`. The core service refuses `router.sources_set` and `wan.candidate_set` with `invalid` before any I/O. |
| Fixture state | `normal`, `partial_read`, `revision_conflict`, `expired_candidate`, `lost_reply`, `legacy_backend`. Select one with `POST /__fixture/studio/scenario` and `{ "case": "..." }`. The section "Browser fixture cases" of the API contract gives the behavior. |
| Checks | All passed: `cargo test --locked -p web-contract --test studio_contract` (9 tests). `cargo test --locked -p web-contract -p web-gateway`. `cargo test --locked -p lqosync3 --lib` (1794 passed, 10 ignored). `cargo check --locked --workspace --all-targets`. `cargo clippy --locked --workspace --all-targets -- -D warnings`. `cargo clippy --locked -p web-ui --target wasm32-unknown-unknown -- -D warnings`. `cargo fmt --all -- --check`. `git diff --check`. A loopback curl smoke test of the fixture on port `19581` passed. It covered 401, 403, 202 with `Location`, key reuse, 409, 410, 503 recovery, paging, and all six cases. Logs are in the session scratchpad only. |
| Not run | `bash tools/frontend/check-all.sh` (browser gate) and `cargo test --workspace` for the other crates. |
| TDD note | The contract types came before `studio_contract.rs`, so its first run passed. The two core tests failed first for the correct reason (unknown variant), then passed. |
| Migration | None. The registry schema stays 9. There is no config change. Rollback: revert `f7e3977e`. |
| Open work | B1–B5. Known limits: the fixture page cursor is a plain offset. `POST /api/v1/topology/edits` keeps the 4 KiB body limit until B5. A fixture job finishes `done` at once, with no `queued`/`running` steps. |
| Next frontend step | F0 can start from the G0 commit. F1 uses the fixture cases. Request a contract change through this file. |
| Time | 2026-11-23T17:38:53+08:00 |
