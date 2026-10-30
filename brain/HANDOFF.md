---
type: handoff
created: 2026-10-29
tags: [lqosync-v3, handoff]
status: active
---

# LQoSync 3 handoff

Updated: 2026-10-30T14:02:38+08:00 (Asia/Shanghai).
Last agent: Codex.
Branch: `lqosyncv3`.
Checkout: `/opt/LQoSync3.0`.
Code: `9070d1b`; source baseline: `ac4c922`. Earlier base references below are historical.

## Frontend stack selected; skills and plan complete

Updated: 2026-10-30T22:54:38+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `lqosyncv3`. Worktree: `/opt/LQoSync3.0`. Baseline: `b51af71`.
Goal: install the two requested design skills, update Tandem, and plan Dioxus with Rust/UI.
The operator explicitly limited this task to skills, Tandem, and a migration plan.
Current application code and dependencies remain unchanged. Maud still serves the console.

Installed `ui-ux` and `frontend-design` under `/home/pi/.agents/skills/` with the Codex Skill Installer.
Both `/home/pi/.claude/skills/` and `/home/pi/.codex/skills/` link to these shared installations.
Updated the maintained Tandem skill, its session hook, and its shared Codex discovery entry.
External skill changes are outside this repository. Backup: `/tmp/lqosync-tandem-20261030/`.
Optional companion skills and inspector tools were not installed.

Repository changes: migration design and plan, ADR 0003, session note, Home, this handoff, and `v3/README.md`.
The plan adds a separate Dioxus browser crate, keeps Axum, and migrates pages through a same-origin preview.
The UI/UX search database has no Dioxus preset. Use general UX guidance and official framework sources.
Rust/UI uses source components. Its installation page contains conflicting Leptos guidance.
Pin compatible Dioxus source and tooling, then compile a component before migration work.

Fresh checks: both installed skills and the shared Codex Tandem entry passed the skill validator.
The UI/UX search script returned the expected keyboard focus guidance.
Tandem Bash syntax, session-hook JSON, and all six skill discovery paths passed checks.
Local documentation links and Brain note links passed. The diff whitespace check passed.
The Codex validator rejects the maintained Claude source's pre-existing `argument-hint` metadata.
That Claude metadata remains intact. The Codex entry passed validation.
The STE heuristic ran. Technical-name and list warnings remain after prose review; this is not a compliance certification.
No Rust or browser tests ran because this task changed only skills and documentation.
The existing graph was queried for recall. It was not rebuilt for these planning notes.

Next: execute Task 1 of the frontend plan when implementation is authorized.
No frontend migration, live router action, production change, dependency install, or service restart occurred.
See [[10-sessions/2026-10-30-frontend-skills-plan]] and [[20-decisions/adr-0003-dioxus-rust-ui]].

## Guarded writer library verified; live integration pending

Updated: 2026-10-30T22:38:15+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `codex/core-writer`. Worktree: `/tmp/lqosync-core-writer`. Base: `909b4b8`.
Integration target: `lqosyncv3` in `/opt/LQoSync3.0`.
Added the bounded owned-host executor, scoped RouterOS transport, and recovery tests/guides.
Changed files: core_writer, core_writer_transport, RouterOS scoped method extraction, module exports, tests, guides, ledger, and Brain notes.
No CLI, web endpoint, scheduler, production verifier, or enable action calls the writer.
The integration must supply fresh settings/source/session/health/authorization context and actual complete core rows.
The existing exclusive maintenance lease serializes this offline stage. Console integration remains Task 4.
Batches contain at most 20 mutations. Unknown outcomes block retry/rollback and retain recovery fencing.
Rollback recreates deleted hosts with actual replacement IDs; it cannot restore deleted RouterOS IDs.
Review defects were reproduced and fixed: incomplete core scope, stale post-read context, inherited completion, and failed later re-verification.
Completion finalization now requires fresh runtime evidence; it cannot trust saved verification or expired confidence.
Fresh checks: 278 tests passed, zero failures, 12 ignored. Twenty-one writer tests and one completion-window unit test passed.
Format, strict Clippy, diff, documentation links, and Bash syntax checks passed. Final review found no outstanding defect.
No live router reads/writes, production settings, shaping outputs, credentials, service changes, or restart occurred.
Next: implement Task 4 console/scheduler integration and disabled-by-default controls. Keep production writes on hold.
See [[10-sessions/2026-10-30-core-writer]] and `v3/CORE_WRITER.md`.

## Pure reconciliation planner verified; live writer pending

Updated: 2026-10-30T20:06:38+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `codex/core-reconcile`. Worktree: `/tmp/lqosync-core-reconcile`. Base: `e96de2a`.
Integration target: `lqosyncv3` in `/opt/LQoSync3.0`.
Added the pure health-aware reconciliation planner and synthetic tests. No live caller or transport exists.
Changed files include core_reconcile, shared dynamic/session helpers, availability-aware route analysis, health evidence binding, CLI revision generation, guides, plan ledger, and notes.
New decisions cover healthy arrivals/reconnects, stable sessions, failed-WAN evacuation, explicit pin override, and confirmed offline cleanup.
Protected residuals, reused IPs, IP changes, cross-core moves, and unverified targets remain blocked.
Settings/WAN identity and completed-sample freshness bind health to decisions. Current healthy destination interfaces must also be enabled/running.
Source-router indices are cached. Shared continuity preserves eligible clients when session metadata first appears.
Manual cleanup uses the current non-excluded source selection. Input errors require discarding cleanup receipts.
Format and strict Clippy passed. Fresh review found no remaining material issue.
Final full suite: 256 tests passed, zero failures, 12 ignored. Thirteen reconciliation tests passed.
Format, strict Clippy, diff, documentation links, and Bash syntax checks passed.
No live router reads/writes, credentials, production settings, shaping output, or services changed.
Next: implement guarded writer and recovery. Isolated RouterOS provider/transition acceptance and explicit live authorization remain required.
See [[10-sessions/2026-10-30-core-reconciliation-planner]] and `v3/CORE_RECONCILE.md`.

## WAN health preview implemented; writer still pending

Updated: 2026-10-30T18:54:37+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `codex/wan-health`. Worktree: `/tmp/lqosync-wan-health`. Base: `7a9f198`.
Integration target: `lqosyncv3` in `/opt/LQoSync3.0`.
Added optional health settings, pure state transitions, conservative Netwatch parsing, and `wan-health-preview`.
The CLI only reads selected health tables. It saves no settings, inventory, health state, or assignments.
Changed code: health module, CLI, Settings field/validation, read allowlist, shared uptime visibility, tests, and one existing Settings fixture.
Included earlier authorized design, plan, notes, and quick restart guide edits. Preserve all of them during takeover.
Checks: 243 tests passed, zero failures, 12 ignored. Eleven focused health/CLI tests passed.
Format, strict Clippy, diff, documentation links, Bash syntax, and TOML example checks passed.
Fresh review found no material defect. Isolated RouterOS field and completion behavior proof remains pending.
No live router reads/writes, settings changes, service changes, or restart occurred during this implementation.
The console does not yet display or consume health. Automatic balancing and live migration remain disabled/on hold.
Next: reconciliation planner and health-aware destination eligibility. Do not call preview evidence production failover proof.
See [[10-sessions/2026-10-30-wan-health-preview]].

## Continuous balancing design ready; live migration on hold

Updated: 2026-10-30T18:39:42+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Code baseline: `7a9f198`.
The operator accepted active-client failover on confirmed WAN failure, with possible connection interruption.
Added the continuous core balancing spec and implementation plan. No runtime code changed.
Changed files: the new spec/plan/session note, Home, and this handoff.
Preserve the earlier uncommitted quick restart changes in `v3/OPERATIONS.md`.
Document links and diff checks passed. No new Rust tests ran for this planning task.
Live migration remains on hold. No router writes or restart occurred in this design task.
Next: health-state and preview implementation in an isolated worktree, then execute the remaining plan tasks.
Probe attribution and same-core transition behavior require isolated RouterOS proof before live rollout.
See [[10-sessions/2026-10-30-continuous-core-balancing-design]].

