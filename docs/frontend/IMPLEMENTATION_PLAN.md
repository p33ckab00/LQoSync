# Rust frontend atomic implementation plan

> For agentic workers: use `superpowers:executing-plans` to execute one task at a time. Use delegation only when authorized.

**Goal:** Deliver a Rust web frontend connected to the current LQoSync3 service, with explicit backend gaps.

**Architecture:** Leptos CSR uses typed HTTP reads and actions through an unprivileged Axum gateway.
A narrow local broker accesses approved service operations and source files.
The existing service retains policy validation, collection, revisions, and publication.

**Tech stack:** Rust, Leptos/Wasm CSR, Axum/Tokio, Serde, CSS, and SQLite for web state.
F00 selects compatible stable dependency versions and a reviewed password hash implementation.

**Spec:** [DESIGN.md](DESIGN.md), [CONTRACTS.md](CONTRACTS.md), [BACKEND_MAP.md](BACKEND_MAP.md), and [QUALITY.md](QUALITY.md).

## Global constraints

- Keep Rust application logic in both browser and server. Allow HTML, CSS, and generated Wasm binding glue.

- Keep the core Rust 1.89 contract. Record compatible web dependencies before implementation.

- Preserve the TUI, CLI, instance lock, current registry, and automatic publication semantics.

- Do not add a second Runner, collector, or shaping writer.

- Do not expose raw snapshots, arbitrary registry commits, filesystem paths, shell commands, or secrets to HTTP clients.

- Use current source as evidence. Recheck `9a465e7` assumptions against the execution revision.

- Default to 50 rows, maximum 200. Use at most 300 chart buckets, a 16-job queue, and one active mutation initially.

- Keep at most two table snapshots for 60 seconds within a combined 64 MiB ceiling.

- Read requests never start collection. Source refreshes use explicit bounded jobs.

- Reconcile uncertain writes. Do not retry them automatically or infer success from a revision change alone.

- Do not deploy to production as part of these implementation tasks without deployment authorization.

- Keep account CRUD CLI-only, allow multiple superadmins, and expose sign-in/sign-out only on the web.

- Use STE for persisted technical prose. Use the requested Google and Vercel skills for UI work.

## File ownership and interfaces

All `crates/` paths below are proposed. They do not exist at this documentation checkpoint.
Existing root modules remain the domain implementation.

| Path | Responsibility |
|---|---|
| `crates/web-contract/src/` | Browser-safe DTOs, error codes, paging, action types; no native dependency |
| `crates/web-gateway/src/main.rs` | Unprivileged HTTP entry point |
| `crates/web-gateway/src/bin/lqosync3-web-broker.rs` | Local privileged entry point; no TCP listener |
| `crates/web-gateway/src/broker/` | Peer checks, typed allowlist, bounded reads, service adapter |
| `crates/web-gateway/src/read/` | Immutable projections and safe queries |
| `crates/web-gateway/src/auth/` | CLI account client, web sessions, superadmin checks, CSRF |
| `crates/web-gateway/src/actions/` | Durable intent, dispatch, audit, and recovery |
| `crates/web-ui/src/` | Leptos shell, routes, components, forms, request state |
| `src/settings_service.rs` | Proposed framework-neutral settings transaction owned by the service |
| `tests/frontend/` | Browser and load fixture definitions; no live credentials |
| `systemd/lqosync3-web*.service` | Separate gateway and broker units, added only with deployment task |

Use shared contract types to prevent DTO drift.
Browser code depends on `web-contract`, never on the native `lqosync3` crate.
The broker can use the existing crate and service client. It must not construct a Runner.

## Task graph and release cuts

```text
F00 -> F01 -> F02 -> F03
F01 -> F04 -> F05
F03 + F05 -> F06, F07, F08, F09, F10
F02 + F04 -> F17
F07 + F17 -> F11
F03 + F05 + F17 -> F12 -> F13
F04 + F17 -> F14
F02 + F17 -> F18 -> F15
F06..F15 + F17 + F18 -> F16
```

Execute in this order: F00, F01, F02, F03, F04, F05, F06, F07, F08, F09, F10, F17, F11, F12, F13, F14, F18, F15, F16.
IDs stay stable even when a dependency executes earlier than its numerical position.

Read-only release: F00–F10 with authentication and no domain mutation controls.
Editing release: F17, F11–F14, F18, F15, then F16 proof.
D01–D04 remain separate domain projects in the backend gap register.

An isolated read-only release still needs the applicable F16 packaging and browser checks.

