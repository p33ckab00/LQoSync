# Routers screen implementation plan

Date: 2026-11-21 (Asia/Shanghai). Status: planning only. No task below has started.

**Goal:** Deliver router registration, a complete disable barrier, reviewed shaping removal, cached router screens, and a visible future WAN pinning surface.

**Architecture:** The core service owns router I/O, configuration, jobs, and publication evidence. The private broker exposes narrow typed operations. Axum and Leptos provide the authenticated web surface.

**Stack:** Existing Rust, Axum, Leptos, service IPC, and current session/CSRF mechanisms. Do not add React or replace the TUI.

**Specification:** [Routers design](ROUTERS_DESIGN.md).

For execution, use the installed `superpowers:executing-plans` workflow task by task.
Complete the initial slice R00–R07 first. R08–R09 are separate future backend packages, not implicit authorization for live RouterOS writes.

## Execution boundary

- This session writes plans and research notes only. Do not implement application code yet.

- Leave Dashboard, DPDK, NIC, worker-ring, and current browser-test changes to their current owner.

- Recheck the branch, diff, index, and handoff before future execution. The inspected HEAD was `c324203`.

- Coordinate shared shell integration after the Dashboard owner completes its work. Do not reset or stage another worker's changes.

- Keep all router registration records. No router delete command or endpoint belongs to this plan.

- Disabled blocks every remote operation, including discovery and readback.

- Include active-shaping removal in the disable preview and result. Keep its evidence separate from the router I/O state.

- The RouterOS write scope is core WAN pinning by address list. PPPoE/profile/account writes are excluded.

- Application IPs require a real DPI provider. A destination address alone does not identify an exclusive application.

- Web account management remains CLI-only.

## Review focus

1. A disable races with a retry, held session, WAN scan, or queued write. No new operation may pass the closed gate.

2. Disable succeeds but local publication fails. The router must stay disabled and the UI must show the incomplete shaping stage.

3. A core WAN router has no exclusive subscriber circuits, or identities have multiple sources. The impact preview must avoid unrelated removal.

4. Config, registry, or source membership changes after preview. Confirmation must reject the stale impact.

5. A DPI IP is shared, expired, NAT-translated, or from an unsupported address family. No broad or unverified pin may silently apply.

Each owning task below includes the corresponding test cases.

## Planned file map

Paths marked New do not exist yet. The implementer must confirm current module layout before adding them.
Existing files are evidence and integration points, not permission for a broad refactor.

| Responsibility | Planned files |
| --- | --- |
| Core control types and gate | New `src/router_control/mod.rs`, `gate.rs`, `tests.rs`; existing `src/config.rs`, `src/lib.rs`, `src/main.rs` |
| Core durable commands and registration | New `src/router_control/jobs.rs`, `store.rs`, `config.rs`; existing `src/api/dispatch.rs`, `src/runner.rs` |
| Safe inventory projection | New `src/router_control/view.rs`; existing `src/collection.rs`, catalog and policy-draft readers |
| Reviewed disable impact | New `src/router_control/disable.rs`, `disable_tests.rs`; existing publication/registry adapters |
| Router web DTOs | New `crates/web-contract/src/routers.rs`; existing contract module exports |
| Gateway and private IPC | New `crates/web-gateway/src/routers.rs`; existing `broker.rs`, `http.rs`, `lib.rs` |
| Rust UI | New `crates/web-ui/src/routers/{mod,inventory,detail,form,pinning}.rs`, `router-style.css` |
| Focused browser evidence | New `tools/frontend/routers-check.cjs`, `docs/frontend/evidence/routers/README.md` |
| Future DPI adapter | New `src/router_control/application_evidence.rs`; provider-specific source remains an open decision |
| Future WAN writer | New `src/router_control/wan_pinning.rs`, `wan_pinning_tests.rs` |

A router-owned stylesheet avoids edits to the Dashboard styles during initial construction.
Shell module exports, navigation, route handling, and stylesheet inclusion require a coordinated integration step in R06.

## R00 — Freeze the control and capability contract

**Dependencies:** approved design. **Deliverable:** typed states and a reviewed operation map.

**Files:** New core control module and `crates/web-contract/src/routers.rs`. Export them from the existing crate roots.

Proposed interfaces:

- `RouterAdminState`: Enabled, Disabling, Disabled, EnablePending.

