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

## Return packet: B2 (2026-11-23T18:34:35+08:00, Asia/Manila)

| Item | Value |
| --- | --- |
| Task | B2. |
| Branch/worktree | `claude/topology-studio-backend` in `/home/pi/LQoSync3.0-worktrees/claude-topology-studio-backend`. |
| Base/head | Base `6ab3742` (B1 packet). Head `ca8dcc801323286a833252a10fbdd039412e545f`. |
| Contract version | `1`. Additive change in `ca8dcc80`: `web_contract::source_studio::StudioCache` and `CACHE` (`source-studio.json`). Only the service and the gateway use them. No browser type changed. |
| Files | `src/topology_manager/source_studio/{snapshots,snapshots_tests,mod,connection}.rs`, `src/topology_manager/{wan,wan_tests}.rs`, `src/runner.rs`, `src/runner/tests/api.rs`, `src/api/dispatch.rs`, `src/router_view.rs`, `crates/web-contract/src/source_studio.rs`, `crates/web-gateway/src/{broker,http,source_studio,topology_jobs}.rs`, `crates/web-gateway/tests/{source_studio,routers,overview}.rs`, `docs/frontend/TOPOLOGY_STUDIO_API.md`. |
| API state | Production: `GET /api/v1/topology/source-capabilities`, `GET /api/v1/routers/{id}/source-config` (cache read, no service call), and the `draft_test` and `collect` jobs. Service operations `source_studio.capabilities`, `source_studio.test`, and `source_studio.collect`. Advertised: `draft_test`, `access_profiles`, `source_collect`, `wan_evidence`. Not advertised: `policy_preview`, `policy_save`. `fields` stays empty until B3. |
| Fixture state | Six cases, unchanged. The fixture answers the Studio routes before the production router. |
| Named tests | `route_failure_keeps_last_evidence`, `collection_limit_is_partial`, `disabled_router_is_not_collected`, `snapshot_digest_matches_loaded_config`, `the_config_view_shows_profiles_without_secret_paths`, `a_failed_route_read_is_not_an_empty_route_table` (wan), `source_collection_answers_a_snapshot_and_writes_the_studio_cache` (runner API), and the gateway tests `an_authenticated_collect_job_answers_a_snapshot` and `source_config_reads_the_cache_without_router_io`. |
| Checks | All passed: `cargo test --locked -p lqosync3 -p web-contract -p web-gateway --no-fail-fast` (1977 passed, 0 failed, 13 ignored). `cargo clippy --locked --workspace --all-targets -- -D warnings`. `cargo clippy --locked -p web-ui --target wasm32-unknown-unknown -- -D warnings`. `cargo fmt --all -- --check`. `git diff --check`. |
| TDD note | The wan test failed first (type). The snapshot tests were written with their code. A deliberate break of the last-good rule made `route_failure_keeps_last_evidence` fail, then the code was restored. |
| Not run | Browser fixture smoke (fixture unchanged in B2). `tools/frontend/check-all.sh`. No live router read. |
| Migration | No config or registry change. The service writes a new cache file `/var/lib/lqosync3/source-studio.json` (mode 0600). Rollback: revert `ca8dcc80`, then delete that file. |
| Open work | B3–B5. Known limits: a collection uses the `default` profile only. A print loads the whole reply before the row limit applies. A collection holds the service API for up to 60 seconds. The config view appears after the next cycle or collection. |
| Next frontend step | F1 can read `RouterSourceConfig` (profiles, snapshots, WAN evidence) and start a `collect` job against the real gateway. |
| Time | 2026-11-23T18:34:35+08:00 |

## B3 owner decision and sub-plan (2026-11-23T23:25:45+08:00, Asia/Manila)

Owner decision: translate each `SourcePolicyDraft` into the current engine structures (catalog sources, nodes, and `SourceSelection` rate bindings). Run the existing identity, resolver, aggregate, and preview code on a cloned registry and a cloned grace gate. Do not build a second policy engine.
A draft that the engine cannot express is a blocker with code `unsupported_in_v1`. The capabilities list advertises only the fields and operators that the translation supports.

