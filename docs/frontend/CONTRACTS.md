# Proposed browser and adapter contracts

Status: proposed contract for tasks F00–F16. No HTTP endpoint below exists at this baseline.
Names are exact planning interfaces. F00 records final dependency versions without changing these domain boundaries.

Implementation checkpoint: session and overview routes now exist. Most domain routes below remain proposed.
See [first slice](FIRST_SLICE.md) and [live traffic](LIVE_TRAFFIC.md) for the implemented subset.

## Implemented live monitor

`GET /api/v1/overview/live` requires the current session cookie.
It sends one `Overview` JSON document per second as a default server-sent message.
It reads the shared broker cache. Client connections do not trigger native source reads.
The gateway permits at most 16 simultaneous streams. Further connections receive HTTP 503.
Slow clients keep at most one snapshot. The stream has no event replay queue.

Before each event, the gateway checks session expiry and the account epoch.
On revocation, it sends `unauthorized` and closes the stream.
On a broker failure, it sends `unavailable` and closes the stream.
These two control messages are plain text. Other messages contain typed `Overview` JSON.
The browser closes the stream on sign-out or page concealment.
It reconnects on return and uses the existing overview read as a fallback.

`Overview.wan_capacity` is optional. It contains `down_bps`, `up_bps`, `wan_count`, and `registry_revision`.
The service publishes this safe summary in `status.json`.
Only current status evidence permits a capacity claim.
The rates are true configured WAN bases, before shaping factors.
They do not prove link health or traffic distribution.
Traffic rates remain observed Mbps. `Point.gap_before` prevents a line across an unavailable observation.

## DPDK monitoring projection

`Overview.dpdk` is optional. Older responses can omit it.
The view contains a server timestamp, independent source stamps, worker summaries, pressure gauges, and bounded NIC diagnostics.
The broker reads `worker-utilization.json` and `nic-telemetry.json`.
It does not open a DPDK socket or start a collector.

Worker analytics use a six-second freshness limit.
Bridge health and NIC telemetry use separate fifteen-second limits.
The browser anchors source ages to `sampled_at` and elapsed time after receipt.
A different device clock must not create stale health.

All validated workers contribute to the mean and total.
The response includes at most sixteen busiest workers and eight ports.
It includes at most sixteen NIC xstats per port.
Optional details have a sixteen-KiB budget; `details_capped` reports trimming.
Counters cross the browser boundary as decimal text to preserve large integer values.
Reported zero, missing data, counter reset, and stale evidence remain distinct.
See the [DPDK design and implementation plan](DPDK_DASHBOARD.md).

## Common Rust types

Place serializable types in `crates/web-contract/src/`.
Use typed enums and Serde. Do not send arbitrary `serde_json::Value` documents to the browser as domain models.

| Type | Required fields or behavior |
|---|---|
| `SnapshotMeta` | `schema_version`, opaque `snapshot_id`, `captured_at`, `sources: Vec<SourceStamp>`, `capabilities` |
| `SourceStamp` | `source`, `observed_at: Option<u64>`, `updated_at: Option<u64>`, `state`, optional `pid`, `cycle_seq`, `revision`, safe `error_code` |
| `Page<T>` | `meta`, `items`, `next_cursor`, `total: Option<u64>` |
| `PageQuery` | `limit`, optional cursor, allowlisted sort and filters; default 50, maximum 200 |
| `SubscriberId` | Opaque service-backed identity; never an IP, display name, or table index |
| `SubscriberView` | Identity and source scope, current endpoints, state, effective speed/source, local plan, shaping parent, freshness |
| `RouterView` | Opaque ID, display name, source role, collection state, counts, age; no password or credential contents |
| `PlanView` | Stable ID, local name, down/up Mbps, membership count, policy revision |
| `PublicationView` | Desired revision, candidate, write receipt, import and take-effect states, observation time, bounded issues |
| `ActionRecord` | ID, idempotency key, actor, kind, request digest, expected revision, state, timestamps, safe result or error, audit ID |
| `ApiError` | Stable code, safe message, request ID, optional field errors/current revision/action ID; no raw internal error chain |

Use integer timestamps in UTC on the wire. Display the configured timezone explicitly.
Default the operational timezone to `Asia/Manila` where existing schedule policy requires it.
Use Asia/Shanghai for documentation timestamps unless the owner selects another timezone.

Use integer rate units in contracts and labeled Mbps in the UI. Do not roundtrip policy through displayed float values.
Preserve existing whole-Mbps restrictions of plan operations until a separate domain change expands them.

## Snapshot consistency

The broker builds an immutable read projection from approved sources.
Check file size, schema, regular-file status, and file identity before and after a read.
Check PID/cycle evidence where available. Retry a bounded number of times if inputs change.

If sources disagree, return explicit partial/stale evidence rather than a falsely coherent snapshot.
One source's revision does not establish another source's freshness.
Never load raw private snapshots into browser DTOs.