## Production adapter complete; live migration pending

Updated: 2026-10-30T18:15:32+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `codex/core-adapter`. Worktree: `/tmp/lqosync-core-adapter`. Base: `b5717dd`.
Integration target: `lqosyncv3` in `/opt/LQoSync3.0`.
Added the local maintenance CLI, scoped RouterOS adapter, console lease, persistent recovery marker, and observed-WAN bootstrap.
Changed files include the adapter, fence, CLI, router protocol patches, engine error handling, console lease, tests, guides, and project notes.
Fresh actual validation runs separately. The CLI does not publish the application current generation or apply LibreQoS output.
Unknown network outcomes retain pending state. Exact returned IDs are mandatory. Known-unsent preflight errors remain recoverable.
Bootstrap recovery accepts only the exact state before or after its candidate CAS save.
Checks: 232 tests passed, zero failures, 12 ignored. All 60 parallel lease stress runs passed.
Format, strict Clippy, diff, documentation links, and Bash syntax checks passed. Final review found no material defect.
No live router writes, credentials, settings, services, or LibreQoS output changed.
Next: regenerate fresh private evidence, review exact protected rows, and obtain live migration authorization.
Stop consoles and establish exclusive external RouterOS writer access before that separately authorized stage.
See [[10-sessions/2026-10-30-core-production-adapter]] and `v3/CORE_MAINTENANCE.md`.
Earlier integration-pending statements below describe historical stages.

## Isolated migration engine complete; production integration pending

Updated: 2026-10-30T17:11:56+08:00 (Asia/Shanghai). Last agent: Codex.
Development branch: `codex/core-migration`. Worktree: `/tmp/lqosync-core-migration`.
Integration target: `lqosyncv3` in `/opt/LQoSync3.0`. Base: `85d989c`.
Added the isolated Rust executor, private journal, synthetic tests, guide, and implementation plan.
It preserves every list's IPv4 membership, retains protected IDs/comments, and reverses known successful operations exactly.
Unknown outcomes block automatic retries and rollback. The executor does not guess the identity of an addition after a lost reply.
Checks: 217 tests passed, zero failures, 12 ignored. Format, strict Clippy, diff, and documentation path checks passed.
One fresh review found two issues. Regression tests reproduced both, fixes passed, and follow-up review found no remaining actionable issue.
Changed files: `core_migration.rs`, module export, tests, migration guide, README, plan, and project notes.
No production transport, console write route, candidate bootstrap integration, continuous writer, or publication integration was added.
No credentials, router writes, service restarts, or LibreQoS apply occurred.
Next: implement and test the production adapter and authenticated authorization, writer fencing, observed-WAN bootstrap, and actual validation integration.
The private migration preview must be regenerated from fresh evidence before production approval and execution.
See [[10-sessions/2026-10-30-core-migration-engine]].

## Fresh core audit and preview complete

Updated: 2026-10-30T14:17:38+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`.
The operator accepted the UI. Earlier UI activation-pending statements below are historical.
Fresh audit: two routers, 27 selected clients, observed core split 9/18, candidate split 13/14, and 14 logical route differences.
Private preview: five protected-row updates, two residual additions, 27 owned host additions, zero modeled WAN membership moves.
All ten simulated checks passed. Actual membership remains Conflict and no actual committed generation exists.
A second read confirmed stable lists, normalized clients, observed routes, and settings.
Independent interval-union and modeled rollback checks passed. These checks do not prove real write or rollback execution.
Changed Git files: this handoff, Home, and the refresh session note. No application code or production state changed.
Evidence remains private under `/home/pi/.local/state/lqosync3/`.
Next: design and test the guarded executor and rollback recovery. Obtain explicit protected-entry authorization before production writes.
See [[10-sessions/2026-10-30-core-membership-refresh]].

## Console operations guide complete

Updated: 2026-10-30T14:05:38+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`.
Added `v3/OPERATIONS.md` and an optional `v3/systemd/lqosync-v3.service` template. The README links to the guide.
Verified the current manual process, unit syntax, shell syntax, documentation paths, and diff.
The guide covers manual operation, one-time service installation, start, restart, stop, disable, logs, and health checks.
The systemd unit is not installed. The live process remains unchanged. UI build activation remains pending.
Next action: use the guide to activate the console or install the service when authorized.

## Console readability update complete; activation pending

Updated: 2026-10-30T14:02:38+08:00 (Asia/Shanghai).
Goal: increase readable fonts and group small cards into two or three columns.
Last agent: Codex. Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`.
Changed files: shared CSS, settings/WAN/core/policy markup, browser checks, README, and project notes.
Checks passed: format, locked build, 196 Rust tests (12 ignored), Clippy with warnings denied, both synthetic browser scripts, and diff review.
The live console process remains unchanged. No router writes, production apply, or service restart occurred.
Unfinished work: activate this verified console build when a restart is authorized.
Next action: activate the UI build, then return to the separately gated core membership migration.
See [[10-sessions/2026-10-30-console-readability]].

## Core membership audit complete; live migration pending

Last agent: Codex. Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Live code and process remain unchanged.
Authorized audit and preview are complete. The live conflict is not resolved, and no actual output was staged.
Fresh reads found eight protected broad rows in bound lists. Five overlap the 26 selected clients; three are unaffected. Five outside rows remain untouched.
Existing core membership is 9/17; candidate membership is 13/13. Fourteen memberships differ, so a direct cutover would move continuous clients.
A continuity-preserving preview retains observed WANs, modifies five broad rows, adds three residual rows, and adds 26 owned hosts.
All other address memberships are preserved by checked set subtraction. The model produces zero WAN membership moves.
All ten output checks Pass in the separate private simulation. Actual core membership remains Conflict and no committed generation exists.
Settings and actual list rows remained unchanged during the audit. Customer-level evidence and rollback details stay private under `/home/pi/.local/state/lqosync3/`.
Changed Git files: the audit session note, Home, and this handoff. No application code, credentials, router configuration, or LibreQoS output changed.
The existing updatecsv service still runs and was not modified. Its source reads routers and can write LibreQoS output.
Next: design and verify the guarded migration/rollback executor and long-running writer, including a fresh observed-WAN baseline and competing-writer checks.
The application client remains read-only. Obtain explicit protected-entry migration authorization before production writes.
Keep actual and simulated validation separate; never bypass ownership or publication gates.
See [[10-sessions/2026-10-30-core-membership-audit]].

## Live dynamic topology activation complete

The user authorized activation, actual topology configuration, and the live console restart.
Last agent: Codex. Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Active code: `9070d1b`.
Live PID: `3564997`, loopback port `9303`. The previous PID `3379628` stopped gracefully.
The verified binary was installed with the existing environment and application data directory. A private matching rollback backup was retained.
The operator selected two roots with profile/server branches and removal of the fixed service layers.
Current configuration: two roots, 20 branches, ten sources/policies, and 20 eligible node/WAN pairs. Six fixed service layers are removed.
All policies retain online-client reconnect balancing. Existing WAN capacities, physical mappings, core bindings, speeds, pins, and stable source identities remain preserved.
Fresh reads returned 14 and 12 tables, zero errors. Candidate: 26 active clients, 26 routes, 13/13 WAN split.
All 26 continuous clients kept their WAN assignments. Live inventory, speed, assignment, minimum, and capacity checks passed.
Fresh locked build, executable verification, nine HTTP routes, and read-only browser checks at three widths passed.
Credential metadata and other monitored service process IDs remain unchanged. No code or dependency changed.
Core membership still reports Conflict. No committed generation exists, and no router write or LibreQoS apply occurred.
Changed Git files: this handoff, Home, the activation session note, and a follow-up link in the implementation session note.
Unfinished activation work: none. Next: review the protected core membership conflict before production publication or routing changes.
Rollback records remain private under `/home/pi/.local/state/lqosync3/`; do not copy their contents into notes or Git.
See [[10-sessions/2026-10-30-dynamic-topology-activation]]. Earlier pre-activation runtime statements below are historical.

## Dynamic topology implementation complete

Goal complete: approved dynamic topology, modal configuration, performance checks, review corrections, and code commit/push.
Last agent: Codex. Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`.
Code commits: `7c48e06` and `9070d1b`. GitHub was verified at `9070d1b41e626ed6ebcf835d975b6f40ef88d71f` before this documentation capture.
Main was fast-forwarded to the exact verified code. The isolated `v3/` source tree matched main.
Changed code covers dynamic identities/routes, migration, assignment/session planning, shared budgets, core/output validation, topology operations, source/WAN/pin controls, and native dialogs.
Guides and synthetic tests accompany the code. The plan, spec, session note, ADR, Home, activation notes, and this handoff are part of the documentation capture.
Final checks: 196 tests passed, zero failures, 12 ignored. Format, strict Clippy, locked build, JavaScript syntax, and both packaged browser scripts passed.
The manual 50k workload passed. Policy HTTP rendering took 1.433 seconds without evidence and 5.651 seconds with the stored candidate, in debug mode.
One fresh review found five Important and two Minor issues. All were corrected and verified. No material finding remains unresolved.
An existing mock-router timeout occurred once under load; the isolated test and final full run passed without changing its timeout.
The synthetic preview stopped. The live console remains on the earlier activated `ac4c922` build; no restart or production action occurred.
Unfinished feature work: none in the approved slice. Live activation and production execution remain separate operational tasks.
Next: activate the new build under separate restart authorization, then select the actual desired hierarchy, sources, and WAN pairs. Do not infer production configuration from fixtures.
Legacy source edits that would orphan unresolved evidence are rejected until a valid dynamic generation is published. Matching private data is required for an older-binary rollback.
The Graphify map remains a baseline. Confirm current relationships in source.
See [[10-sessions/2026-10-30-dynamic-topology]] and [[20-decisions/adr-0002-independent-topology-routing]].