- `RouterCapabilities`: cached inspection, local edit, test, source read, disable, enable, WAN pinning, application evidence, and explicit unavailable reasons.

- `RouterCommand`: create, patch, preview disable, confirm disable, enable, test, read. No arbitrary method or delete variant.

- `RouterJob`: stable ID, router ID, kind, actor, expected versions, stages, result, timestamps, audit reference.

- `RouterDisablePreview`: config digest, registry revision, published generation, impact hash, source/circuit changes, dependency warnings, expiry.

- `RouterSummary` and `RouterDetail`: administrative and observed state, source ages/counts, safe connection fields, credentials-configured flag, and job links.

- [ ] Write failing serialization/validation tests for invalid IDs, unknown operations, credential leakage, and unavailable capabilities.

- [ ] Run those tests and record the expected missing-contract failure.

- [ ] Add only the typed contract and validation. Keep remote operations unavailable.

- [ ] Run the focused tests. Check old overview/session DTO compatibility.

- [ ] Commit the contract and record which operations remain unavailable.

Suggested checks: `cargo test -p web-contract`; `cargo test router_control --lib --locked`.
**Stop condition:** no command string, delete operation, secret readback field, or false writer capability exists.

## R01 — Enforce router-wide administrative state

**Dependencies:** R00. **Deliverable:** a restart-safe no-I/O barrier across current read paths.

**Files:** Core gate/config modules; existing `src/config.rs`, `src/routeros.rs`, `src/runner.rs`, `src/mikrotik.rs` as needed.
Audit and adapt `src/topology_manager/coordinator.rs` and `wan.rs` too.

Proposed interfaces: `RouterIoGate::acquire(router_id, epoch, kind)` and `disable_and_quiesce(router_id, expected_epoch)`.
A permit owns cancellation and active-operation accounting. Recheck admission before each network command and retry.
A socket alone is not a permit.

Add a router-wide `enabled` setting with a compatibility default of true for existing configurations.
Web-created entries explicitly set it false. Do not replace protocol participation flags with this flag.
Restore the administrative gate before startup reads or job recovery.

- [ ] Write failing mock-connector tests for disabled startup, legacy polling, coordinator reads, WAN discovery, manual tests, held sessions, and retry admission.

- [ ] Add a race test where disable occurs between job admission and command dispatch. Count all connect/login/print operations.

- [ ] Add compatibility tests for configurations without the new field, mixed enabled/disabled routers, and TUI config round trips.

- [ ] Implement the shared gate and session invalidation. Keep failure isolated to the affected router.

- [ ] Run core router/collection/coordinator/WAN tests. Prove zero remote calls after the acknowledged Disabled boundary.

- [ ] Commit the barrier before exposing a Disable button.

Suggested check: `cargo test router_control --lib --locked`, followed by the affected RouterOS/coordinator suites.
**Stop condition:** changing one protocol flag cannot bypass router-wide disable. Cached inspection still works.

## R02 — Project cached inventory and safe details

**Dependencies:** R00–R01. **Deliverable:** bounded inventory/detail reads through the broker and gateway.

**Files:** Core view projection; router contract; new gateway routers module; existing broker/HTTP module registration.
Tests: New `crates/web-gateway/tests/routers.rs`.

Proposed interfaces: `RouterQuery { search, functions, admin_state, observation_state, cursor, limit }`, `RouterPage`, and `RouterDetail`.
Use immutable snapshot IDs. Default to 25 rows, cap at 50, and obey a 60-KiB target within the current 64-KiB broker reply ceiling.
If the byte budget stops a page early, resume from the last emitted item.

- [ ] Test missing config/catalog mappings, mixed functions, unknown counts, stale versus failed reads, and disabled cached detail.

- [ ] Test pagination while snapshots change, cursor mismatch, byte limits, and safe error strings.

- [ ] Assert zero connector calls for inventory, search, filters, detail, and activity reads.

- [ ] Implement only allowlisted fields from service-owned configuration and collection evidence.

- [ ] Add authenticated GET routes and narrow broker requests. Reject router mutations until their tasks exist.

- [ ] Run gateway tests and secret-redaction checks. Commit the connected read surface.

Check: `cargo test -p web-gateway --test routers --locked`.
**Stop condition:** a browser GET never starts collection and cannot fetch credentials, config documents, or filesystem paths.