## Execution method

Each numbered task is one reviewable deliverable with its own commit and rollback boundary.
Each checkbox is one action. Split a step further if it takes more than one focused work interval.
Inspect connected callers before changing existing shared logic.

Record actual commands and outcomes in the task evidence. This documentation task did not execute the implementation commands.
Do not mark implementation, benchmark, or production proof complete from this plan alone.

## F00: Freeze dependencies and a measurable baseline

Dependencies: None. Status: not started.

**Files:** `docs/frontend/DEPENDENCIES.md`, `docs/frontend/evidence/F00.md`, `disposable build fixture outside production`.

**Interfaces:** Consumes the Rust 1.89 contract and Rust/Wasm requirement. Produces exact compatible crate/tool versions, licenses, artifact sizes, hardware profile, and confirmed budgets.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Record a red dependency/build case when a candidate fails compatibility. select a stable compatible release instead of raising the core toolchain silently.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Compile a minimal Leptos CSR fixture and Axum typed read against a fake service. Check native core compatibility and the Wasm dependency boundary.

- [ ] Run the checks below. Record each result separately.

Browser or fixture check: Run the fixture native and Wasm release builds.

Browser or fixture check: record compressed bytes, cold interaction, and a 1,000-row page measurement..

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `docs(frontend): freeze Rust web baseline`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: No production connection. Stop if Rust constraints cannot be met. document the exact incompatibility before feature work.

## F01: Create browser-safe contracts

Dependencies: F00. Status: not started.

**Files:** `Cargo.toml`, `Cargo.lock`, `crates/web-contract/Cargo.toml`, `crates/web-contract/src/{lib,read,action,error}.rs`, `crates/web-contract/tests/contracts.rs`.

**Interfaces:** Produces SnapshotMeta, SourceStamp, Page<T>, PageQuery, SubscriberId, ApiError, and ActionRecord from CONTRACTS.md.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test unknown schema, invalid page limits, missing evidence, integer rate roundtrips, and canary-secret exclusions.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Add only existing crate directories to the workspace. Keep the default core build behavior. Implement Serde DTOs, stable enums, page validation, and schema version handling.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-contract --test contracts`.

Run: `cargo check -p web-contract --target wasm32-unknown-unknown`.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): add browser contracts`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: Contract crate contains no collector, native socket, config, or secret type.

## F02: Add the narrow local broker and check service replies

Dependencies: F01. Status: not started.

**Files:** `crates/web-gateway/Cargo.toml`, `crates/web-gateway/src/bin/lqosync3-web-broker.rs`, `crates/web-gateway/src/broker/{mod,protocol,service}.rs`, `crates/web-gateway/tests/broker.rs`, `src/api/client.rs`.

**Interfaces:** Consumes typed BrokerRead and existing service protocol. Produces BrokerClient::read(request) -> Result<ReadReply, BrokerError>. Mutation dispatch stays disabled until F17.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test foreign UID, mismatched ID/version, arbitrary path/op, symlink replacement, oversized reply, absent service, and queue saturation.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Add gateway-UID peer checks, fixed source paths, operation allowlist, message bounds, and a private Unix socket. Check service response version and request ID before decoding.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-gateway --test broker`.

Run: `cargo test -p lqosync3 api::client`.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): add restricted service broker`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: Existing api.sock permissions remain unchanged. No generic proxy, shell execution, or second Runner.

## F03: Build immutable safe read snapshots

Dependencies: F02. Status: not started.

**Files:** `crates/web-gateway/src/read/{mod,snapshot,paging,subscribers}.rs`, `crates/web-gateway/tests/snapshots.rs`, `src/tui/presence.rs`, `proposed src/subscriber_view.rs`, `src/lib.rs`.

**Interfaces:** Produces SnapshotStore::capture() -> Arc<ReadSnapshot> and query(snapshot_id, PageQuery) -> Page<T>. Extracts shared identity/freshness projection without terminal drawing.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test mixed cycles, PID restart, missing/corrupt reports, duplicate IPs, identity change, stable paging, expired cursor, and memory ceilings. Bracket projection extraction with existing presence tests.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Read bounded approved reports, keep source stamps, reconcile PID/cycle evidence, index once per revision, and implement retained-snapshot cursors.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-gateway --test snapshots`.

Run: `cargo test -p lqosync3 tui::presence`.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): publish safe read snapshots`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: No config text or raw snapshot JSON crosses the broker. Keep stale evidence explicitly.

## F04: Add local authentication and sessions

Dependencies: F01. Status: not started.

