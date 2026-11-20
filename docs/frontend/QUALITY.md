# Performance, reliability, and integrity requirements

Status: acceptance requirements for future implementation. This document does not report runtime results.

## Performance

| ID | Requirement | Proof |
|---|---|---|
| PERF-01 | Page requests never read RouterOS or sample analytics sockets directly | Connector counters stay unchanged during HTTP read tests |
| PERF-02 | Parse and index once per source revision; share immutable projections | Parse counters and allocation profile across repeated reads |
| PERF-03 | Default 50 rows, maximum 200; bound DOM rows and payload | 1,000- and 10,000-record fixtures; payload and DOM measurements |
| PERF-04 | Keep stable paging during updates | No duplicate or skipped IDs in one snapshot; explicit expiry result |
| PERF-05 | Charts contain at most 300 buckets per series initially | Payload assertions, aggregation tests, and memory profile |
| PERF-06 | Pause hidden charts; cancel superseded reads | Background-tab, rapid filter, and navigation tests |
| PERF-07 | Bound queues, retries, jobs, and snapshot memory | Saturation, slow source, and eviction tests |
| PERF-08 | Keep sync cadence independent of browser load | Compare cycle interval and CPU with zero and 20 concurrent viewers |
| PERF-09 | Keep native dependencies out of Wasm | Dependency tree and release artifact inspection |
| PERF-10 | Serve compressed versioned assets and local fonts | Cold/warm traces, offline-font test, cache headers |

Use one gateway timer per active data set instead of one loop per card.
Start with five-second visible-page refreshes and one in-flight request per resource.
Use conditional reads where safe. Reevaluate stale status even when an ETag does not change.

Keep at most two table snapshots for 60 seconds, with an initial combined limit of 64 MiB.
Evict the oldest snapshot before admitting another that exceeds the limit.
Return an explicit resource error if one projection exceeds the ceiling.

Do not truncate policy evidence silently.

Initial measurement targets are proposals, not capacity promises:

- Cached read p95 at most 200 ms for 20 local viewers on recorded reference hardware.

- Browser filter response p95 at most 200 ms after input debounce and a local response.

- Compressed initial Wasm plus critical assets at most 4 MiB.

- Cold first usable view at most 3 seconds with a 10 Mbps link and 50 ms RTT on recorded hardware.

- Service cycle p95 interval regression below 5 percent against the same fixture and hardware.

- No sustained browser heap growth after ten minutes of refreshes and repeated route changes.

Record browser version, CPU, RAM, dataset, bandwidth, sample count, and measurement method.
F00 confirms feasible budgets before feature work. Record and explain any revised budget.
Do not describe unmeasured code as optimized.

## Reliability

| ID | Requirement | Failure proof |
|---|---|---|
| REL-01 | Keep last valid evidence and mark its age/state | Delete, corrupt, truncate, or replace one input report |
| REL-02 | Browser load cannot stall collection | Slow broker, full queues, disconnected viewers, busy service |
| REL-03 | Preserve unsaved edits | Background update, route guard, reload, conflict tests |
| REL-04 | Persist admitted actions before dispatch | Kill processes before admission, before dispatch, and after service acknowledgement |
| REL-05 | Never retry an uncertain mutation blindly | Timeout after service commit; reconcile action evidence |
| REL-06 | Recover sessions and audit state safely | Restart, disk-full, corruption, failed transaction, revocation tests |
| REL-07 | Detect incompatible server and browser versions | Old cached UI receives an explicit recovery message |
| REL-08 | Keep the TUI as is and preserve CLI behavior | Existing core suite and unchanged terminal regression checks |
| REL-09 | Web is the independent primary operator frontend | Complete supported workflows without a TUI process; synchronization continues after web shutdown |

Keep intent records and results atomic within each store transaction.
SQLite and the service registry do not share a transaction. Preserve an explicit unknown state at that boundary.
Do not rewrite shaping files, signal processes, or activate the manager from a read path.

Stop the web services for rollback without stopping synchronization.
A web outage must not require a TUI process to keep synchronization active.
Back up web state before migrations. A failed migration must not change the core registry.

