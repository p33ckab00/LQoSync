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

## Return packet: B1 (2026-11-23T18:12:04+08:00, Asia/Manila)

| Item | Value |
| --- | --- |
| Task | B1. |
| Branch/worktree | `claude/topology-studio-backend` in `/home/pi/LQoSync3.0-worktrees/claude-topology-studio-backend`. Separate target directory in that worktree. |
| Base/head | Base `a0e6035` (G0). Head `86870cc54f647961bef7c8f5f3cd5be31670c6f3`. |
| Contract version | `1`, commit `f7e3977e`. B1 changes no shared type. |
| Files | `src/config.rs`, `src/router_edit.rs`, `src/cli.rs`, `src/mikrotik.rs`, `src/api/dispatch.rs`, `src/runner.rs`, `src/runner/tests/api.rs`, `src/topology_manager/mod.rs`, `src/topology_manager/source_studio/{mod,connection}.rs`, `crates/web-gateway/src/{broker,http,lib,topology_jobs,source_studio}.rs`, `crates/web-gateway/tests/{source_studio,router_edit,routers,overview}.rs`, `crates/web-gateway/examples/browser_fixture.rs`, `docs/CONFIGURATION.md`, `docs/frontend/TOPOLOGY_STUDIO_API.md`. |
| API state | Production: `GET /api/v1/topology/source-capabilities` and the `draft_test` job of `POST /api/v1/topology/source-jobs`. Service operations `source_studio.capabilities` and `source_studio.test`. Advertised features: `draft_test` only. `RouterEdit.access_profiles` works, but `access_profiles` is not advertised until `RouterSourceConfig` exists (B2). `collect` and `evaluate` jobs return `404` with `unavailable`. |
| Fixture state | All six cases. `legacy_backend` now returns `404` itself, because the production router has Studio routes. |
| Named tests | `draft_test_never_saves_or_enables`, `disabled_registered_router_stays_no_io`, `secret_is_absent_from_reports`, `shared_profile_does_not_enable_writes` (all in `source_studio/connection.rs`). Also `each_failure_names_its_stage`, `a_stalled_router_ends_at_the_deadline`, `an_invalid_draft_is_refused_before_any_connect`, three router-edit profile tests, `source_studio_ops_report_capabilities_and_never_echo_a_draft`, three gateway HTTP tests, and `a_profile_credential_needs_the_current_password`. |
| Checks | All passed: `cargo test --locked -p lqosync3 -p web-contract -p web-gateway --no-fail-fast` (1968 passed, 0 failed, 13 ignored; 1807 in `lqosync3 --lib`). `cargo clippy --locked --workspace --all-targets -- -D warnings`. `cargo clippy --locked -p web-ui --target wasm32-unknown-unknown -- -D warnings`. `cargo fmt --all -- --check`. `git diff --check`. |
| TDD note | The connection tests failed first on a `todo!()` stub. The router-edit profile tests failed first ("no change"). The config, `check`, and reauth tests were written with their code. |
| Fixture smoke | Passed on `86870cc` after the frontend lane released port `19581`: a loopback curl run of all six cases, including `legacy_backend` with `404` on each new route. |
| Not run | `tools/frontend/check-all.sh`. No live router test. |
| Migration | Config: an optional new key `access_profiles`. Old configs load unchanged. Registry schema stays 9. Rollback: revert `86870cc5`. A config with `access_profiles` then fails to load on the older binary, so remove that key first. |
| Open work | B2–B5. Known limits: the table permission check reads every row ID of each table (no count-only print). A draft test holds the service API for up to 60 seconds. A removed profile keeps its secret file on disk. `writer_username` is not an access profile. |
| Next frontend step | F1 can use `GET source-capabilities` and the `draft_test` job against the real gateway. `AccessProfileEdit` saves through the existing router edit transport with `current_password` for a new secret. |
| Time | 2026-11-23T18:12:04+08:00 |