Version 1 translation subset:

| Draft element | Translation | Other values |
| --- | --- | --- |
| `source: ppp` | One selection with scope `source_id = "ppp"` and `PppSecretName` authority, the same scope as `policy_draft::propose`. At most one enabled PPP policy for each router. | — |
| `source: dhcp` | One selection for each server, scope `dhcp-<encoded server>`, `DhcpClientMac` authority, per-site only. | — |
| `source: hotspot` or `wan` | No translation. | Blocker. Hotspot has no reference authority in the engine. WAN uses `wan.candidate_set` (B4). |
| `members` | PPP `joined`, DHCP `active`. | Blocker. |
| `filter` | No filter (`all` with no rules). DHCP: `lease.server` with `eq` or `in` selects servers. | Blocker. |
| `group` | PPP: `by_field account.profile` (one group for each profile) or `fixed` (one group). DHCP: `by_field lease.server`. | Blocker. |
| `rates` | Must give the same order as the current legacy priority of that source. | Blocker. Do not change the saved order. |
| `parent` / `capacity` | Catalog node parent and `GroupDefaults`. | Validation issue. |

Stable scopes keep every identity reference. A new subscriber has a `draft:` reference. A candidate never allocates a durable ID.

| Step | Content | Named tests |
| --- | --- | --- |
| B3.1 | Draft validation and translation (pure functions). | `hotspot_authority_is_not_invented`, unsupported-construct and equal-priority overlap blockers |
| B3.2 | Engine run on the clone with snapshot evidence: records, issues, graph, diff. | `preview_preserves_durable_state`, `missing_ppp_join_is_a_blocker`, `dynamic_dhcp_keeps_identity`, `explicit_rate_precedes_profile_rate` |
| B3.3 | Candidate cache, `source_studio.evaluate` and candidate service operations, gateway GETs and the job, advertisement of `policy_preview` and `fields`. | `paged_issues_keep_complete_counts`, `foreign_actor_cannot_read_candidate`, `restart_expires_candidate` |

## Contract change for the frontend (2026-11-23T23:28:34+08:00, Asia/Manila)

Owner decision: contract version 1 gets four more rate kinds. Commit `4ec8a0adf09df4751cd32df571d1f2059445cb2a` on `claude/topology-studio-backend`.

- `RateSource`: `profile_comment { format }`, `profile_name`, `server_comment { format }`, `settings`.
- `RateKind`: `profile_comment`, `profile_name`, `server_comment`, `settings`.
- Required frontend change: the exhaustive `match` on `RateSource` in `crates/web-ui/src/topology_studio_model.rs` (about line 256 in `codex/topology-studio-ui`) needs four new arms after integration. Claude Code did not edit that file.
- Reason: the engine rate orders use profile name, profile comment, server comment, and the configured default. Without these kinds, a draft for the default PPP configuration is a blocker.

## B3 progress and B3.2 design notes (2026-11-23T23:35:44+08:00, Asia/Manila)

- Done: contract amendment `4ec8a0a` (four rate kinds) and B3.1 `42e3ab9` (`source_studio/evaluate.rs`: `check_limits`, `translate`, `RouterIntent`).
- B3.1 checks: `cargo test --locked -p lqosync3 --lib source_studio` passed (19 tests). `cargo clippy --locked -p lqosync3 --all-targets -- -D warnings` passed.

B3.2 design, from the engine code:

1. `identity::authorize` needs current evidence for each router that has a selected scope ("selected current router evidence is missing"). Its scopes must also equal `index.required_scopes`. A single-router evaluation is not possible.
2. Evidence without router I/O: the target router uses the Studio snapshot. The other routers use `Coordinator.observed` (raw rows of the last successful source read). Add a time to `observed`, because each read clears it. A router without evidence makes the candidate `complete: false` with a blocker.
3. Add `RouterRead::from_tables` in `reader.rs` for both evidence kinds. Then `normalize_configured`. Give all evidence one synthetic generation.
4. Candidate: clone the registry. Replace the target router sources, profiles, selections, and new nodes with the result of `policy_draft::propose` on a one-router copy of the config (`pppoe.enabled`, `per_plan_node`, `dhcp.servers` from `RouterIntent`; `hotspot.enabled = false`). Keep the existing nodes with the same deterministic group IDs (parent, capacity, position).
5. Engine: `authorize_configured(candidate, revision, generation, evidence)?.reconcile(&mut ObservationGate::default())`, then set `router_locations` and `wan`, then `publication::prepare`. This includes compiler admission. Map each error (for example `PreparationConflicts`) to a blocker.
6. Output: the `PreparedPair::view()` nodes and subscribers. Map a subscriber or node that is not in the saved registry to a `draft:` reference. Discard the clone and the gate.

## B3 progress (2026-11-23T23:41:54+08:00, Asia/Manila)

| Step | State | Commit | Checks |
| --- | --- | --- | --- |
| Contract amendment | Done | `4ec8a0a` | `studio_contract` 9 passed |
| B3.1 translation | Done | `42e3ab9` | `source_studio` 19 passed |
| B3.2a kept evidence | Done | `fa6970d` | `topology_manager::` 772 passed; `kept_evidence_prepares_the_same_pair_without_router_io` proves the same rows and CSV as a live prepare, with no connection |
| B3.2b candidate | Open | — | Keep last-cycle evidence in `Studio`; merge `policy_draft::propose` for the target router into a registry clone; map the `PreparedPair` view to records, issues, graph, and diff; the four named B3.2 tests. Remove the two `expect(dead_code)` marks when the runner calls the new functions. |
| B3.3 transport | Open | — | Candidate cache, evaluate and candidate service operations, gateway GETs, advertisement of `policy_preview` and `fields`. |

## Owner Sankey reference for the frontend (2026-11-24T06:02:44+08:00, Asia/Manila)

The owner wants the Studio Sankey to look like `docs/frontend/prototypes/topology-studio-v2/04-owner-sankey-reference.png` ("ganyan na ganyan dapat").
Owner decision: build this visual style now with the current data. Live throughput comes later from a backend task. This is a frontend lane task (Codex). Claude Code did not edit `crates/web-ui`.

Visual requirements from the reference:

- Columns from left to right in hierarchy order: WAN nodes, router nodes, intermediate nodes, groups, then service cards (PPPoE, IPoE/DHCP, Hotspot).
- Cards on a dark navy canvas. Each card has a colored left accent bar, a bold title, a muted monospace subtitle, one large colored value, and small monospace stat lines.
- Curved ribbons between cards. The ribbon width is proportional to the ribbon value. Use a cyan-to-green gradient for normal paths.
- Use amber for a path that is not shaped normally, for example a backup WAN or a tree-excluded router node ("TREE BYPASS").
- Hover or focus on a ribbon opens a panel with "Source → Target" and metric tiles.

Data mapping until live throughput exists (do not show an invented value):

| Reference value | Current source | Label |
| --- | --- | --- |
| Ribbon width | `Flow.subscribers` | subscribers |
| Large card value | `Node.exported_mbps` or `calculated_mbps` | capacity |
| Subscriber count | `Node.members` / `auto_members` | subs |
| WAN RTT | `wan_health.quality_percent` and `state` | quality |
| TREE BYPASS | `Node.tree_excluded` | not in network.json |
| Throughput (Gbps), drop rate, CPU, queues, avg Mbps, shaped % | None yet | Hide, or show "no data" |

Planned backend task after B3 (not started): live throughput for each node and link from LibreQoS, with drop rate where LibreQoS gives it. The ribbons then change from subscriber counts to Gbps, with a contract change before the UI uses it.

## Return packet: B3 (2026-11-24T06:25:46+08:00, Asia/Manila)