**Files:** `crates/web-gateway/src/main.rs`, `crates/web-gateway/src/auth/{mod,store,session,csrf,accounts}.rs`, `crates/web-gateway/src/bin/lqosync3-web-admin.rs`, `crates/web-gateway/tests/auth.rs`.

**Interfaces:** Produces SessionPrincipal, superadmin authorization, CLI bootstrap, broker credential verification, and authenticated /api/v1 middleware.
The CLI owns account writes. The gateway owns sessions and reads account state through the broker.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test superadmin permissions, forged cookies, foreign origin, CSRF, disabled accounts, restart, and missing public bootstrap/account routes.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Add the unprivileged HTTP entry point and SQLite migrations.

- [ ] Separate root-owned account records from gateway-owned sessions. Add private broker credential and account-version checks.

- [ ] Add reviewed password hashing, session cookies, CSRF/Origin checks, revocation, login limits, and loopback defaults.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-gateway --test auth`.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): add superadmin sign-in sessions`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: Create crates/web-gateway/src/main.rs in this task. No domain route is public. Do not log passwords, CSRF values, or session tokens.

## F05: Deliver the authenticated Leptos shell

Dependencies: F01, F04. Status: not started.

**Files:** `crates/web-ui/{Cargo.toml,Trunk.toml,index.html}`, `crates/web-ui/src/{main,app,api}.rs`, `crates/web-ui/src/pages/login.rs`, `crates/web-ui/src/components/{shell,status,dialog}.rs`, `crates/web-ui/style/{tokens,layout}.css`, `tests/frontend/shell.md`.

**Interfaces:** Consumes SessionPrincipalView, SnapshotMeta, and ApiError. Produces AppShell, route outlet, safe request cancellation, and loading/error components.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Create browser cases for keyboard navigation, dialog focus, mobile drawer, 200% zoom, session expiry, and missing data without zero flashes. Retrieve current Google drawer/dialog guides first.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Build the login page and Leptos CSR shell with native controls and local fonts.

- [ ] Add routes, session expiry, responsive navigation, and shared state components.

- [ ] Run the checks below. Record each result separately.

Run: `cargo check -p web-ui --target wasm32-unknown-unknown`.

From `crates/web-ui`, run `trunk build --release`.

Browser or fixture check: execute tests/frontend/shell.md.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): add authenticated Rust UI shell`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: Use Vercel design review and browser screenshots. No fixture monitoring cards in production routes.

## F06: Connect Dashboard and bounded traffic

Dependencies: F03, F05. Status: not started.

**Files:** `crates/web-contract/src/overview.rs`, `crates/web-gateway/src/routes/overview.rs`, `crates/web-gateway/src/read/traffic.rs`, `crates/web-ui/src/pages/dashboard.rs`, `crates/web-gateway/tests/overview.rs`, `tests/frontend/dashboard.md`.

**Interfaces:** Produces GET /overview -> OverviewView and GET /traffic -> TrafficSeries from cached evidence.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test unknown versus zero, stale data without new ETag, different WAN capacities, absent attribution, 300-bucket ceiling, and no router reads during page requests.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Connect counts, freshness, issue links, health and bounded series. Share one snapshot across cards. Offer only available windows and proven traffic sources.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-gateway --test overview`.

Browser or fixture check: Wasm build.

Browser or fixture check: dashboard browser cases and PERF-01/PERF-05 traces.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): connect monitoring dashboard`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: Do not sum unlike WAN utilization percentages. A missing history store gives an explicit unavailable window.

## F07: Connect subscriber lists and details

Dependencies: F03, F05. Status: not started.

**Files:** `crates/web-gateway/src/routes/subscribers.rs`, `crates/web-ui/src/pages/subscribers.rs`, `crates/web-ui/src/components/subscriber_table.rs`, `crates/web-gateway/tests/subscribers.rs`, `tests/frontend/subscribers.md`.

**Interfaces:** Consumes PageQuery and canonical SubscriberView. Produces /subscribers list/detail with ppp, dhcp, and hotspot filters.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test changed IP/MAC, duplicate display names, offline versus stale, cursor expiry, long IDs, URL state, keyboard scrolling, and 10,000-record bounded DOM. Retrieve responsive-table guidance.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Add server sort/filter/paging, stable identity links, selected-record detail, safe copy, and panel-width responsive layout.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-gateway --test subscribers`.

Browser or fixture check: Wasm build.