## Dynamic topology design (historical)

The user requires optional service layers, flat roots, arbitrary branches, configurable destinations, and add/remove/promote/demote operations.
The user explicitly confirmed independent top-level roots with preserved WAN/core associations.
Last agent: Codex. Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Source baseline: `ac4c922`.
Read `/opt/libreqos/src/updatecsv.py` without modification or execution.
Draft design: `docs/superpowers/specs/2026-10-30-dynamic-topology-design.md`.
The design separates source identities, exported parents, and routing WANs. It defines removal replacements and compatibility requirements.
The user also requested modal configuration forms, cog buttons, and correct layer order throughout the console.
The updated design includes dialog behavior, focus, scroll, errors, and overlay acceptance checks.
The independent builder review below predates this addition. The addition received a local review.
Independent builder review reported no important unanswered question. Diff checks passed. No application tests ran for this design task.
No application code, live configuration, router records, or LibreQoS output changed. The activated console remains intact.
Changed Git files: the draft spec, dynamic design session note, Home, and this handoff. Prior activation notes remain uncommitted and preserved.
Implementation plan: `docs/superpowers/plans/2026-10-30-dynamic-topology.md`.
The user asked for the next step after the modal discussion. The plan now defines six tasks and their test gates.
This planning handoff was superseded by the completed implementation above.
Subsequent user instructions authorized execution of the approved design and plan.
See [[10-sessions/2026-10-30-dynamic-topology-design]].

## Console activation complete

The user authorized the console restart. Active code: `ac4c922`, branch `lqosyncv3`, checkout `/opt/LQoSync3.0`.
The console now runs as PID `3379628` on `127.0.0.1:9303` with its existing launch environment and data directory.
The locked binary build, health check, and six UI routes passed.
Fresh collection returned 14 and 12 tables with zero errors. The candidate has 26 clients with a 13/13 WAN split.
Inventory health, subscriber speeds, and shaping capacity passed. Core membership remains Conflict. No committed generation exists.
Settings and credential metadata remain unchanged. Monitored production service PIDs remain unchanged.
No router configuration, LibreQoS output, or production shaping changed.
Changed Git files for this activation: Home, this handoff, and the console activation session note. Runtime files remain private and ignored.
Next: confirm the final hierarchy, observe reconnect continuity, and review protected manual membership before any router migration.
See [[10-sessions/2026-10-30-console-activation]]. Earlier activation statements below are historical.

## Flexible topology and reconnect assignment complete

Last agent: Codex. Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Source baseline: `df120ed`.
The NOC console and prior shared workflow/map artifacts were committed and pushed as `df120ed` before this slice.
The user authorized implementation, performance review, correction, and commit/push after successful checks.

Independent selectors now support group and individual sources, exclusions, generic destinations, and automatic branch creation.
Legacy node bindings remain valid. Source IDs and subscriber identities remain independent of hierarchy edits.
Pools default to online client count, with explicit planned-load mode still available.
Continuous sessions keep compatible assignments. New sessions and reconnects can rebalance; manual pins take priority.
Candidate observations use private `state/online-assignments.toml`, separate from the committed generation.
Healthy empty scans or core admission failures can preserve session evidence while retaining the last valid output.
Live Validation, core planning, and output share the same assignment planner. Candidate-only clients can receive validated manual pins.

Changed modules: selection, sessions, model, normalizer, assignment, orchestrator, routeros, validation, source UI, assignment UI, web, and NOC presentation.
Tests, `v3/FLEXIBLE_TOPOLOGY.md`, guides, design/plan, the session note, and ADR accompany the code.
No dependency change occurred. Private runtime data remains ignored.

Final checks: 177 tests passed, zero failed. Eleven manual benchmarks remain ignored in the normal suite.
Format, strict all-target Clippy, JavaScript syntax, and diff whitespace checks passed.
Four relevant 50k-client/selector workloads passed separately. Browser checks passed on 12 routes at 1440px, 390px, and 320px.
Browser checks also saved/restored a group parent and inspected source controls, candidate labels, and assignment metrics.
Independent review found exclusion, DHCP absence/identity, candidate pin, and same-second evidence defects. Regressions reproduced them before correction.
Final independent source review found no material issue.

Debug timing: 50k/100-profile normalization 665 ms, complete in-memory compilation pipeline 1.291 seconds.
The legacy 50k/8-WAN assignment path took 559 ms for new clients and 232 ms for retained clients.
The new 50k/2-WAN session workload took about 1.003 seconds for assignment and 1.221 seconds for retention in a standalone run.
Candidate load took 2.731 seconds; durable save took 1.059 seconds standalone versus 7.890 seconds under concurrent checks.
The publication path avoids a second TOML parse by checking the already validated file revision before save.
These debug single-run results do not establish a production latency guarantee.

Limits: DHCP counts prove bound leases, not physical connectivity. Polling needs observable absence, handle change, or uptime reset to detect reconnect.
Keep one process per data directory. Corrupt candidate data fails closed. Older binaries need their matching settings backup for rollback.
No credentials or production inventory were read. No live settings, router configuration, LibreQoS files/packages/services, or shaping changed.
The live console still runs its prior process. Only isolated preview processes were started and stopped for checks.
The Graphify map remains at its earlier source baseline. Confirm new module relationships in code until a later graph update.
Next: authorized activation of the verified console, then protected manual membership review before any future router writer.
See [[10-sessions/2026-10-30-flexible-topology]] and [[20-decisions/adr-0001-session-candidate-state]].
This note forms part of the verified behavior commit. Verify the branch tip against the remote before takeover.