Use a snapshot token for pagination. A cursor binds snapshot, sort, filters, and last stable key.
Keep at most two table snapshots for 60 seconds initially, with a configured byte ceiling.
Return `snapshot_expired` when a client outlives retention. Restart its query without mixing pages.

Do not parse or reindex an unchanged source on every HTTP request.

## Web identity and persistence

Use `POST /api/v1/session` for login and `DELETE /api/v1/session` for logout.
Use `GET /api/v1/session` for the safe current-principal view.
Login has Origin checks, strict content type, rate limits, and no cacheable response.
Session creation rotates credentials. Logout revokes the server-side session.

The local CLI and broker own a root-only `web-accounts.db` for account records.
The gateway owns `web-auth.db` for sessions, but cannot write account records.
The broker verifies credentials and current account version through typed private IPC.

Every protected request checks current account state/version before authorization.
The broker owns `web-actions.db` for privileged action admission and results.
The gateway reads broker actions through typed messages, not direct access to the privileged database.
CLI account administration audit belongs to the root-owned account store.

Domain mutation audit belongs to the broker store.
Audit queries identify the source store and use stable IDs. Do not assume one cross-store transaction.

## Initial reads

All domain paths use `/api/v1`. Every domain read requires an authenticated session. The safe public branding read is defined separately below.

| Method and path | Result | Adapter source | Task |
|---|---|---|---|
| `GET /overview` | `OverviewView` | Status, collection, health, receipt projection | F06 |
| `GET /traffic?window=15m|1h|6h` | `TrafficSeries` with source and sample interval | Bounded telemetry cache; unavailable windows are explicit | F06 |
| `GET /subscribers` | `Page<SubscriberView>` | Canonical presence and policy projection | F07 |
| `GET /subscribers/{id}` | `SubscriberDetail` | Same identity and snapshot as list | F07 |
| `GET /routers` | `Page<RouterView>` | Safe config and collection projection | F08 |
| `GET /routers/{id}` | `RouterDetail` | Fixed source scope, collection error/age | F08 |
| `GET /plans` | `Page<PlanView>` | `topology.policies` projection | F11 |
| `GET /topology/tree` | `TopologyTreeView` | `topology.tree` result | F12 |
| `GET /topology/policies` | `PolicyView` | `topology.policies` result | F12 |
| `GET /topology/assignments` | `Page<AssignmentView>` | Fresh cached resolved topology projection | F13 |
| `GET /libreqos` | `PublicationView` | Receipts, import and take-effect evidence | F09 |
| `GET /libreqos/issues` | `Page<ShapingIssue>` | Fixed diagnostic snapshot; includes receipt/source | F09 |
| `GET /libreqos/preview?format=csv|tree|native` | `PreviewPage` | Bounded approved output projection | F09 |
| `GET /workers` | `WorkerView` | Existing telemetry caches | F10 |
| `GET /journal` | `Page<JournalEntry>` | Fixed unit allowlist and redaction | F10 |
| `GET /actions/{id}` | `ActionRecord` | Durable actor-authorized action ledger | F17 |
| `GET /settings` | `GlobalSettingsView` | Application-wide fields only | F15 |
| `GET /audit` | Paged redacted operational audit records | Authorized audit projection; no account CRUD | F14 |

Use `source_kind=ppp|dhcp|hotspot` for Subscriber filters.
Expose only supported windows and targets through capabilities.
Keep raw configuration, arbitrary paths, secret references, and internal debug messages out of responses.

## Actions

All initial authenticated accounts have the `superadmin` role.
Superadmins can perform supported operational actions, subject to domain validation.
Account creation, rename, password change, disable, enable, and deletion require the privileged local CLI.
No HTTP method exposes account CRUD, even to a signed-in superadmin.
UI visibility never replaces authorization on the server.

| HTTP action | Service operation or missing boundary | Result |
|---|---|---|
| `POST /plans`, `PATCH /plans/{id}`, `DELETE /plans/{id}` | `plan.create`, `plan.update`, `plan.delete` | Durable action plus resulting policy revision |
| `PUT /subscribers/{id}/plan` | `subscriber.plan` with server-resolved identity | Durable action plus revision |
| `POST /topology/proposals` | `topology.edit.propose` | Action, proposal ID, diff, deadline and reviewed revision |
| `POST /topology/proposals/{id}/confirm` | `topology.edit.confirm` | Action plus saved policy revision |
| `DELETE /topology/proposals/{id}` | `topology.edit.cancel` | Action plus cancellation result |
| `POST /topology/refresh-jobs` | `topology.preview.refresh` | Bounded job, no implicit browser-page scan |
| `POST /topology/discovery-jobs` | `wan.discover` or `topology.sources.candidates` | Explicit allowlisted job and observations |
| `PATCH /settings` | Typed application-wide settings patch only | Digest conflict or saved/reloaded result |
| Local `lqosync3-web-admin user ...` commands | Root-owned account store; no HTTP adapter | Audited account CRUD and credential-version update |