Browser or fixture check: subscriber browser cases and DOM/payload measurements.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): connect subscriber views`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: No authorization or account-disable button until D02. No full inventory fetch for every keystroke.

## F08: Connect router inventory

Dependencies: F03, F05. Status: not started.

**Files:** `crates/web-gateway/src/routes/routers.rs`, `crates/web-ui/src/pages/routers.rs`, `crates/web-gateway/tests/routers.rs`, `tests/frontend/routers.md`.

**Interfaces:** Produces paged RouterView and safe RouterDetail from collection and allowlisted configuration fields.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test failed router versus offline router, missing observation, redacted credentials, changed source scope, and forbidden endpoint fields.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Show role, observed state, source counts, age, and collection errors. Link to filtered subscribers and relevant shaping evidence.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-gateway --test routers`.

Browser or fixture check: Wasm build.

Browser or fixture check: router browser cases.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): connect router monitoring`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: Do not expose Add/test/sync as working actions before F15/D04.

## F09: Connect LibreQoS evidence and diagnostics

Dependencies: F03, F05. Status: not started.

**Files:** `crates/web-contract/src/shaping.rs`, `crates/web-gateway/src/read/shaping.rs`, `crates/web-gateway/src/routes/libreqos.rs`, `crates/web-ui/src/pages/libreqos.rs`, `crates/web-gateway/tests/shaping.rs`, `tests/frontend/libreqos.md`.

**Interfaces:** Produces PublicationView, Page<ShapingIssue>, and bounded PreviewPage. Reuses receipts, import evidence, and TakeEffect results.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test newer blocked candidate with previous imported world, missing-parent circuit/row counts, waiting removals, late apply, stale evidence, and native/CSV target differences.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Project existing evidence with stable receipt/source correlation. Add bounded issue pages and selected-target preview without recomputing planner rules.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-gateway --test shaping`.

Run: `cargo test -p lqosync3 apply_evidence`.

Browser or fixture check: Wasm build.

Browser or fixture check: LibreQoS browser cases.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): expose shaping evidence`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: An imported file is not proof of dataplane application. No raw private snapshot download.

## F10: Connect Workers and Journal diagnostics

Dependencies: F03, F05. Status: not started.

**Files:** `crates/web-gateway/src/read/diagnostics.rs`, `crates/web-gateway/src/routes/diagnostics.rs`, `crates/web-ui/src/pages/{workers,journal}.rs`, `crates/web-gateway/tests/diagnostics.rs`, `tests/frontend/diagnostics.md`.

**Interfaces:** Produces WorkerView and Page<JournalEntry> with fixed service-unit allowlist and safe error codes.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test no analytics sampling from HTTP, arbitrary unit rejection, secret canaries, huge lines, unavailable journal, rollover, and paused selection.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Reuse bounded telemetry caches. Add a capped journal reader with argument-safe fixed unit selection, redaction, cursor expiry, and limited retention.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-gateway --test diagnostics`.

Browser or fixture check: Wasm build.

Browser or fixture check: diagnostics browser cases.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): connect worker and journal views`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: No arbitrary journalctl flags, shell interpolation, or unbounded live stream.

## F17: Add durable action admission and recovery

Dependencies: F02, F04. Status: not started.

**Files:** `crates/web-gateway/src/actions/{mod,store,dispatch,recovery}.rs`, `crates/web-gateway/src/routes/actions.rs`, `crates/web-gateway/tests/actions.rs`.

**Interfaces:** Produces admit(principal, key, TypedAction) -> ActionId and GET /actions/{id}. Broker-owned intent/result records persist before dispatch.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test duplicate keys, key/body mismatch, foreign actor, crash at each dispatch boundary, service commit with lost reply, full disk, and read-after-restart.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Add one mutation worker, queue limit 16, durable actor/body/revision binding, audit outcomes, service restart detection, and outcome_unknown reconciliation.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-gateway --test actions`.

Browser or fixture check: run crash-injection fixtures from temporary directories.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): track durable service actions`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: Gateway HTTP cancellation never resubmits or cancels an already dispatched write. Do not claim exactly-once execution.

## F11: Connect local plans and subscriber plan edits

Dependencies: F07, F17. Status: not started.

**Files:** `crates/web-gateway/src/routes/plans.rs`, `crates/web-gateway/src/broker/plans.rs`, `crates/web-ui/src/pages/profiles.rs`, `crates/web-ui/src/components/plan_picker.rs`, `crates/web-gateway/tests/plans.rs`, `tests/frontend/plans.md`.

**Interfaces:** Consumes typed plan CRUD and SubscriberId. Calls existing plan.create/update/delete and subscriber.plan with expected revision.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test stale revision, plan in use, invalid rates, changed subscriber scope, lost response, no RouterOS write, and preservation of unsaved edits.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Add local plan cards, membership links, integer rates, safe delete confirmation, and action result/readback. Resolve browser IDs to full domain identity server-side.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-gateway --test plans`.

