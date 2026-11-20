# Backend connections and missing functions

Baseline: `9a465e7` on `lqosync3`, inspected on 2026-11-20.
Existing means source exists. It does not establish deployment or web availability.
Paths below refer to this repository. New paths in the implementation plan do not exist yet.

## Existing connections

| Area | Existing source | Reuse and boundary | Plan |
|---|---|---|---|
| Local transport | [client](../../src/api/client.rs), [server](../../src/api/server.rs), [protocol](../../src/api/protocol.rs) | JSON lines, socket `0600`, root/service UID, bounded messages; not HTTP | F02 |
| Service dispatch | [dispatch](../../src/api/dispatch.rs), [Runner](../../src/runner.rs) | Single service owner; current operations below | F02, F11–F13 |
| Router observations | [collection](../../src/collection.rs), [RouterOS](../../src/routeros.rs) | Sanitized observations, source states, cycle sequence; collection remains service-owned | F03, F08 |
| Subscriber identity and presence | [model](../../src/topology_manager/model.rs), [presence](../../src/tui/presence.rs) | Stable identities, source kind, freshness rules; extract shared projection, not terminal drawing | F03, F07 |
| Health and status | [health](../../src/system_health.rs), [status](../../src/status.rs), [TUI data](../../src/tui/data.rs) | Multiple source reports; build bounded browser projection | F03, F06 |
| Plans and speeds | [catalog](../../src/topology_manager/catalog.rs), [rates](../../src/topology_manager/rates.rs) | Local plans, inheritance, limits, factors; not complete RouterOS profile management | F11 |
| Topology edits | [edits](../../src/topology_manager/edits.rs), [TUI topology](../../src/tui/topology/mod.rs) | Builder, pin, rebalance, proposal confirmation; preserve service validation | F12, F13 |
| Write and native evidence | [receipts](../../src/write_receipt.rs), [import](../../src/import_evidence.rs), [apply](../../src/apply_evidence.rs), [take effect](../../src/apply_evidence/take_effect.rs) | Reuse per-identity waiting results; add typed read projection and bounded issue details | F09 |
| Traffic and workers | [workers](../../src/worker_util.rs), [subscriber metrics](../../src/subscriber_stats.rs), [NICs](../../src/nic_stats.rs) | Existing samples and caches; no assumed long-term history or per-WAN attribution | F06, F10 |
| Settings | [draft](../../src/tui/settings/draft.rs), [validate](../../src/tui/settings/validate.rs), [apply](../../src/tui/settings/apply.rs) | Existing terminal workflow writes config; extract neutral logic before web reuse | F15 |
| Journal | [journal](../../src/tui/journal.rs) | Terminal reader and view; needs safe, bounded service adapter | F10 |

## Current service operations

The exact dispatch list at this baseline is:

```text
status
registry.get
policy.commit
topology.preview.refresh
topology.preview.get
topology.policies
wan.discover
topology.tree
topology.sources.candidates
topology.edit.propose
topology.edit.confirm
topology.edit.cancel
plan.create
plan.update
plan.delete
subscriber.plan
```

`status` reports service version, runtime mode, registry revision, enrollment, last cycle start, and writer.
It does not supply every Dashboard card or a complete telemetry snapshot.
`registry.get` returns a domain document. Do not expose that document directly as a browser response.

`policy.commit` replaces catalog and source settings. Keep it outside the initial web allowlist.
Expose the narrower domain operations instead.
Discovery and fresh preview operations can read sources. Queue them explicitly. do not call them on page load.

The current protocol limits requests to 1 MiB and responses to 16 MiB.
These are transport ceilings, not browser payload targets.
The web adapter must validate response protocol version, request ID, and result shape.

The current client does not explicitly check response version and ID. F02 adds that proof before reuse.

## Gap register

| Gap | Missing or incomplete capability | Required result | Task |
|---|---|---|---|
| G01 | Browser HTTP server and shared Rust DTOs | Leptos CSR plus typed Axum reads; no native dependencies in Wasm | F00–F01, F05 |
| G02 | Unprivileged access to root-owned service data | Narrow broker, peer checks, fixed paths, bounded requests | F02 |
| G03 | CLI accounts, sessions, CSRF, revocation | Multiple superadmins managed only by local CLI; sign-in/sign-out only on web | F04, F14 |
| G04 | Stable browser snapshots and paged queries | Immutable snapshot IDs, source age, consistent cursors, safe fields | F03 |
| G05 | Durable web action and audit records | Known dispatch state, duplicate protection, restart reconciliation | F17 |
| G06 | Browser access to existing plans and topology | Revision-checked narrow adapters and observed result panels | F11–F13 |
| G07 | Safe settings mutation outside terminal code | Typed fields, digest checks, secret handling, service-owned transaction, reload evidence | F18, F15 |
| G08 | Typed native issues and waiting details | Counts, bounded pages, receipt correlation, no fabricated acceptance | F09 |
| G09 | Historical and attributed traffic | Bounded chart samples and source labels; retention and WAN attribution proof before wider claims | F06, D03 |
| G10 | Web journal and worker reads | Fixed units, redaction, bounded pages, no arbitrary command | F10 |
| G11 | Router connection tests and router CRUD through web | Service-owned operations, credential handling, validation and reload results | F15; tests deferred to D04 |
| G12 | Schedule CRUD and execution | Timezone, overlap, DST, restart behavior, effective-speed and capacity proof | D01 |
| G13 | RouterOS authorization, disable, and profile writes | Separate object ownership, dry-run, idempotent writer, readback and rollback | D02 |
| G14 | Routing allocation, health-triggered failover | Real routing observation and approved writer contract; logical shaping assignment is insufficient | D02 |
| G15 | Remote LibreQoS control and destination intelligence | Explicit source, identity, retention, privacy and ownership contracts | D03 |

## Important corrections to the images

- Local plans replace the proposed PPPoE router-profile sync workflow.

- `Authorize` and `Disable` controls require G13. Do not release those controls with no backend.

- WAN pin and rebalance can change shaping policy. They do not prove a MikroTik route change.

- Do not use generated fixture counts, customer names, dates, rates, or utilization formulas in production.

- The Users mockup is withdrawn from the delivery scope. All account CRUD uses the local CLI. No signup or account-management HTTP route exists.

- A saved setting or file write is not proof of a successful reload or dataplane change.

- Do not claim the manager activation design is implemented merely because its plan exists.

## Deferred domain packages

| Task | Dependency and concrete scope | Completion gate |
|---|---|---|
| D01 | After F11: design `src/topology_manager/schedule.rs`, store migration, resolver, service operations, and web schedule editor | Resolve timezone, midnight, DST, overlap, manual override, restart, and capacity-ceiling cases before any automatic action |
| D02 | After F13: separate RouterOS routing/access writer project | Owner selects writable objects, credentials, observation, pin-outage rules, failback, rollback, and readback; simulate partial writes before deployment |
| D03 | After F06/F09: traffic retention, WAN attribution, remote sync, or destination intelligence | Specify each data source and retention policy; prove attribution and freshness; do not infer applications from shared CDN IPs |
| D04 | After F15: explicit router test and router management jobs | Reuse service sessions, bound scans and retries, protect existing identity scopes, and check configuration reload |

Each deferred package needs a focused design and its own atomic implementation plan.
Its interface is not frozen by a picture. Initial release controls must reflect actual capabilities.