| Item | Value |
| --- | --- |
| Task | B3 (steps B3.1, B3.2a, B3.2b, B3.3). |
| Branch/worktree | `claude/topology-studio-backend` in `/home/pi/LQoSync3.0-worktrees/claude-topology-studio-backend`. |
| Base/head | Base `d9be272` (B2 packet). Head `722a1097a2e26d8f75246b02088758b710da5096`. Steps: `4ec8a0a` (contract), `42e3ab9`, `fa6970d`, `9854081`, `722a109`. |
| Contract version | `1` with the B3 amendment `4ec8a0a` (four rate kinds). The frontend `match` on `RateSource` needs four arms after integration. |
| Owner decisions | Translate drafts into the engine (no second engine). Add the four rate kinds. |
| API state | All new routes and jobs are in production code. Advertised: `draft_test`, `access_profiles`, `source_collect`, `wan_evidence`, `policy_preview`. Not advertised: `policy_save` (B4). The section "Version 1 details from task B3" of the API contract lists the supported drafts and blocker codes. |
| Fixture state | Six cases, unchanged. |
| Named tests | `preview_preserves_durable_state`, `missing_ppp_join_is_a_blocker`, `dynamic_dhcp_keeps_identity`, `explicit_rate_precedes_profile_rate`, `hotspot_authority_is_not_invented`, `paged_issues_keep_complete_counts`, `foreign_actor_cannot_read_candidate`, `restart_expires_candidate`, `kept_evidence_prepares_the_same_pair_without_router_io`, `source_evaluation_refuses_stale_input_and_changes_no_state_file`, and the gateway test `an_evaluate_job_keeps_a_candidate_that_only_its_actor_reads`. |
| Checks | All passed: `cargo test --locked -p lqosync3 -p web-contract -p web-gateway --no-fail-fast` (1996 passed, 0 failed, 13 ignored). `cargo clippy --locked --workspace --all-targets -- -D warnings`. `cargo clippy --locked -p web-ui --target wasm32-unknown-unknown -- -D warnings`. `cargo fmt --all`. `git diff --check`. |
| TDD note | B3.1, B3.2b, and B3.3 tests were written with their code. The engine tests failed first on fixture data (compiler admission and DHCP membership), and the fixtures were corrected, not the engine. `kept_evidence` failed first on the router key. |
| Not run | Browser fixture smoke and `tools/frontend/check-all.sh`. No live evaluation on the production registry. |
| Migration | None. No config, registry, or cache file change. Candidates are in memory. Rollback: revert the B3 commits. |
| Open work | B4 (policy save) and B5. Known limits: the engine does not report the winning rate source (only a subscriber plan is named). Candidate GETs wait for the service API (10-second limit). Another router with a selected source needs its own Studio snapshot. A stale revision gives a `failed` operation, not `409` on the POST. |
| Next frontend step | F2 can call the `evaluate` job and the candidate GETs against the real gateway. The fixture still gives `409` for `revision_conflict`; the real service gives a `failed` operation with a conflict message. Treat both as a conflict. |
| Time | 2026-11-24T06:25:46+08:00 |

## Return packet: B4 (2026-11-24T06:54:13+08:00, Asia/Manila)