## R03 — Add safe local registration and connection edits

**Dependencies:** R00–R02. **Deliverable:** durable local add/edit operations with reload evidence.

**Files:** Core control config/jobs/store modules; service dispatch; router DTOs and gateway adapter.
Reuse the behavior in TUI draft/apply/secrets code through a core boundary. Keep the terminal workflow unchanged.

Proposed service operations: `router.create`, `router.update`, and `router.job.get`.
Requests carry expected config digest and idempotency key. The server supplies actor identity from the authenticated session.
Keep request bodies within the current 4096-byte private-IPC limit or define and review a new bounded operation first.

- [ ] Test duplicate names/endpoints, invalid input, digest conflicts, credential replacement versus blank preservation, and lost-response retries.

- [ ] Test failures during secret/config persistence and service reload. Do not claim rollback without verification.

- [ ] Assert that create writes `enabled=false` and makes no remote call, even if source flags are selected.

- [ ] Implement narrow local config changes, owned secret storage, and a durable operation/audit record.

- [ ] Keep configured connection names immutable until a separate identity migration exists.

- [ ] Prove that local edits of a disabled entry make no remote calls. Run reload and existing TUI settings regressions.

- [ ] Commit the local registration capability. Keep Delete absent at every adapter layer.

**Stop condition:** config saved, service reloaded, and router healthy remain separate outcomes.

## R04 — Review disable impact and remove active shaping

**Dependencies:** R01–R03 and existing canonical publication/evidence adapters.
**Deliverable:** reviewed disable jobs with two independent result tracks.

**Files:** Core disable/jobs/store modules and tests; narrow dispatch/gateway routes.
Reuse the existing registry/compiler/publication path. Do not write a second CSV or native JSON producer in the gateway.

Proposed operations: `router.disable.preview`, `router.disable.confirm`, and `router.enable`.
The preview uses cached identity membership and published state. It does not contact MikroTik.
Confirmation binds config digest, registry revision, published generation, impact hash, and router epoch.

- [ ] Write failing tests for preview expiry, source membership change, stale revisions, safety-limit rejection, and concurrent config edits.

- [ ] Cover an access router with exclusive circuits, shared-source identities, a WAN-only core, and dependencies on a disabled WAN node.

- [ ] Prove that a core disable does not implicitly remove every subscriber assigned to that WAN.

- [ ] Prepare and validate the local candidate before mutation. Persist the reviewed intent and administrative disable before drain/publication.

- [ ] Record stages for admission closed, sessions drained, config/reload acknowledged, local candidate written, import confirmed, and apply verified.

- [ ] Test restart or failure between each stage. Recover the disabled gate first and never reconnect to finish readback.

- [ ] Test shadow mode, failed publication, and pending import/apply. Keep Disabled with Shaping removal pending/blocked when appropriate.

- [ ] Test enable: explicit action, no replay of old write jobs, and no direct restoration of stale cached circuits.
  Fresh validated source evidence follows the normal compiler/publication checks.

- [ ] Run publication, store recovery, and ownership regressions. Commit with evidence for both result tracks.

**Stop condition:** a successful disable response cannot imply shaping removal without publication evidence.
An unavailable membership mapping blocks an unreviewed removal; it does not authorize deletion of unrelated records.

## R05 — Add bounded connection-test and source-read jobs

**Dependencies:** R01–R04. **Deliverable:** explicit service-owned read operations with stable results.

**Files:** Core jobs/dispatch; gateway routers adapter and tests. Use the existing session and read-plan infrastructure.

Proposed operations: `router.test`, `router.read`, and `router.job.get`.
POST routes enqueue work and return a job ID promptly. Do not wait for a network scan inside an HTTP response or the broker exchange.

- [ ] Test disabled rejection, queue limits, duplicate requests, cancellation, and disable during a test or retry.

- [ ] Report TCP, TLS, login, and source-read failure separately. Do not substitute unreachable for every failure.

- [ ] Coalesce duplicate read jobs and bound one active remote job per router initially.

- [ ] Preserve ordinary collection cadence. A page refresh must not enqueue a job.

- [ ] Test restart and lost response recovery. Keep audit text free of credentials.

- [ ] Run focused connector and gateway tests, then commit the explicit read capability.