## Compact NOC console update

Last agent: Codex. Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Source baseline: `32eb255`.
The user selected an attached HTML design. The console now uses its CSS with actual application data and forms.
Overview, router search/inspector, and the three-pane topology workspace use the reference layout.
The other pages share the compact shell, tables, forms, and responsive style.
Display density, mobile navigation, and collapsed creation forms work through a same-origin script.
Save/delete handlers, tokens, ownership checks, and publication gates remain intact.

Changed: `v3/src/{noc_web.rs,console.js,style.css,web.rs,lib.rs}`, `v3/tests/web.rs`, and `v3/README.md`.
The design and plan are `docs/superpowers/{specs/2026-10-30-noc-console-design.md,plans/2026-10-30-noc-console.md}`.
Home, this handoff, and the new session note record the work. Earlier skill/graph changes remain preserved.

Checks: 130 Rust tests passed, zero failed, nine manual benchmarks ignored. Format, strict Clippy, and binary build passed.
Browser checks passed on 12 routes at 1440px, 390px, and 320px widths with temporary application data.
Checks cover overflow, script policy, density, router search, node selection, creation forms, and mobile keyboard navigation.
Independent review found a closed-sidebar focus defect. The reproduced browser check passed after the fix.
Follow-up review found no material issue. Reference CSS comparison, JavaScript syntax, and diff whitespace checks passed.

The live console remains on its prior process. No live service restart, router write, or LibreQoS change occurred.
Private credentials and production inventory were not read. The Graphify map predates the UI changes and this note.
The user put flexible topology ahead of protected membership migration.
Future work: operator-selected hierarchy/categories and manual/automatic orchestration, with no RouterOS deletion from topology removal.
Automatic assignment must use online client count for new sessions/reconnects, stay stable during a session, and honor manual pins.
Topology, core membership plans, and shaping output must share the intended assignment. Failed reads must not imply disconnects.
These behavior changes are not implemented here. Current planned-load balancing and assignment retention still apply.
Next: review the visual result and complete the topology/reconnect design. Preserve protected manual membership throughout.
See [[10-sessions/2026-10-30-noc-console]].

## Graphify map update

Last agent: Codex. Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Source baseline: `32eb255`.
Goal complete: map the v3 code, guides, plans, and project notes with Graphify.
Outputs: `graphify-out/graph.html`, `graph.json`, `GRAPH_REPORT.md`, `HEALTH.txt`, and `SCOPE.md`.
Saved exclusions and the manifest support later updates. Local caches and temporary files remain ignored.
Checks: complete coverage of 38 documents, graph endpoints, source paths, query/explanation smoke, HTML export, and diff whitespace.
The graph has 892 nodes and 2,384 links in 54 named communities.
Limits: 369 unresolved references, 62 external placeholders, and 42 combined repeated links. Agent token usage was unavailable.
No runtime, router, or LibreQoS changes occurred. Application tests did not run for this map.
Next: query the map before broad code searches. Confirm results in the source files.
Use semantic `--update` to include this final handoff, Home edits, and the new capture note.
See [[10-sessions/2026-10-30-graphify-map]]. Prior product work remains the protected membership review.

## Shared skills update

The user authorized the shared TI skill workflow for LQoSync v3.
Last agent: Codex. Branch: `lqosyncv3`. Checkout: `/opt/LQoSync3.0`. Base for this task: `32eb255`.
Added `AGENTS.md` and a `CLAUDE.md` entry point. Updated Home, this handoff, and the shared-skills session note.
The instructions use existing skills and retain this project's own vault and operational limits.
Checks: shared skill paths, project references, note links, and diff whitespace. No application tests ran for this documentation change.
No runtime or production changes occurred. No new application work formed part of this task.
Next: use this workflow to review protected manual membership and a per-host migration plan without writes or ownership bypass.
See [[10-sessions/2026-10-30-shared-skills]]. Earlier implementation checks below are historical results.

## Goal

Build a Rust backend and Rust web UI for MikroTik subscriber and topology orchestration.
Use local TOML settings and state. Preserve the live LibreQoS v2.2 installation and its package hold.
The user authorized commit and push to `p33ckab00/LQoSync`, branch `lqosyncv3`.

## First slice

New application: `v3/`.
Axum serves Maud pages written in Rust.
Router forms store password environment references.
The topology editor supports explicit WAN roots and manual/computed branches.
The compiler validates parents, cycles, directional caps, duplicate IPv4 addresses, and subscriber rates.
The local publisher completes both files before it changes a durable generation pointer.
No production adapters exist in this first slice.
Existing root-level application files remain available for reference.

Changed files: `v3/`, root `README.md`, `docs/superpowers/`, and this project-specific `brain/` vault.

## Checks