| Item | Value |
| --- | --- |
| Task | B4. |
| Branch/worktree | `claude/topology-studio-backend` in `/home/pi/LQoSync3.0-worktrees/claude-topology-studio-backend`. |
| Base/head | Base `49d8a99` (B3 packet). Head `2ddfa1e4a4243a03612b29e4e31325a49415bfe7`. |
| Contract version | `1`. No contract type changed in B4. |
| API state | `router.sources_set` and `wan.candidate_set` work through `POST /api/v1/topology/edits`. Advertised: all six features, including `policy_save`. The edit route takes 32 KiB. |
| Fixture state | Six cases, unchanged. The fixture `revision_conflict` case still refuses `router.sources_set`. |
| Named tests | `uninitialized_registry_refuses_policy_without_side_effects`, `policy_save_goes_through_receipts_revisions_and_digests` (stale digest, stale revision, partial read, same key once, changed payload refused, `Some([])` removes the sources, the preview of the saved policy), `wan_decision_survives_read_failure`, `schema_9_migrates_to_10_and_keeps_legacy_sources` (also `None` keeps the sources), `large_policy_uses_scoped_body_limit`, and the `add_source` refusal in `source_add_adds_the_router_and_its_role`. |
| Checks | All passed: `cargo test --locked -p lqosync3 -p web-contract -p web-gateway --no-fail-fast` (2001 passed, 0 failed, 13 ignored). `cargo clippy --locked --workspace --all-targets -- -D warnings`. `cargo fmt --all`. `git diff --check`. |
| TDD note | The schema tests failed first on the version change. The B4 flow tests were written with their code; they passed at the first run after a borrow fix. |
| Not run | Wasm clippy (no contract change in B4). Browser fixture smoke and `tools/frontend/check-all.sh`. No save on the production registry. |
| Migration | Registry schema 9 → 10 at the first load by the new binary. Before deploy, back up `/var/lib/lqosync3/topology-manager/registry.json` (unique target name). Rollback: stop the service, restore that schema-9 backup, then install the older binary. Without a backup, a schema-10 file with no `source_policies` and no `wan_decisions` differs from schema 9 only in `schema_version`; an operator can set it to 9 by hand. |
| Open work | B5 (transport and release evidence, `studio-service-smoke.sh`). Known limits: the save does not run the compiler (the cycle does). In legacy writer mode the saved policy changes the registry only; the live output follows the config until the topology writer owns publication. A removed profile or source keeps its files. |
| Next frontend step | F3 can save access profiles (router edit), wait for `loaded_config_digest`, then send `router.sources_set` with the registry revision. A failed receipt `error` starts with its code. |
| Time | 2026-11-24T06:54:13+08:00 |

## Return packet: B5 (2026-11-24T07:37:14+08:00, Asia/Manila)

| Item | Value |
| --- | --- |
| Task | B5. The backend lane (B0–B5) is complete. |
| Branch/worktree | `claude/topology-studio-backend` in `/home/pi/LQoSync3.0-worktrees/claude-topology-studio-backend`. |
| Base/head | Base `f92764b` (B4 packet). Head `238f4894d76fa4b42aacca2612a134e55f82f643`. B5 commits: `e39f912` (transport tests and operation names), `238f489` (smoke test and a fix). |
| Contract version | `1` with the B3 amendment `4ec8a0a`. No contract change in B5. |
| Owner decision | The smoke test uses an in-process harness: the installed broker needs root, and the core reads root-owned secrets. |
| API state | Final service operations: `source_studio.capabilities`, `.test`, `.collect`, `.evaluate`, `.candidate`, and `topology.web.mutate` for saves. `source_studio.config` does not exist; the source-config GET reads the service cache. All six features are advertised. |
| Named tests | `lost_policy_reply_recovers_receipt`, `apply_failure_never_reports_applied`, `studio_posts_need_the_same_origin`, `large_policy_uses_scoped_body_limit` (B4), and the earlier CSRF, actor, and bound tests. |
| Fix | The smoke test found that a `ppp` + `wan` collection could not be saved: `tables_snapshot` gave route rows (no row locator) as engine evidence. Fixed in `238f489`, with a unit assertion. |
| Checks | All passed: `cargo test --locked --workspace --no-fail-fast` (2076 passed, 0 failed, 13 ignored). `cargo clippy --locked --workspace --all-targets --examples -- -D warnings`. `cargo fmt --all`. `git diff --check`. `bash tools/integration/studio-service-smoke.sh` on `238f489`: 19 of 19 checks. |
| Smoke evidence | See the block below. It is test evidence from a synthetic router and test runtime, not production acceptance. |
| Not run | `tools/frontend/check-all.sh` (frontend lane). Production binaries as root. Publication by the topology writer: activation needs the live-test gates, so the smoke test proves the save and the engine preview, not an output pair from the topology writer. |
| Migration | None in B5. B4 schema 10 applies. |
| Open work | I0 (integration, Codex). Owner follow-up: live throughput for each node and link (Sankey reference), after integration. Known limits: see the B3 and B4 packets. |
| Next frontend step | F2/F3 use the real gateway. Integration needs the four `RateSource` arms. |
| Time | 2026-11-24T07:37:14+08:00 |