**Stop condition:** no disabled-router probe exception exists, including during Add router.

## R06 — Connect the Routers UI in four reviewable increments

**Dependencies:** R02–R05 and a coordinated shell integration point after Dashboard work.
Each increment has its own focused browser cases and commit.
Do not combine these increments into one frontend rewrite.

### R06a — Inventory and router route entry

**Files:** New router module entry, `inventory.rs`, router-owned stylesheet, and `tools/frontend/routers-check.cjs`.
Coordinate changes to `crates/web-ui/src/app.rs`, `main.rs`, navigation, and stylesheet inclusion with the current owner.

- [ ] Add synthetic empty, unavailable, stale, mixed-role, disabled, and paged inventory fixtures.

- [ ] Implement the semantic table, cached search/filter/paging, and quick inspection drawer from R02 data.

- [ ] Keep names, administrative state, and row actions available on narrow screens. Preserve filters and scroll across refresh.

- [ ] Make authenticated direct links under `/routers` work through the gateway fallback and asset routing.

- [ ] Test themes, 320px, keyboard access, 200% zoom, and zero remote collection from navigation.

- [ ] Save evidence and commit inventory. Keep unfinished form/job links unavailable.

**Stop condition:** the inventory is useful on its own and does not change Dashboard behavior.

### R06b — Router detail, sources, read operations, and activity

**Files:** New `detail.rs`, related gateway projections only if R02 needs a missing safe field, and focused browser cases.

- [ ] Implement Overview, Connection summary, Sources, Read and write, and Activity tabs on a stable router route.

- [ ] Connect Test connection and Read sources to R05 jobs. Project their exact backend states and observation times.

- [ ] Keep source-specific configuration on its owner feature page through scoped links.

- [ ] Test disabled rejection, authentication/TLS errors, stale evidence, queue limits, and pending jobs after refresh.

- [ ] Test direct links, browser Back, tab/scroll persistence, and disabled unavailable feature links.

- [ ] Commit the connected detail and operation views with focused evidence.

**Stop condition:** every active read control maps to a bounded core job. A page refresh starts no remote operation.

### R06c — Add router and local connection editor

**Files:** New `form.rs`, its route integration, and form browser cases. Use the R03 API.

- [ ] Implement the three-step Add router form and the local Connection editor.

- [ ] Save new entries as disabled. Show Enable as a separate explicit action.

- [ ] Support credential replacement without exposing values, secret-file paths, or secrets in browser persistence.

- [ ] Test validation, inherited defaults, digest conflict, unsaved edits during refresh, and lost response recovery by job ID.

- [ ] Prove local edits of a disabled entry make no remote calls. Do not add a test-before-save exception.

- [ ] Check keyboard flow, focus restoration, themes, mobile forms, and screen-reader errors. Commit the forms separately.

**Stop condition:** local registration succeeds without claiming that a connection was tested or healthy.

### R06d — Disable impact review and two-track result

**Files:** Router detail action/review component, existing job result view, and disable browser cases. Use the R04 API.

- [ ] Show exact circuit/source changes, shared identities to keep, WAN dependencies, and publication safety results.

- [ ] Confirm with the reviewed config digest, registry revision, impact hash, and router epoch.

- [ ] Show Disabling until quiescence. Keep the I/O result separate from Shaping removal pending/blocked/applied.

- [ ] Test stale-preview rejection, disable during a held job, publication failure, shadow mode, and a WAN core with zero direct subscriber circuits.

- [ ] Test enable without replay or immediate restoration of stale records. Fresh evidence follows the normal publication path.

- [ ] Verify the dialog's keyboard/focus behavior and keep input context after rejection. Commit review/result UI separately.

**Stop condition:** no disabled router has a usable remote action, and no UI success label outruns backend evidence.

### Shared validation after R06

Run `cargo test -p web-gateway --test routers --locked`, the Rust/Wasm build, and the focused router browser check.
Run shared-shell regressions only where the coordinated integration changed shared behavior.
Use `tools/frontend/build.sh` with the configured Trunk path. Save synthetic evidence under `docs/frontend/evidence/routers/`.
Do not modify Dashboard visual behavior as part of these increments.

## R07 — Include the future WAN pinning surface now

**Dependencies:** R06. **Deliverable:** visible WAN pinning structure with honest unavailable states.