- `cargo test --locked`: 11 tests passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo fmt --check`: passed.
- HTTP smoke: six routes responded; automatic local publication worked.
- Invalid snapshot smoke: the previous generation pointer remained unchanged.
- Existing v3 preflight against generated offline example: topology validated; two circuits detected.
- Overall preflight exit: 1 (existing host warnings). This is not deployment clearance.
- Independent review: fixed computed-parent validation and generation directory durability. Follow-up review found no remaining material issue in those fixes.
- Live package remains `2.2.202608181454`, held. LibreQoS and sync process IDs remained unchanged.

## Next work

Normalize the new read-only RouterOS inventory into source/health snapshots before live subscriber ingestion.
Separate the discovered network graph from real shaping nodes.
Read source-owned subscriber speeds; correct router `rx/tx` direction for asymmetric PPP rates.
Current script defaults and aggregation rules must not be misrepresented as router-native policy.
Add stable WAN assignments and system-owned individual IPv4 `/32` core lists.
Review existing routing rules before any router write.
Validate the exact v3 reload authentication and applied-generation acknowledgement.
Add user authentication, controlled deployment, audit history, and generation retention before network exposure.
The current console binds only to loopback and publishes only local output.

Start the console from `v3/` with `cargo run --locked`.
See `v3/README.md` for the offline example and SSH access.


## RouterOS and version detection update

Added the pinned `mikrotik-rs` 0.7.0 client with verified TLS and operation deadlines.
The Routers UI reads allowlisted inventory tables and saves per-table errors in local TOML.
It does not replace the subscriber snapshot. Missing credentials fail before a connection.
Settings and inventory remain under this application, separate from LibreQoS.
The UI clears active inventory after router settings change. Historical files remain diagnostic only.

The latest client releases changed to AGPL-3.0-only. The selected release retains MIT/Apache licensing.
Independent review found a real burst-response defect in its transport dependency.
The local `mikrotik-tokio` patch awaits bounded sends for reply and terminal events.
The 200-row burst test failed before the patch and passed after it.
Source revision, copyright, licenses, and patch limits are in `v3/vendor/mikrotik-tokio/`.
The client must consume commands sequentially. Do not retain unread concurrent receivers.

The Settings UI detects only the fully installed main `libreqos` package.
It selects legacy for 2.0–2.2 and the v3 scheduler family for 3.x.
Other versions and incomplete package states block selection.
The separate v3 preflight package does not select the v3 adapter.
HTTP smoke on this host selected legacy for `2.2.202608181454`.
Actual production apply execution remains disabled for both families.
V3 needs an authenticated `ReloadLibreQoS` request and applied-generation verification.
A scheduler request acknowledgement is not proof of applied shaping.

Changed files: `v3/src/{routeros,libreqos,model,web,lib}.rs`, Rust dependencies,
transport vendor files, RouterOS/version/UI tests, documentation, and this vault.
The new plan is `docs/superpowers/plans/2026-10-29-routeros-version-adapters.md`.


Update checks: 21 application tests passed; formatting and strict Clippy passed.
The HTTP smoke selected the correct legacy family on this host.
Independent repeat review found no remaining material issue after the burst/TLS fixes.
An optional `cargo test -p mikrotik-tokio` could not run because that dependency is not a workspace member.
Application protocol tests exercise the patched transport directly.
The dependency uses let chains; the declared Rust minimum is now 1.88.
The actual checks ran with Rust 1.99.1.


## Speed rules update

The user authorized speed rules, performance/integrity review, commit, and push.
Base commit for this slice: `56278d6`; branch and checkout remain unchanged.
New modules: `v3/src/speeds.rs`, `speed_preview.rs`, and `speed_web.rs`.
Changed: model settings, read-only RouterOS fields, web routes/inventory sharing,
Rust dependencies, speed and web tests, documentation, and this vault.

Speed resolution and UI previews are complete. Subscriber snapshot publication is not connected to inventory yet.
See [[10-sessions/2026-10-29-speed-rules]] and `v3/SPEED_RULES.md` for exact precedence and semantic corrections.
The old script was read as a reference. It was not run or modified.
No production credentials, router writes, LibreQoS writes, package changes, or service restarts occurred.

Checks: 39 functional tests passed; manual performance test passed separately.
Last benchmark: 50,000 PPP sessions, 100 profiles, 220.3 ms in a debug build.
Strict lint, formatting, diff checks, and HTTP smoke passed.
Independent review found numeric-token and duplicate-selection defects. Both were fixed with regression tests.
A follow-up whitespace regression was also fixed. Final review found no remaining material issue.
Concurrent policy changes invalidate preview work; a deterministic queued-worker test verifies this.
The preview and policy save preserve subscriber snapshots and staged generations.

Next action: implement stable subscriber identity and source selection, then publish only complete valid snapshots.
Keep the discovered map separate from the shaping tree. Do not apply production shaping in that work without scope authorization.


## Subscriber normalization update

Goal complete: subscriber normalization, discovered source selection UI, automatic local publication, review, and verification.
Branch/worktree remain `lqosyncv3` at `/opt/LQoSync3.0`. Base for this slice: `7623e31`.
Last agent: Codex.
Changed modules: `normalizer`, `orchestrator`, `sources_web`, `model`, `store`, `speed_preview`, `speed_web`, `web`, and `main`.
Changed tests, documentation, UUID feature/lockfile, and project notes accompany the code.

Sources bind exact router/profile/server metadata to real subscriber nodes.
Scheduled collection polls only selected routers, at most four at a time. Pausing stops pending launches.
Healthy source changes, reads, and policy saves trigger local normalization automatically.
The snapshot and both generated files share one authoritative immutable generation pointer.
The state snapshot is a compatibility mirror. A failed mirror does not invalidate committed data.
Unknown, partial, stale, empty, or conflicting collections retain the last valid generation.

Verification: 59 functional tests passed. Clippy, formatting, and diff checks passed.
Two manual benchmark tests are ignored by default; the new normalization benchmark passed separately.
Final debug measurement: 50k subscribers /100 selected profiles, 639.2 ms normalization, 921.7 ms including compilation.
Source HTTP tests and a full local mock-router scheduled round prove automatic local publication.
Independent review fixes cover active DHCP fields, unknown nonempty mappings, exact names, queued-read pause, and UI work placement.
Repeat review found no remaining material issue.

No production credentials were used. Mock logins use a non-secret environment value only against local mock sockets.
No LibreQoS files, packages, services, shaping, or router configuration were changed.
Operation and limits: `v3/SUBSCRIBERS.md` and [[10-sessions/2026-10-29-subscriber-normalization]].
Next action: validate representative isolated router behavior before deployment or additional production integrations.
Stable WAN/core-list assignment, effective RADIUS, IPv6/multi-device circuits, authentication, and generation retention remain future work.
Run one app process per data directory. Production export/apply remains disabled.

## WAN assignment update

Goal complete: WAN mapping and subscriber assignment UI, automatic local publication, performance/integrity review, and verification.
Last agent: Codex. Branch/worktree: `lqosyncv3`, `/opt/LQoSync3.0`. Base: `a156ec7`.
Changed modules: `assignment`, `assignment_web`, `wan_web`, `model`, `orchestrator`, `store`, `web`, and `lib`.
Tests, `v3/WAN_ASSIGNMENTS.md`, README, subscriber guide, plan, and project notes accompany the code.

WAN node router/source fields hold the explicit interface mapping. Manual root capacities remain authoritative.
Mapped WAN core routers join collection. Fresh complete interface health gates normalized publication.
Source pools permit compatible same-kind targets without another source binding.
Pins override assignments. Optional planned-load balancing applies to new/unassigned subscribers only.
Valid previous assignments remain stable across disconnects, capacity changes, restart, and pin removal.
The state keeps inactive records without adding their load. Policy removal prunes ineligible inactive records.
This is an intended exported parent. Actual subscriber routing remains unchanged.

`assignments.toml` now commits with the snapshot and output pair before the authoritative generation pointer changes.
The loader checks active identity/parent consistency and blocks missing or corrupt declared assignment state.
Older normalized generations remain readable and seed ownership from subscriber parents.
Settings and generation guards protect concurrent edits and publication.
Subscriber pages contain at most 100 rows. Pagination preserves encoded search.
Source-pool controls currently display the first 200 bound sources.

Checks: 68 functional tests passed, formatting and strict Clippy passed, diff checks passed.
The new manual benchmark passed separately: 50,000 subscribers across eight WANs.
Debug assignment timings: new 525 ms, retained 229 ms. Collection and disk publication are excluded.
Independent review fixes cover offline ownership loss and search pagination. Follow-up review found no remaining material issue.
HTTP tests verify forms and automatic publication. Local mocks verify core collection and WAN failure retention.
No LibreQoS files, packages, services, shaping, or router configuration changed.

Unfinished work: actual core address lists and routing verification, production apply, authentication, retention, and representative router validation.
Next action: design system-owned per-subscriber core lists with route consistency checks before any router write implementation.
No development console process was restarted by this slice. Code and route tests use isolated application data.
Commit and push are authorized. Check the branch tip and actual remote head before a takeover.

## Core planner update

Goal complete: Core address-list planner and route validation UI, automatic local generation, performance/integrity review, and checks.
Last agent: Codex. Branch/worktree: `lqosyncv3`, `/opt/LQoSync3.0`. Base: `7c7b68c`.
Changed modules: `core_plan`, `core_routes`, `core_web`, `model`, `routeros`, `store`, `orchestrator`, `web`, and `lib`.
Tests, README, subscriber/WAN guides, `v3/CORE_PLANNER.md`, plan, and brain notes accompany the implementation.

Core lists binds WANs to their mapped core/both routers, unique address lists, and custom FIB tables.
The planner derives individual subscriber `/32` members from the same assigned snapshot used for shaping output.
Persistent UUID ownership markers scope add/keep/remove entries. Manual and other-owner rows remain protected.
Overlaps, broad/DNS ambiguity, malformed markers, duplicate ownership, and dynamic/disabled owned rows block publication.
Only current configured lists permit removals. Retired or unbound lists remain untouched.

Initial route checks support RouterOS 7 direct prerouting list-to-table marks and one resolved active WAN default.
Complex rules, PCC matchers, jumps, routing rules, FastTrack, ECMP, and ambiguous defaults require review.
This is structural evidence only. It does not prove the packet path or implement router writes.
Core/both reads include five new allowlisted print tables and complete route/firewall matcher fields.
There are no mutation commands.

`core-plan.json` commits as the fifth file under the authoritative generation pointer.
The loader checks membership, IPs, parent nodes, ownership, bindings, and inventory provenance.
Old generations remain readable. Corrupt or missing declared plans block managed publication.
The plan records selected source routers and source/core collection timestamps.
Final freshness checks run after file writes, immediately before the pointer changes.
This fixes the independent review finding about distro expiry during planning or commit work.
Regression proof: the separate-core/source test failed before the guard and passed after it.

Checks: 81 functional tests, formatting, strict Clippy, build, and diff checks passed.
Four manual benchmarks are ignored by default. The new benchmark passed separately.
50k subscribers /60k members, debug: plan 876 ms, integrity 211 ms, compact JSON 791 ms.
These measurements exclude network collection and disk publication.
Actual HTTP smoke: four routes passed using isolated empty data, then the temporary process and files were removed.
No production credentials, router configuration, LibreQoS files/packages/services, or existing processes changed.

Unfinished work: actual core writes and coordinated shaping apply, representative router validation, broader routing policies, authentication, and retention.
Next action: validate isolated real-world policy fixtures and define write-time ownership rechecks, transaction order, rollback, and actual-result verification.
See [[10-sessions/2026-10-29-core-list-planner]] for details.
Commit/push remain authorized. Verify the local tip and actual remote head before takeover.


## Actual core review update

Goal complete: read-only actual policy diagnostics, private TOML credentials, UI, performance/integrity review, and checks.
Last agent: Codex. Branch/worktree: `lqosyncv3`, `/opt/LQoSync3.0`. Base: `7141844`.
Changed files include the new credentials loader, shared core analyzer, diagnostics, review UI, fixture projector, and diagnostic CLI.
Router collection paths, model validation, guides, tests, sanitized fixtures, and project notes accompany the code.

The user completed private credentials and independently confirmed the private CA fingerprint.
Only the app-local core CA path changed. Verified RouterOS 7.25 reads completed 11 tables with zero table errors.
Raw inventory and credentials remain under ignored application data. Do not inspect or copy secrets during a takeover.
Actual findings: five complex-prerouting, four conditional-matcher, and one mapping-required.
Two paths remain observed only. No explicit WAN/list/table bindings exist in current local settings.
PCC and FastTrack entries are disabled. Active counts are zero.
Connection-mark policy remains unsupported. Subscriber-to-WAN paths and writer readiness are not proven.

Diagnostics and planning share one conservative route analyzer.
Saved data stays historical and cannot enter live collection or publish output through review.
Specific FIB switch support fixes an actual empty-value representation. Other boolean flags remain strict.
Independent review identified silent observed-path truncation. The report now includes an explicit global finding and omitted count.
Regression tests cover that limit, unknown filter actions, FIB, credential redaction, saved/live separation, and superseded UI work.
The sanitized actual-policy projection preserves the finding histogram, not full address or packet semantics.

Checks: 95 functional tests passed. Five manual benchmarks are ignored by default.
The new benchmark passed separately: 1,000 mapped WANs /100k routes /50k list rows, 28.1 ms debug review.
Formatting, strict Clippy, diff checks, and four-route isolated HTTP smoke passed.
A final verified read repeated the actual results. No generation was published by review or smoke.
No router configuration, LibreQoS files/packages/services, or existing processes changed.

Unfinished work: explicit WAN bindings, broader connection-mark policy support, isolated packet-path validation, router writer, and coordinated shaping apply.
Next action: confirm intended WAN/list/table relationships and design support for the actual connection-mark policy before a writer.
See [[10-sessions/2026-10-29-actual-core-validation]] and `v3/CORE_REVIEW.md`.
Commit/push are authorized. Verify the actual remote branch hash against the local tip before a takeover.


## DHCP server speed update

Goal complete: all three DHCP server speed sources, per-server UI, normalization integration, performance/integrity review, and checks.
Last agent: Codex. Branch/worktree: `lqosyncv3`, `/opt/LQoSync3.0`. Base: `0b365ab`.
Changed modules: speeds, speed_preview, speed_web, dhcp_speed_web, sources_web, routeros, model, web, and lib.
Tests, shared fixtures, guides, the plan, and project notes accompany the code.

The user selected DHCP server comment, name, and per-server manual settings.
Rules use exact router/server keys under `speed_rules.dhcp_servers`.
The agent default is Auto: comment, unit-marked name suffix, then configured manual fallback.
Explicit Comment, Name, and Manual modes are also available. Manual values are subscriber download/upload Mbps.
Bound leases inherit the actual server's rule. Lease comment/rate overrides no longer apply.
Branch capacities, aggregation factors, and WAN caps remain separate.
Unknown or invalid server evidence blocks resolution. Failed new policy keeps the last valid local generation.
Old settings remain readable. Lease-only plans need finite server policy before their next valid publication.
No manual defaults were inserted into actual settings.

The UI discovers fresh server choices, saves local rules, rejects stale forms, preserves rules on global saves, and supports removal.
Each shared server is parsed once per preview. Minimum-rate checks avoid repeated full-policy scans in subscriber loops.
Independent review found one malformed-name suffix bug. A failing regression preceded the slash/whitespace guard.
Space and tab cases pass. The review found no additional material issue.

Checks: 100 functional tests passed; six manual benchmarks remain ignored by default.
The new DHCP benchmark passed separately: 50k leases /1,000 server rules, 245.8 ms debug, excluding collection and disk I/O.
Formatting, strict Clippy, build, diff checks, and four-route isolated HTTP smoke passed.
The verified latest binary now runs in the LQoSync web process. Existing app settings and confirmed CA paths remained unchanged during this reload.
Fresh verified reads completed 14 distro and 11 core tables, zero errors. Four DHCP servers include collected comments.
The preceding read-only support steps configured both independently confirmed CA files in this app only.
MikroTik configuration, LibreQoS files/packages/services, and production shaping remain unchanged.

Next action: select per-server policy on Speed rules and bind each DHCP server to its intended shaping node.
Broader core-policy support, router writes, coordinated apply, authentication, and retention remain unfinished.
See [[10-sessions/2026-10-29-dhcp-server-speeds]] and `v3/SPEED_RULES.md`.
Commit/push authorization continues. Verify the actual remote head before takeover.


## Connection-mark and output validator update

Goal complete: read-only connection-mark analysis and end-to-end output review, performance/integrity review, and checks.
Last agent: Codex. Branch/worktree: `lqosyncv3`, `/opt/LQoSync3.0`. Base: `3b99e33`.
Changed modules: core_connections, core_analysis, core_routes, core_diagnostics, core_review_web, validation, validation_web, store, core-validate, policy_fixture, web, and lib.
Tests, sanitized fixture, guides, implementation plan, and project notes accompany the code.

The user chose validator completion first. Configure actual WAN capacities, subscriber sources, and core bindings through the UI later.
Do not invent topology, rates, or WAN/list/table relationships. Existing settings contain no topology or core bindings.
The actual core has two WAN-ingress marks for router replies, separate from direct subscriber list routing.
Both narrow reply mark-link checks pass. Overall policy remains Unsupported with eight findings: three complex-prerouting, four conditional-matcher, and one mapping-required.
No subscriber-forward connection-mark chain or packet-path proof has been established.

Validation compares current captured inputs with one committed local generation: snapshot, assignments, CSV, JSON, and core plan.
Review never publishes or promotes historical input. Missing setup/generation produces Unavailable.
CLI `--end-to-end` requires Pass for exit code zero after it saves the private report.
Independent review fixed reply classifiers that excluded local destinations and false successful CLI exit codes.
Failing regressions preceded both fixes. No additional material issue was reported.

Checks: 111 functional tests passed; seven manual benchmarks are ignored by default.
The 500-marked-WAN /100k-route benchmark passed separately at 28.8 ms debug, excluding collection and disk I/O.
Formatting, strict Clippy, binary build, diff checks, and four-route isolated HTTP smoke passed.
The latest binary now runs in the LQoSync loopback console. Fresh verified reads returned 14 distro and 11 core tables with zero errors.
Validation and Core review live routes passed. Actual settings, CA paths, and generation pointer remained unchanged.
Only the LQoSync console was restarted. No router configuration, LibreQoS files/packages/services, or production shaping changed.

Next action: configure the intended WAN/source/core mappings in the UI, then review remaining conditional/bypass policies.
Actual end-to-end validation remains Unavailable until setup and supported policy produce a committed generation.
Router writer, coordinated shaping apply, full packet-path validation, authentication, and retention remain unfinished.
See [[10-sessions/2026-10-29-connection-mark-validation]] and `v3/VALIDATION.md`.
Commit/push remain authorized. Verify the actual remote branch hash against the local tip before takeover.


## Actual local configuration complete

Last agent: Codex. Branch/worktree: `lqosyncv3`, `/opt/LQoSync3.0`, base `6bafc91`.
The user confirmed two 320/320 Mbps WANs, factor 1.0, and automatic balancing for all service types.
PPP profile WAN is the source/default only. No subscriber pins exist.
The agent configured the current UI: two WAN roots, six service branches, ten source branches, ten source bindings/pools, and two core list/table mappings.
Sources comprise three DHCP servers, six PPP profiles, and the default Hotspot profile. Management DHCP remains excluded.
Hotspot has no active clients or finite rate evidence. Future unresolved rates must block publication.
Router connection settings and confirmed CA paths remain unchanged. Automatic collection/local publication is enabled.

Live Validation passed inventory health, speeds, assignment, and shaping capacity for 25 subscribers.
Candidate split: WAN1 has 12 clients (3 DHCP, 9 PPPoE), WAN2 has 13 (1 DHCP, 12 PPPoE).
Each WAN carries 650 Mbps of total plan rates under a 320 Mbps root cap.
The unchanged 50 percent minimum policy totals 325 Mbps per WAN. Review this 5 Mbps excess before any production use.
Seven core findings remain: three complex-prerouting and four conditional-matcher findings. Core membership is Unsupported.
No generation was published. End-to-end remains Unavailable without a committed generation. Candidate assignments are not committed sticky state or actual routes.

The private `v3/data/state/setup-preview/` contains a candidate pair and records from the compiled app library.
Its nine exported nodes have 25 unique CSV identities/IPs, valid parents, and directional ancestor caps.
The installed v3 preflight validated the topology and 25 circuits with pre-install exit 0 under filesystem-write confinement.
This is a file-pair compatibility check, not packet-path or runtime clearance.
Fresh verified reads completed 14 distro and 11 core tables without errors. Eight UI routes and scheduled collection passed.
No app code changed or application tests repeated. No router or LibreQoS change occurred.

Changed Git files: Home, this handoff, and the actual-setup-review session note. Only sanitized project notes belong in the commit.
Private settings, backups, reports, credentials, and candidate files remain ignored. Do not copy them to Git.
Next: review the remaining core policy support and aggregate minimum rates without bypassing gates or changing router rules.
See [[10-sessions/2026-10-29-actual-setup-review]]. Commit/push authorization continues.


## Scoped routing and minimum allocation complete

Last agent: Codex. Branch/worktree: `lqosyncv3`, `/opt/LQoSync3.0`. Base: `1a51b9e`.
The user authorized automatic proportional minimum allocation with floors and narrow support for the seven routing findings.
The analyzer reads interface-list definitions and members. Indexed evidence and a 512-candidate budget bound scope work.
Known filter/mangle writers cannot mutate a checked exclusion list. RAW, scripts, and external updates remain outside this structural review.
Publication and Validation allocate effective minima after assignment. The compiler independently enforces every ancestor budget.
Source-managed topology edits validate an allocated temporary copy. Failures preserve caller records and prior generations.

Independent review found three defects: mutable exclusion writers, a topology-edit caller gap, and quadratic lookups.
All were corrected. Regressions failed before the fixes and passed after them.
Additional regressions cover full exclusion unions, exact-floor rounding, and tiny positive capacity tolerance.
Checks: 127 functional tests passed, zero failed. Nine manual benchmarks are ignored by default.
Formatting, strict Clippy, binary build, diff checks, and eight live UI routes passed.
Final debug results: 50k minimum allocation 288.4 ms; indexed 5,001-member scope 33.1 ms versus 2.515 s before indexing.
The 50k/8-WAN and 50k/100-profile pipeline workloads also passed.

The latest verified console now runs. Fresh UI collection returned 14 distro and 12 core tables without errors.
The latest candidate has 26 clients, split 12/14, with approximately 320 Mbps effective minimum totals per WAN/direction.
Router maxima, the configured floor, and the base minimum fraction remain unchanged.
Route findings are zero. Core membership remains Conflict: all 26 candidates overlap protected manual entries; 17 proposed WANs differ from current membership evidence.
No generation was published. End-to-end remains Unavailable without committed output. Do not claim that routing or shaping has been applied.
The private candidate pair passed the existing v3 preflight: topology validated, at least 26 circuits, pre-install exit 0 under write confinement.
LibreQoS `2.2.202608181454` remains held. No router configuration or LibreQoS change occurred. Only the LQoSync web process was restarted.

Changed modules: core_scopes, minimums, core_analysis, core_diagnostics, core_plan, core_review_web, routeros, model, orchestrator, validation, web, speed_web, policy_fixture, and lib.
Synthetic tests, sanitized actual fixture, guides, plan, and notes accompany the change. No dependency or settings schema change.
Private settings, credentials, raw inventory, and candidate reports must remain ignored.
Next: review protected manual membership and a per-host migration plan without writes or ownership bypass.
See [[10-sessions/2026-10-29-scoped-routing-minimums]]. Commit/push authorization continues. Verify remote head against the local tip.


## Core console preview verified; integrated writer pending

Updated: 2026-10-30T23:08:51+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `codex/core-console`. Worktree: `/tmp/lqosync-core-console`. Base: `b51af71`.
Integration target: `lqosyncv3` in `/opt/LQoSync3.0`.
Goal: connect health and reconciliation previews to the scheduler and modal controls.
Changed files: core_console, core_control_web, model, scheduler, RouterOS optional reads, navigation, modal recognition, tests, guides, plan, and notes.
Automatic mode rejects. No writer caller, production verifier, or lease upgrade exists.
The preview discards superseded results and cleanup receipts after incomplete observations.
Health-only ticks preserve the ordinary publication interval. Caller cancellation cannot strand busy state.
Eight focused console tests passed. Format and strict Clippy passed. Final full-suite result: 287 passed, zero failures, 12 ignored.
Five-width Chromium checks passed. Fresh review found no remaining material defect.
No live router reads/writes, production settings, shaping outputs, credentials, service changes, or restart occurred.
Next: complete Task 4 writer coordination, live verifier, journal controls, and interruption tests.
Keep automatic writes disabled until isolated RouterOS acceptance and concrete rollout authorization exist.
Preserve the separate frontend planning edits in the main checkout.
See [[10-sessions/2026-10-30-core-console-preview]] and `v3/CORE_CONTROL.md`.


## Offline worker and verifier integration; lab acceptance pending

Updated: 2026-10-31T06:14:13+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `codex/core-worker`. Worktree: `/home/pi/lqosync-core-worker`. Base: `b6efb03`.
Target: `lqosyncv3` in `/opt/LQoSync3.0`.
Goal: connect fresh verification to bounded execution with code and mock tests only.
Changed files: core_worker, helper extraction in core_console/sessions/wan_health, module export, tests, guides, ledger, and notes.
RouterSource reads settings and actual routers. The verifier checks canonical root, all selected sessions/rates, health, policy, approval, and complete actual rows.
The exclusive offline lease remains unchanged. No automatic, HTTP, CLI, or scheduler caller exists.
Fresh review found no concrete defects. Final full-suite result: 295 passed, zero failures, 12 ignored.
The existing 50,000-session workload passed. Debug times: collect 592.8 ms, assign 854.9 ms, retain 1.062 s, candidate save 1.331 s, candidate load 2.448 s.
This measures the synthetic session pipeline, not end-to-end worker throughput or a paired baseline comparison.
No live router reads/writes, production settings, shaping output, package changes, service changes, or restart occurred.
Next: online console coordination, journal controls, recovery approval, and separately authorized isolated RouterOS acceptance.
The operator has no lab available. Do not replace lab evidence with mock results.
Preserve the separate frontend planning edits in the main checkout.
See [[10-sessions/2026-10-31-core-worker]] and `v3/CORE_WORKER.md`.


## Shadow controls complete; online mutation coordination pending

Updated: 2026-10-31T06:35:36+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `codex/shadow-controls`. Worktree: `/home/pi/lqosync-shadow-controls`. Base: `6921171`.
Target: `lqosyncv3` in `/opt/LQoSync3.0`.
Goal: add shadow pause and read-only journal controls with code and mock tests only.
Changed files: core_coordination, core_console, core_control_web, core_recovery, core_recovery_web, core_writer validation extraction, web/lib, viewer binary, tests, guides, and notes.
The shadow lock serializes work across processes. Local tickets invalidate on pause. Persisted settings checks reject external policy drift.
The separate GET-only viewer can inspect saved records while the normal console remains blocked by its unchanged recovery fence.
Approval, resume, rollback, and marker removal have no HTTP action. The offline writer still requires an exclusive lease.
Fresh review found two defects. Both received failing regression tests, fixes, and passing checks.
Final checks: format passed, 308 tests passed, zero failures, 12 ignored, strict all-target Clippy passed.
Five-width Chromium checks passed with synthetic records. Documentation links and diff checks passed.
No live router access, production settings, shaping output, package changes, service changes, or live restart occurred.
Next: actual online mutation coordination and separate recovery grants, then isolated RouterOS acceptance before production authorization.
No lab is available. Mock tests cannot replace lab acceptance or prove uninterrupted traffic.
Preserve the separate frontend planning edits in the main checkout.
See [[10-sessions/2026-10-31-shadow-controls]] and `v3/SHADOW_CONTROLS.md`.


## Coordinated worker library verified; runtime activation remains disabled

Updated: 2026-10-31T06:47:37+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `codex/coordinated-worker`. Worktree: `/home/pi/lqosync-coordinated-worker`. Base: `3d4e973`.
Target: `lqosyncv3` in `/opt/LQoSync3.0`.
Goal: implement coordinated execution with code and mock tests without destructive live MikroTik changes.
Changed files: core_coordination, core_worker, core_writer, core_console helper visibility, web settings replacement, settings handlers, tests, guides, ledger, and notes.
The explicit library open holds a shared console lease and the exclusive shadow slot. Offline and maintenance opens remain exclusive.
Grants bind to root, policy revision, batch, epoch, and lease lifetime. New coordinated sessions cannot resume old journals.
Pause and successful settings replacements revoke prior grants. Every production settings replacement uses the common setter.
Fresh review found missing revocation in ordinary settings handlers. Its regression failed before the fix and passed afterward.
Mock protocol cases cover known replies, exact rollback, lost replies, and cancellation during an in-flight request.
Final checks: format passed, 314 tests passed, zero failures, 12 ignored, strict all-target Clippy passed.
Documentation links and diff checks passed. The STE heuristic received manual review for technical terms.
No UI layout or dependency changed. No new browser test or performance benchmark ran for this backend stage.
No live router access, production settings, shaping output, package change, service change, or restart occurred.
Next: distributed controls, recovery approval, and separately authorized isolated RouterOS acceptance before any runtime activation.
Local epochs require the same coordinator. They do not deliver pause across console processes or recall an in-flight request.
No scheduler, CLI, HTTP action, or accepted automatic mode enables this library. No lab is available.
Preserve unrelated frontend planning edits in the main checkout.
See [[10-sessions/2026-10-31-coordinated-worker]] and `v3/CORE_WORKER.md`.


## Durable pause and recovery assessment verified; execution approval remains blocked

Updated: 2026-10-31T07:03:59+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `codex/distributed-controls`. Worktree: `/home/pi/lqosync-distributed-controls`. Base: `6500d83`.
Target: `lqosyncv3` in `/opt/LQoSync3.0`.
Goal: implement cross-process cancellation and read-only recovery assessment with no destructive live changes.
Changed files: core_shared_control, coordinator, writer assessment and pure history helper, store settings transaction, web initialization, core policy persistence, recovery display, tests, guides, ledger, and notes.
Shared generations and inhibit state revoke old work. Failed saves and lost initialized state remain paused. Ordinary saves cannot clear an inhibit.
Explicit Preview save compares persisted settings before it resumes controls. All old grants remain revoked.
The durable control-lock anchor distinguishes first initialization from state loss. Do not delete state or locks to force progress.
Fresh review found three defects. Their regression checks failed before fixes and passed afterward.
Actual child-process pause and separate-console worker revocation passed. Recovery assessment rejects unknown outcomes, stale evidence, changed context, and incomplete membership.
Assessment is not an execution grant. The viewer remains GET-only and shows exact saved IP/list scope without a live verification caller.
Final checks: format passed, 326 tests passed, zero failures, 12 ignored, strict all-target Clippy passed.
Five-width Chromium checks passed with a synthetic pending journal. Documentation paths, links, and diff checks passed.
No performance benchmark ran. No live router access, production setting, shaping output, package change, service change, or restart occurred.
Next: persist the worker guard for reviewed recovery approval, then separately authorized isolated RouterOS acceptance before runtime activation.
No lab is available. No HTTP, CLI, scheduler, or accepted automatic mode enables writes.
Older binaries, other hosts, and external router writers do not participate in these controls. A pause cannot recall an in-flight request.
Preserve unrelated frontend planning edits in the main checkout.
See [[10-sessions/2026-10-31-distributed-controls]] and `v3/DISTRIBUTED_CONTROL.md`.


## Persisted worker guard and reviewed recovery verified

Updated: 2026-10-31T07:31:11+08:00 (Asia/Shanghai). Last agent: Codex.
Branch: `codex/recovery-guard`. Worktree: `/home/pi/lqosync-recovery-guard`. Base: `a366cd8`.
Target: `lqosyncv3` in `/opt/LQoSync3.0`.
Goal: persist original worker context and require exact reviewed recovery approval with code and mocks only.
Changed files: core_worker and recovery module, writer, coordination, artifact reader, migration fence, strict settings reader, worker tests, guides, ledger, and notes.
The worker saves a bounded private guard manifest before any send. Recovery review holds the existing exclusive fence and does not create recovery records.
Approval binds fresh evidence, exact artifact bytes, direction, nonce, shared generation, and a short deadline. It cannot extend the original forward deadline.
Unknown outcomes, changed sources, missing settings, lost state, and stale continuity remain blocked. Pause and lease drop revoke authority.
Final checks: format passed; 331 tests passed, zero failed, 12 ignored; strict all-target Clippy passed.
Seventeen focused worker tests passed, including loopback forward recovery and exact rollback.
A missing-settings regression failed before the strict reader fix and passed afterward. Fresh branch review and final-delta review found no concrete defect.
An intermediate full run had two existing scheduler mock timeouts during a concurrent build. The final full run passed with no concurrent build and unchanged test deadlines.
Documentation paths, links, and diff checks passed. No UI change, browser test, dependency change, or performance benchmark formed part of this stage.
No live router access, production configuration, shaping output, service change, or restart occurred. Private artifacts remain outside Git.
Next: separately authorized isolated RouterOS acceptance before runtime activation. No lab is available. No HTTP, CLI, scheduler, or automatic mode activates this library.
Preserve unrelated frontend planning edits in the main checkout.
See [[10-sessions/2026-10-31-recovery-guard]] and `v3/RECOVERY_GUARD.md`.