Smoke output:

    TEST EVIDENCE — synthetic router and test runtime; not production acceptance
    root /tmp/lqosync3-studio-smoke.243lCf, repo 238f489, 2026-11-24T07:36:38+08:00
    PASS core and gateway started
    PASS capabilities advertise policy_save
    PASS draft test accepted
    PASS draft test passes every stage that runs (TLS off)
    PASS draft secret absent from the operation
    PASS source config is served from the cache
    PASS topology initialize accepted
    PASS topology initialized
    PASS collection gives complete PPP tables
    PASS candidate summary is complete and publishable
    PASS candidate has the active PPP record at the profile rate
    PASS evaluation changes no durable topology state
    PASS policy save gives a saved receipt
    PASS the registry holds the policy set
    PASS the current engine prepares the saved policy
    PASS WAN decision saved
    PASS WAN evidence shows the saved decision
    PASS the fake router saw no write or other command
    PASS no password in any log
    not covered: publication by the topology writer (activation needs live-test gates); production binaries as root
    RESULT PASS

## Live throughput for the Sankey (2026-11-24T08:33:21+08:00, Asia/Manila)

Owner follow-up, backend only. Commit `854fa52304f0a1edc5df0d2622322e9bc45a1827`. The frontend is not changed.

- New endpoint: `GET /api/v1/topology/throughput` → `web_contract::topology::TopologyThroughput` (additive contract types). The API contract section "Live throughput" describes the fields.
- Frontend step for the owner Sankey reference: take the ribbon width from `flows[].down_bps` (match `from` and `to` with `TopologySummary.flows`). Take the large card value from `nodes[].down_bps` / `up_bps`. Show `rtt_ms` where the reference shows RTT. Keep the subscriber count as a label. When `stale` or `!complete`, show the state; do not show the values as current.
- Still unavailable: drop rate, router CPU, and queue counts. Hide them.
- The browser fixture serves synthetic rates for its three active subscribers (updated every second).
- Checks: `cargo test --locked --workspace --no-fail-fast` (2080 passed, 0 failed, 13 ignored), workspace clippy with examples, wasm clippy, fmt, `git diff --check`. A loopback fixture check returned live flows. No production run.
- Production note: this host has a live sample of 27 hosts, far below the 512-row limit. A larger network can reach the limit; the answer is then `complete: false`.

## Branch review before integration (2026-11-24T08:59:13+08:00, Asia/Manila)

The owner asked for a review of this branch before integration, then for a fix of every finding. Fix commit `87e56035f9c2c96785eeb3e472eaa31b2d1b94f1`.

| # | Finding | Fix |
| --- | --- | --- |
| 1 | A preview or save could use a snapshot of an older loaded config. | Refused unless `config_digest` equals the loaded digest, also for other routers. |
| 2 | A DHCP policy without a filter did not honor the config discovery rule, and the engine preflight refuses a server set that differs from the config. | A clear blocker: with `discover = false`, the policies must name exactly the listed servers. |
| 2b | Found during fix 2: the cycle reads a protocol only when the config enables it and the catalog router has its role. | A blocker for a disabled source or non-strict DHCP settings. A save sets the catalog router roles from the translation and keeps a WAN role. |
| 3 | `read_access_ref` was accepted but reads use the router credential. | A blocker for any profile other than `default`. |
| 4 | The gateway kept an unkeyed SHA-256 of a draft test request (with its password). | The digest uses a random key of the process. |
| 5 | The disabled-router rule compared address text and port. | It compares resolved addresses, on any port. |
| 6 | No test for factor fractions. | Added. |
| 7 | Duplicate RouterOS wire code. | `tests/support/routeros_wire.rs`, shared by the mock test and the fake router. |

Checks: `cargo test --locked --workspace --no-fail-fast` (2084 passed, 0 failed, 13 ignored), `cargo clippy --locked --workspace --all-targets --examples -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check`, and `bash tools/integration/studio-service-smoke.sh` (19 of 19, test evidence).
Frontend note: the new blockers use the existing code `unsupported_in_v1`. No contract type changed.
