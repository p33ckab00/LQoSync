# E02 — first web contract and G0 decision proposal

**Status:** First-slice route and DTO contract for W01/W02 planning. G0 choices below are recommendations pending owner evidence in [DECISIONS](../brain/DECISIONS.md). No HTTP API, store, auth service, or Leptos app exists in this checkout. Do not start W01 persistence as if G0 were accepted.

## Scope and source of truth

This contract covers the Dashboard summary, one paged subscriber table, and the LibreQoS missing-parent read model. It uses the existing LQoSync3 collector and shaping model without changing its files or RouterOS behavior. [E01](E01_BASELINE.md) supplies parity fixtures. [Routes](../docs/ROUTES_AND_COMPONENTS.md) supply browser placement. [The alert contract](../docs/MISSING_PARENT_ALERT.md) supplies warning behavior.

The current `src/status.rs` writes string alerts. `src/runner.rs` records the latest attempt time and a process-local `cycle_seq`, including failed attempts. Neither is a durable, typed diagnostic generation. W01 must add a persisted operational snapshot; W06 must produce typed native issues from validation. A failed probe must not erase the last completed native diagnostic or turn its count into zero.

## G0 decision record for owner review

| Decision | Recommended first slice | Alternative and trade-off | Evidence needed before acceptance |
|---|---|---|---|
| HTTP framework | **Axum** for a separate `/api/v1` server and Leptos CSR assets on the same origin. Keep collectors/jobs outside handlers. | **Rocket** offers route attributes and request guards. Run the same overview, issues, auth, and refresh spike if its guard model proves simpler. Neither framework choice proves runtime speed. | Build both minimal routes if the owner wants a comparison; record API p50/p95, idle/peak memory, error/streaming behavior, deployment complexity on the same fixture. |
| Durable store | **SQLite on local disk** for one active controller: migrations, bounded connection pool, WAL, explicit transactions, backup/restore test. Persist snapshots, issue generations, sessions, users, job/audit metadata; keep bounded retention. | External PostgreSQL supports shared writers/replicas, but adds a service and operations burden. In-memory plus JSON is smaller but complicates atomic restart-safe pagination, sessions, and audit. | Confirm one active controller, local filesystem, retention/backup needs, and expected write rate. SQLite WAL has one writer and cannot use network filesystem. |
| Rust database access | **SQLx SQLite** is a candidate for the async API and migrations; select its exact features after a compile-size and query-path spike. Keep DB operations out of hot collector loops. | `rusqlite` plus a dedicated blocking worker avoids an async SQL layer but requires careful queue/backpressure ownership. | Benchmark bounded read pages and write batches, check build footprint and shutdown behavior on target hardware. |
| Authentication/session | **Local accounts** with Argon2id password hashes and server-side, revocable sessions in the durable store. Same-origin HTTPS, `Secure`, `HttpOnly`, `SameSite=Lax` or `Strict` host-only cookie; rotate on login/privilege change, set idle and absolute expiry, `Cache-Control: no-store` for private responses. Add CSRF token and Origin check to every state-changing browser request; rate-limit login. Bootstrap the first admin through a one-time local CLI command with audit evidence. | OIDC can replace password handling if the deployment already has an identity provider; this requires a known issuer, recovery policy, and outage behavior. Stateless bearer tokens make immediate revocation and browser storage harder. | Owner confirms local accounts or existing IdP, remote-access/MFA requirement, and recovery path. Administrative remote access must not launch without an approved MFA control. |
| Roles | **viewer**, **operator**, **admin**. Server enforces each permission; frontend only reflects it. Default deny. | More granular roles can follow real operational duties; adding them now increases audit and test surface. | Owner confirms who may trigger LibreQoS apply, manage routers/secrets, and manage users. |