Run: `cargo test -p lqosync3 runner::tests::api`.

Browser or fixture check: Wasm build.

Browser or fixture check: plan browser cases.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): connect local service plans`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: Replace pictured router-profile sync. Schedule controls remain unavailable until D01.

## F12: Connect topology Builder and proposal workflow

Dependencies: F03, F05, F17. Status: not started.

**Files:** `crates/web-gateway/src/routes/topology.rs`, `crates/web-gateway/src/broker/topology.rs`, `crates/web-ui/src/pages/topology.rs`, `crates/web-ui/src/components/{tree,node_form,publication}.rs`, `crates/web-gateway/tests/topology.rs`, `tests/frontend/topology.md`.

**Interfaces:** Consumes TopologyTreeView, PolicyView, TypedEdit, and ActionId. Uses `topology.tree`, `topology.policies`, and the proposal operations in CONTRACTS.md.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test illegal parent, stale proposal, expired deadline, canceled proposal, partial source, duplicate refresh admission, and unchanged service validation.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Add tree and parent picker, capacity/source information, narrow supported edit forms, proposal diff/deadline, and publication strip. Queue explicit discovery/refresh jobs.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-gateway --test topology`.

Run: `cargo test -p lqosync3 runner::tests::api`.

Browser or fixture check: Wasm build.

Browser or fixture check: topology browser cases.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): connect topology builder`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: No generic policy.commit route. Save semantics preserve automatic publication. no universal second Apply step.

## F13: Connect topology assignments, pin, and rebalance

Dependencies: F07, F12. Status: not started.

**Files:** `crates/web-gateway/src/read/assignments.rs`, `crates/web-gateway/src/routes/assignments.rs`, `crates/web-ui/src/pages/assignments.rs`, `crates/web-gateway/tests/assignments.rs`, `tests/frontend/assignments.md`.

**Interfaces:** Produces Page<AssignmentView> with desired/resolved shaping placement and evidence. Pin/rebalance use supported typed proposal edits.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test pinned capacity conflict, changed source identity, scoped versus full rebalance, stale revision, inactive manager, and no route-write claim.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Add assignment reasons, filters, scoped rebalance preview, pin/unpin and publication progress. Label placement independently of routing.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-gateway --test assignments`.

Browser or fixture check: Wasm build.

Browser or fixture check: assignment browser cases.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): connect topology assignments`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: Logical shaping assignment never becomes a routing-success badge without D02 observations.

## F14: Complete CLI-only superadmin account administration

Dependencies: F04, F17. Status: not started.

**Files:** `crates/web-gateway/src/bin/lqosync3-web-admin.rs`, `crates/web-gateway/src/auth/accounts.rs`, `crates/web-gateway/src/broker/auth.rs`, `crates/web-gateway/tests/accounts_cli.rs`, `crates/web-gateway/tests/account_routes.rs`.

**Interfaces:** Use the exact proposed commands in [CLI_ACCOUNT_POLICY.md](CLI_ACCOUNT_POLICY.md).
All accounts use the `superadmin` role. The CLI requires root and has no fixed superadmin count.
No account mutation function has an HTTP route.

- [ ] Test create, list, show, rename, password change, disable, enable, and delete through the CLI.

- [ ] Test multiple superadmins, duplicate names, hidden password entry, and rejection of account writes by a non-root process.

- [ ] Test final-active-account protection, concurrent changes, failed transactions, restart, and audit secret exclusion.

- [ ] Test immediate session rejection after credential/account changes and account-version verification failures.

- [ ] Run these cases before implementation and record the missing capability.

- [ ] Add transactional account CRUD, credential epochs, safe terminal prompts, and root-owned storage without changing file ownership during updates.

- [ ] Keep the browser limited to sign-in/sign-out. Do not create Users navigation, signup, invitation, password-reset, or account CRUD routes.

- [ ] Run `cargo test -p web-gateway --test accounts_cli` and `cargo test -p web-gateway --test account_routes` separately.

- [ ] Review the diff and evidence. Commit as `feat(web): manage superadmins through CLI`.

**Stop condition:** Root CLI manages multiple superadmins. The HTTP surface cannot create, edit, list, or delete accounts.
Operational audit reads remain redacted and separate from account enumeration.

## F18: Extract service-owned settings transactions

Dependencies: F02, F17. Status: not started.

**Files:** `src/settings_service.rs`, `src/lib.rs`, `src/api/dispatch.rs`, `src/runner.rs`, `src/tui/settings/{apply,validate,preview}.rs`, `src/runner/tests/api.rs`, `tests/settings_service.rs`.

**Interfaces:** Produces typed settings.get, settings.preview, settings.commit service operations with expected digest and redacted result. Extend the broker allowlist only for these types.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test concurrent TUI/web edit, changed digest, managed-field ownership, invalid config, secret rollback, interrupted write, failed reload, and output ownership preservation.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Bracket extraction with existing tests. Move reusable validation/transaction logic out of terminal code. Serialize commits with service ownership, backups, secret writes, and reload acknowledgement.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p lqosync3 --test settings_service`.

