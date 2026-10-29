---
type: handoff
created: 2026-10-29
tags: [lqosync-v3, handoff]
status: active
---

# LQoSync 3 handoff

Updated: 2026-10-29T19:20:50+08:00 (Asia/Shanghai).
Last agent: Codex.
Branch: `lqosyncv3`.
Checkout: `/opt/LQoSync3.0`.
Base: `f22cfbf8d974c10300a7a704cd7a712509d9827d` from `lqosync-in-rust`.

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