**Files:** Router pinning UI module, capabilities projection, focused fixtures and browser cases.
No RouterOS writer or DPI provider is implemented in this task.

- [ ] Show the target core router/WAN, Subscriber IPs and Application IPs input groups, address-list scope, review, and result areas.

- [ ] Explain missing writer and missing DPI provider separately. Keep Apply and unsupported data selectors disabled.

- [ ] Show why an access-only router cannot currently supply a core WAN target.

- [ ] Test absent capability, disabled router, unsupported address family, and stale target evidence.

- [ ] Verify that no save/apply/test request occurs from a disabled field or future-feature panel.

- [ ] Check direct links, keyboard descriptions, themes, and mobile layout. Commit the visible future surface.

**Stop condition:** the UI exists, but neither routing nor persistence is simulated as successful.
R00–R07 finish the initial Routers slice. The next two tasks remain future backend projects.

## R08 — Define subscriber and DPI address evidence

**Status:** future backend package. Provider and policy review required before execution.
**Deliverable:** typed, fresh, attributable candidates for WAN policy preview.

**Files:** Core application-evidence adapter and tests; typed contract additions; pinning read projection.
Select the real DPI provider before naming a provider-specific client or endpoint.

Proposed `AddressCandidate`: address/family, subscriber or application identity, source, observation interval, expiry, confidence, and shared-address indicator.
A subscriber candidate also records the observed address as visible at the target core, including explicit NAT mapping where required.

- [ ] Prove subscriber identity and IP freshness without new reads from disabled routers.

- [ ] Define the DPI provider schema, ownership, retention, confidence, and revision contract.

- [ ] Test absent evidence, shared CDN IPs, expired addresses, subscriber IP reuse, NAT mismatch, IPv6 capability, and conflicting application labels.

- [ ] Bound candidates, batch sizes, and update frequency. Do not build a public-CDN IP scraper as a substitute for DPI.

- [ ] Publish typed evidence only. Do not perform RouterOS writes.

- [ ] Enable a selector only when its provider and target mapping are verified. Commit evidence support separately.

**Stop condition:** an observed application address is not represented as an exclusive application guarantee.
Subscriber-only pinning need not wait for a DPI provider once its own evidence contract is ready.

## R09 — Implement the future core WAN address-list writer

**Status:** separate future backend project. No production execution is authorized by this planning document.
**Dependencies:** shared control/job infrastructure, fresh address evidence, approved object ownership, and router-version/policy support.

**Files:** Core WAN pinning writer/tests; narrow dispatch/gateway adapter; pinning capability/result projection.

A preview must bind stable core/WAN IDs, source/destination selector semantics, observed addresses, list ownership, routing-policy binding, and expected revisions.
Use explicit AND semantics when both subscriber and application selectors are chosen unless a later reviewed policy defines another mode.

- [ ] Approve supported RouterOS versions, address families, existing mangle/routing-policy bindings, ownership markers, expiry, outage, and failback behavior.

- [ ] Test unowned rules, list-name collisions, stale previews, local/management traffic exclusions, and wrong core/WAN mapping.

- [ ] Implement idempotent changes to approved address-list objects only. Do not create broad route/mangle rules by inference.

- [ ] Test partial apply, connection loss, duplicate requests, disable during a write, and restart recovery.

- [ ] Use separate results for list membership readback and routing-policy/path verification.

- [ ] If a router becomes disabled, stop writes and readback. Record Unknown outcome without automatic retry.

- [ ] Verify object-by-object cleanup or compensation within the approved ownership contract. Never modify unrelated operator rules.

- [ ] Test subscriber-only, application-only, and combined selectors in an isolated RouterOS fixture or lab.

- [ ] Enable Apply only after capability and readback gates pass. Record lab evidence before any separately authorized live deployment.

**Stop condition:** a list change alone is not labeled a verified WAN routing change.

## Completion evidence

For each task, record its baseline, changed files, exact checks/results, unavailable capabilities, and the next dependency.
Use focused red/green tests for new backend logic. Run meaningful regressions after shared changes.
A mock screenshot, a file write, or a saved setting does not prove router I/O or shaping application.
Keep production credentials, subscriber identities, and packet captures out of synthetic screenshots.

The current planning session performs documentation and source checks only. It does not run implementation tests or claim these tasks complete.