## Integrity and access

| ID | Requirement | Proof |
|---|---|---|
| INT-01 | One service owns policy and publication | No second Runner or direct registry write from web code |
| INT-02 | Stable identity and reviewed revision bind every edit | IP/name change, concurrent edit, deleted target, expired proposal tests |
| INT-03 | Browser models contain no secret values | Canary secrets never appear in responses, URLs, logs, audit, or Wasm |
| INT-04 | Broker accepts only the configured gateway UID | Foreign UID, forged operation, arbitrary path, symlink, oversized-message tests |
| INT-05 | Authorize every route and mutation | Superadmin permissions, absent account CRUD routes, missing CSRF, foreign origin, expired session tests |
| INT-06 | Distinguish saved, written, imported, waiting, and applied | Held import, identity mismatch, waiting removal, late apply, stale evidence fixtures |
| INT-07 | Reject unknown versions and mismatched response IDs | Service-client protocol tests before adapter reuse |
| INT-08 | Protect persistent web state | Permission, backup/restore, lock contention, disk-full checks |

Use opaque server-side sessions in Secure, HttpOnly, SameSite cookies over HTTPS.
Do not store bearer credentials in browser local storage.
Require CSRF tokens and Origin validation for state-changing requests.

Disable broad CORS. Bind the gateway to loopback behind the approved HTTPS proxy by default.
Use a reviewed memory-hard password hash with per-password salts.
Create the first superadmin through the privileged local CLI. Do not ship a default password or public bootstrap route.

Reject existing sessions after password change, rename, disable, deletion, or credential-version change.
The broker checks account state/version for every protected request. Fail closed if that check is unavailable.
Test several simultaneous superadmins and reject removal or disable of the final active account.
Test that every account-management HTTP method is absent, including direct calls from an authenticated superadmin.
Restrict authentication retries and use safe, bounded audit records.

## Browser and design checks

Apply [THEMING.md](THEMING.md) to every route. Test Light, Dark, System, live environment changes, and persistent manual overrides.
Keep initial paint, native controls, chart colors, and focus states consistent with the selected theme.
Measure glass blur cost and provide a legible opaque fallback.

Use the requested Vercel skills for presentation and interaction review.
Use Google Modern Web Guidance before selecting HTML/CSS and browser API patterns.
Retrieve relevant current guides at implementation time and apply their browser fallbacks.

Test current stable Chromium, Firefox, and Safari/WebKit on desktop and phone layouts.
Use 1440×900, 1024×768, 390×844, and 320×640 as initial viewport fixtures.
Check 200 percent zoom, OS text scaling, reduced motion, keyboard-only use, and a screen reader.

Check text contrast of at least 4.5:1 and UI boundaries/focus contrast of at least 3:1.
Use 44 px touch targets and at least 40 px desktop action targets where density permits.
Keep table semantics and header relationships after responsive changes.

Use visible labels, field-specific errors, focus return, and safe confirmation focus.

Test drawer dismissal, dialog Escape behavior, long IDs, stale data, and permission failures.
Preserve numeric units and full values in copy/export actions.
Make sure chart legends and textual summaries convey data without color alone.

Run browser inspection and record screenshots. A successful Rust build is not browser proof.

## Screen ownership acceptance

Follow [SCREEN_SETTINGS.md](SCREEN_SETTINGS.md) for every feature.
Check the shared main-header theme selector on Dashboard and each feature route, also on phones.
Keep a feature form's draft, focus, and validation when the theme changes.

Check that PPPoE, DHCP, Hotspot, plan, topology, router connection, and LibreQoS fields do not appear as global editable settings.
Direct global or source-scoped patches with foreign fields must fail before any write.
Concurrent changes from different screens must share the backend digest/revision conflict boundary.
Do not split persistent configuration authority by screen.

## Release evidence

Record implemented, compiled, tested, measured, and production-verified status separately for each feature.
Run the Rust 1.89 core build, stable format/Clippy/tests, Wasm build, contract tests, and browser checks.
Record artifact hashes and the dependency lockfile revision.

Install first on an isolated fixture deployment. Production deployment remains a separate authorized action.