Run: `cargo test -p lqosync3 tui::settings`.

Run: `cargo test -p lqosync3 runner::tests::api`.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(api): add typed settings transactions`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: No browser filesystem paths or arbitrary TOML replacement. Preserve current TUI behavior and rollback boundaries.

## F15: Connect settings and safe router configuration

Dependencies: F05, F14, F18. Status: not started.

**Files:** `crates/web-gateway/src/routes/settings.rs`, `crates/web-ui/src/pages/settings.rs`, `crates/web-ui/src/components/settings_review.rs`, `crates/web-gateway/tests/settings.rs`, `tests/frontend/settings.md`.

**Interfaces:** Consumes SettingsView, expected config digest, typed SettingPatch, and reload evidence from F18.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Test managed-field read-only state, invalid field, canary secret, stale digest, dirty navigation, failed reload, and unsupported integration capability.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Add allowlisted forms, router connection fields where supported, write-only credential updates, diff review, and saved/reload result separation.

- [ ] Run the checks below. Record each result separately.

Run: `cargo test -p web-gateway --test settings`.

Browser or fixture check: Wasm build.

Browser or fixture check: settings browser cases.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `feat(web): connect safe settings`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: No activation/deactivation or implicit service restart. Router connection test jobs remain D04.

## F16: Check and package the connected releases

Dependencies: Applicable release dependency set above. Status: not started.

**Files:** `systemd/lqosync3-web.service`, `systemd/lqosync3-web-broker.service`, `tools/frontend/check.sh`, `.github/workflows/ci.yml`, `docs/frontend/DEPLOYMENT.md`, `docs/frontend/evidence/F16.md`.

**Interfaces:** Produces reproducible native/Wasm artifacts, isolated installation instructions, rollback procedure, and feature evidence matrix.

- [ ] Read the listed existing callers and linked specification. Record the execution baseline and owned files.

- [ ] Add tests: Run superadmin/CSRF and absent-account-route checks, crash recovery, redaction, read-only connector counters, 1k/10k load, cycle interference, and cross-browser/accessibility cases. Inspect actual screenshots.

- [ ] Run that case before implementation and record the expected failure or missing capability.

- [ ] Implement the smallest connected change: Package unprivileged gateway and restricted broker, local state permissions, TLS proxy contract, CSP/static assets, and test-only fixture configuration. Do not enable live mutations by installation.

- [ ] Run the checks below. Record each result separately.

Run: `cargo fmt --all --check`.

Run: `cargo clippy --workspace --all-targets --locked -- -D warnings`.

Run: `cargo test --workspace --locked`.

Run: `cargo +1.89 build -p lqosync3 --locked`.

Browser or fixture check: Wasm release build.

Browser or fixture check: documented browser/load commands.

- [ ] Review the diff and record evidence, limitations, and the next dependency. Commit as `build(web): package verified Rust frontend`.

**Stop condition:** The deliverable passes its focused checks and preserves this boundary: Install only on an isolated fixture deployment. Stopping web units must leave lqosync3.service and shaping output unchanged.

## Review and completion ledger

Create `docs/frontend/evidence/<task>.md` when that task executes.
Record source revision, files, dependencies, commands, failures, fixes, screenshots, and unresolved limits.
For each requirement, record implemented, compiled, tested, measured, and production-verified status separately.

The read-only and editing releases must publish a capability list that matches backend support.

Do not mark D01–D04 complete with disabled mock controls or simulated writes.
Their completion requires separate focused domain plans and the backend proof in BACKEND_MAP.md.
The current documentation commit does not execute any implementation checkbox above.