Pin and rebalance actions are typed variants of a topology proposal, not arbitrary operation strings.
Do not expose `policy.commit`, filesystem writes, CLI commands, or systemd commands through a generic proxy.
Do not implement the pictures' router-write or Sync-all buttons by invoking undocumented commands.

## Screen-owned settings contracts

Apply [SCREEN_SETTINGS.md](SCREEN_SETTINGS.md) to all configuration reads and writes.
Global `/settings` excludes PPPoE, DHCP, Hotspot, router connection, plan, topology, and LibreQoS fields.
Reject out-of-scope fields on the server. Do not silently discard them or write them through a global patch.

Proposed scoped reads and patches use these exact API paths under `/api/v1`:

| Scope | Read and patch path | Result or existing domain boundary |
|---|---|---|
| PPPoE/DHCP/Hotspot source | `/routers/{id}/sources/{source_kind}/settings` | `SourceSettingsView` / `SourceSettingsPatch`; source_kind is `ppp`, `dhcp`, or `hotspot` |
| Router connection | `/routers/{id}/settings` | `RouterSettingsView` / `RouterSettingsPatch`; credentials remain write-only |
| LibreQoS | `/libreqos/settings` | `LibreQosSettingsView` / `LibreQosSettingsPatch` |
| Application-wide | `/settings` | `GlobalSettingsView` / `GlobalSettingsPatch` |
| Local plans | Existing `/plans` paths | Existing typed plan operations and expected revision |
| Topology/WAN | Existing topology proposal paths | Existing typed edits and proposal confirmation rules |

Use GET for safe scoped reads and PATCH for scoped configuration changes.
Configuration patches carry `expected_config_digest` and enter the durable action path.
Read models include the reviewed digest, editable capabilities, safe values, and read-only reasons.
Scope IDs map to configured source identities on the server, not arbitrary file paths.
All configuration scopes share the service transaction and conflict boundary when they use one physical file.

These adapters remain future work in F18/F15. Do not imply that distinct source screens create distinct configuration authorities.
The shared header theme preference does not use these operational endpoints or start a backend job.

## Company identity contract

Apply [COMPANY_BRANDING.md](COMPANY_BRANDING.md). `WebBranding` is gateway-owned presentation state with a separate `branding_revision`.
`BrandingView` contains only the saved company name, optional generated asset reference, and branding revision.

`GET /api/v1/branding` is public so sign-in can show company identity without exposing the private settings response.
Logo drafts and `PATCH /api/v1/settings/branding` require superadmin authentication, CSRF protection, and expected branding revision.
The upload path is `POST /api/v1/settings/branding/logo-drafts`. Its 2 MiB limit is separate from the ordinary 64 KiB JSON mutation ceiling.

The gateway validates, decodes, and bounds image work. It does not forward logo data to the privileged broker or sync service.
An explicit branding save commits the validated name/logo reference. It does not claim a config reload or native apply.

## Mutation durability and conflicts

Every mutation requires session authorization, same-origin/CSRF checks, and an `Idempotency-Key`.
Bind the key to actor, operation, body digest, and reviewed revision.
Record intent durably before dispatch. The same key/body returns the same action ID.

The same key with a different body returns a conflict.
Proposal confirmation also binds its actor and domain to the reviewed proposal.

Action states are `queued`, `running`, `succeeded`, `rejected`, `failed`, and `outcome_unknown`.
Return HTTP 202 only after durable admission. Return an action ID for subsequent observation.
Do not mark a timed-out dispatched mutation failed unless the service proves rejection.

Reconcile unknown results through proposal state or resulting policy revision and expected affected data.
A revision change alone is insufficient proof that this action succeeded.
Do not claim exactly-once execution across service and web databases.

If the result cannot be established, keep `outcome_unknown` and require explicit recovery review.

Map bad input to 400, missing session to 401, forbidden action to 403, missing record to 404, and stale revision to 409.
Use 422 for domain validation, 429 for local admission limits, and 503 for service unavailability or a full service queue.
Preserve typed internal errors in private logs. Return redacted errors to clients.

Never retry a mutation automatically after dispatch. Reconnect reads with bounded backoff.

## Resource and privilege boundaries

Initial browser mutations have a 64 KiB body ceiling. Broker messages remain below the current 1 MiB service ceiling.
Initial query pages contain at most 200 rows. Raw bulk export requires a separate bounded streaming contract.
Use a job queue of 16 with one active broker mutation initially.

Use 120 seconds as the initial service-call ceiling, consistent with the existing client. Show queued/waiting state in the UI.
Close abandoned read requests. Do not cancel a dispatched write by closing its HTTP request.
Run read projection separately from the service cycle. Bound source refresh jobs and prevent duplicate refresh storms.