The Axum and Rocket capabilities are documented by [Axum](https://docs.rs/axum/latest/axum/) and [Rocket](https://rocket.rs/guide/v0.5/). [Leptos CSR](https://book.leptos.dev/getting_started/index.html) can use an existing server/API. The SQLite concurrency and filesystem limits are in [SQLite WAL](https://sqlite.org/wal.html). [SQLx SQLite](https://docs.rs/sqlx/latest/sqlx/sqlite/) documents its pool. Session, password, and CSRF controls follow [OWASP session](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html), [password](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html), and [CSRF](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html) guidance. These links establish capabilities and security requirements; the performance and deployment fit remain to be measured.

### Proposed permission matrix

| Capability | Viewer | Operator | Admin |
|---|---:|---:|---:|
| Read overview, subscribers, router status, LibreQoS status/issues | Yes | Yes | Yes |
| Run read-only router sync and shadow preview | No | Yes | Yes |
| Request LibreQoS apply after preview | No | Proposed; owner gate | Yes, after preview |
| Edit router connection or LibreQoS settings; manage credentials | No | No | Yes |
| Manage users, roles, sessions, MFA policy | No | No | Yes |
| WAN RouterOS writes | No | No | No in phase 2; G2/G3 gate |

G0 must resolve the operator apply cell and MFA policy before exposing write controls. A role change or disable revokes prior sessions. Every denied mutation returns 403 and records a bounded security event without secrets. No public operational read route.

## First-slice HTTP contract

These exact names and field meanings are the **E02 candidate contract** for W01/W02. W01 may implement them after G0 is accepted; only additive optional fields are compatible within `/api/v1`. Breaking changes require a new version or an explicit migration. JSON timestamps use RFC 3339 UTC; IDs, versions, and cursors are opaque strings, never derived from the runner's process-local `cycle_seq`.

| Route | Owner and consumer | Purpose |
|---|---|---|
| `GET /api/v1/overview` | W01 snapshot API → W02 `SummaryCards` | Small cached counts plus LibreQoS warning summary. No subscriber array, traffic claim, or health claim. |
| `GET /api/v1/subscribers?kind=ipoe|pppoe&limit=50&cursor=...` | W01 identity/page API → W02 `SubscriberTable` | Stable, filtered page from one snapshot. `kind` is required; cursor is optional. |
| `GET /api/v1/libreqos/status` | W01 snapshot API → W03 `/libreqos`, W02 summary | Target, mode, last attempt, last completed native diagnostic, write/read states. |
| `GET /api/v1/libreqos/issues?code=missing_parent&limit=50&cursor=...` | W01 page API, W06 typed producer → W06 `MissingParentList` | Typed, deterministic diagnostic page. W01 may return an explicit `unavailable` diagnostic state until W06 production is wired; never parse string alerts. |
| `POST /api/v1/auth/session` | W01 auth API → W02 login view | Verify credentials and approved MFA policy, create a new server-side session, set cookie. No session is issued before required factors pass. |
| `GET /api/v1/auth/session` | W01 auth API → W02 shell | Return `user_id`, `display_name`, `role`, `expires_at`, and a CSRF token for the active session; never return cookie/session secret. |
| `DELETE /api/v1/auth/session` | W01 auth API → W02 account menu | Revoke the session and clear its cookie; require CSRF and same-origin checks. |

The remaining endpoints in [the route map](../docs/ROUTES_AND_COMPONENTS.md) remain proposals for W03–W07. W01 serves the issues route with `diagnostic_state: unavailable`, null totals/generation, and empty items until W06 connects its typed producer; this route never invents a zero count. Other unimplemented panels do not get fake live values. All four operational reads require a valid viewer-or-higher session. All auth and operational responses use `Cache-Control: no-store`. Login accepts only same-origin JSON requests and rejects cross-origin/simple-form login attempts; authenticated mutations also require the session CSRF token. Exact MFA challenge fields remain open until the owner selects the policy, so the login wire DTO must be finalized with G0 before implementation.

### Shared field types

| Type | Required fields and meaning |
|---|---|
| `ReadState` | `fresh`, `stale`, `unavailable`. `fresh` requires a successful, time-bounded observation; `stale` retains last known evidence with its time; `unavailable` has no completed evidence. A failed latest attempt does not clear last known evidence. |
| `CountMetric` | `value: u64?`, `state: ReadState`, `observed_at: timestamp?`. `null` means unknown/not yet supported; zero means a verified empty count. |
| `SnapshotRef` | `snapshot_version: string`, `published_at: timestamp`. Durable version increments only after an atomic snapshot commit, survives restart, and is never reused. |
| `DiagnosticRef` | `diagnostic_generation: string?`, `diagnostic_observed_at: timestamp?`, `diagnostic_state: ReadState`. Its generation changes only after a completed native render, independently of the latest attempt or other snapshot updates. |
| `ErrorEnvelope` | `error: {code, message, request_id, current_version?}`. Message is safe for users; internal paths, credentials, SQL, or raw router replies stay in protected logs. |

`GET /overview` returns `snapshot_version`, `published_at`, `counts` (`ipoe_subscribers`, `pppoe_online`, `pppoe_offline`, `routers`, `libreqos_rows` as `CountMetric`) and `libreqos_warning` (`missing_parent_circuit_count: u64?`, `missing_parent_row_count: u64?`, `diagnostic_generation: string?`, `diagnostic_observed_at: timestamp?`, `diagnostic_state: ReadState`). The warning links to `/libreqos?issue=missing_parent` only when the **current selected target is `v3_network`** and its last completed native result has a positive count. On a target switch, the current warning is not applicable: counts and generation in this summary are `null`, and the old diagnostic remains in run history. Later traffic, WAN, and health panels use an explicit unavailable state until their producers exist.

`GET /subscribers` returns `snapshot_version`, `published_at`, `items`, and `next_cursor: string?`. Each `SubscriberItem` has `id` (stable opaque ID), `kind`, `display_name`, `circuit_id?`, `router_id?`, `ipv4?`, `mac?`, `profile_id?`, `online_state` (`online|offline|unknown`), `authorization_state` (`authorized|unauthorized|unknown|not_applicable`), and `observed_at?`. A missing IP or profile is `null`, not a fabricated string. The collector owns source identity and timestamps; the API owns pagination; the browser owns display only.

`GET /libreqos/status` returns `snapshot_version`, `published_at`, `target` (`v2_pair|v3_pair|v3_network|unknown`), `mode` (`shadow|apply|unknown`), `last_attempt_at?`, `last_attempt_state` (`succeeded|failed|none`), `last_attempt_error?` (safe summary), `write_state` (`none|candidate|blocked|written|import_pending|imported|unknown`), `data_state: ReadState`, `diagnostic_generation?`, `diagnostic_observed_at?`, `diagnostic_state: ReadState`, and `missing_parent` (`circuit_count: u64?`, `row_count: u64?`). The counts are `null` unless a completed `v3_network` diagnostic exists for the selected target. Do not label native v3 `imported` without accepted-generation readback. `written` alone does not imply import.

`GET /libreqos/issues` returns `snapshot_version`, `diagnostic_generation?`, `diagnostic_observed_at?`, `diagnostic_state`, `total_circuits: u64?`, `total_rows: u64?`, `items`, and `next_cursor`. When diagnostic state is unavailable, counts and generation are `null`, items are empty, and next cursor is `null`. Each `MissingParentIssue` has `code: "missing_parent"`, `circuit_id`, `circuit_name`, and `row_count`. A second issue code such as `conflicting_parent` gets its own filter and typed producer in W06. The cursor binds the query filter and **diagnostic generation**. It may continue across unrelated snapshot updates if the diagnostic generation remains retained. The subscriber cursor binds its kind, filter, sort and snapshot version in the same way. Encode and authenticate cursor contents or store them server-side; reject tampering. Sort issues by stable circuit ID, then a stable tie-breaker. Limit defaults to 50, valid range 1–100; reject invalid limits/cursors with 400. Retain bounded completed generations; if a cursor's generation was retired, return 409 `page_expired` so the UI restarts from page one.

Example after a failed probe, with a last completed native diagnostic still available:

```json
{
  "snapshot_version": "s-42",
  "published_at": "2026-11-05T10:05:00Z",
  "target": "v3_network",
  "mode": "apply",
  "last_attempt_at": "2026-11-05T10:04:59Z",
  "last_attempt_state": "failed",
  "last_attempt_error": "Native probe failed",
  "write_state": "blocked",
  "data_state": "stale",
  "diagnostic_generation": "d-7",
  "diagnostic_observed_at": "2026-11-05T09:55:00Z",
  "diagnostic_state": "stale",
  "missing_parent": { "circuit_count": 1, "row_count": 2 }
}
```

The API may show that this alert was last observed at 09:55. It must not claim that the 10:04 attempt confirmed it again. A later completed native render with no issue creates a new diagnostic generation with verified zero counts.

### Read, error, and refresh rules

- 401 `unauthenticated` for missing/expired session; 403 `forbidden` for a valid session without permission. Invalid query is 400 `invalid_query`; missing resource is 404 `not_found`; an unavailable read with no last known data is 503 `data_unavailable` only when the whole route cannot provide a meaningful typed state. A stale last known snapshot is 200 with explicit `stale` fields and observation time.
- W01 publishes an atomic version only after all included records and counts commit. Readers use one committed version. A newer failed collection updates attempt state without replacing completed observation/diagnostic values with zeros. On restart, load the last committed snapshot and mark it stale until a fresh collection succeeds.
- W01 sets a measured/configured freshness limit per source, not an invented fixed number. Browser refresh may poll the small summary with bounded interval and pause when hidden. It must ignore responses older than the current displayed snapshot and must not mount an entire 1k/10k list.
- Handler work is bounded: read persisted snapshots/pages or enqueue a job; it never opens a RouterOS session or runs a LibreQoS render. A deadline/DB error returns a typed error with request ID and leaves the last completed snapshot intact.
- An API response does not claim `imported` from file existence alone. W06 must define accepted-generation readback for each target before it can set that state.

## W01/W02 handoff and verification

1. **Coordinator/G0:** Record the owner's framework, store, auth/session, and role decisions in DECISIONS with the deployment assumptions. If a choice changes these fields, revise the contract before W01.
2. **Middle layer/W01:** Add versioned persistent snapshots and typed DTOs; implement the four operational reads, auth session routes, and pagination. Add migration/backup and restart paths. Keep the current status JSON and LibreQoS output compatible. The typed issue producer belongs to W06; until then, report diagnostic `unavailable` explicitly.
3. **Frontend/W02:** Build `SummaryCards` and one subscriber table against these exact fields. Show loading, stale, unavailable, empty, permission, and page-expired states. A card's warning count and list link must come from the same diagnostic generation.
4. **Verifier:** Use synthetic E01 fixtures. Check 0 versus unknown, one/many/multi-row missing-parent issues, failed probe after a positive warning, target switch, version-bound paging, restart, 401/403, malformed cursor, slow router, no secret in response, and bounded rows. Measure p50/p95 and memory on 1k/10k data before setting numeric budgets.

**Exit evidence:** accepted G0 decision record; serialized DTO examples and contract tests; W01 integration tests for restart, stale persistence, permissions, and pagination; W02 browser build and narrow-screen checks. This document alone supplies the implementation contract and decision proposal, not those results.
