# LQoSync4 product specification — draft

## Goal

Port the LQoSync3 synchronization core without TUI. Build a web application for router, subscriber, profile, LibreQoS, and upstream operations. Use all supplied NetCore screenshots as workflow and visual references. Implement original code; no NetCore internals were inspected.

All application logic, API handlers, UI components, and browser-side application code SHALL be written in Rust. The browser client SHALL use Leptos and compile to WebAssembly. CSS and required browser/wasm binding glue remain permitted assets; do not add hand-written JavaScript or TypeScript application code. See [Rust frontend options](RUST_FRONTEND_OPTIONS.md) for the decision context. The backend framework remains open.

LQoSync4 selects and reconciles subscriber assignments. MikroTik core applies routing and handles sessions. LibreQoS remains the shaping target. Failover may change the public IP and interrupt active sessions.

## Baseline

The public LQoSync3 branch collects PPPoE, DHCP, and Hotspot data over RouterOS API, creates and validates a circuit model, and writes LibreQoS `v2_pair`, `v3_pair`, or `v3_network` input. It supports `shadow` and `apply` modes and avoids unchanged writes. Its README says router writes are outside its scope. Preserve these behaviors during the port; verify against the exact source commit when the target branch exists.

Sources: [README](https://github.com/p33ckab00/LQoSync/blob/lqosync3/README.md), [architecture](https://github.com/p33ckab00/LQoSync/blob/lqosync3/docs/ARCHITECTURE.md).

## Screens and backend

| Screen shown | Placement and interaction | Backend requirement |
|---|---|---|
| Dashboard | Fixed dark left sidebar; slim top bar; summary cards above charts; health and load balancer panels below. | Counts, traffic samples, upstream state, LibreQoS sync, timestamps and errors. |
| Routers | Core and access router management. | Registration, connection checks, read-only sync, status. |
| IPoE Subscribers | Searchable identity, authorization, profile and online state. | DHCP discovery, reconciliation, authorization, profile assignment. |
| IPoE Profiles | Cards with baseline speed and inline scheduled overrides. | CRUD, overlap priority, effective speed, scheduler. |
| PPPoE Users | Dense table with router, profile, effective speed, override, status and actions. | RouterOS import, status refresh, profile and manual-speed changes. |
| PPPoE Profiles | Router-filtered cards, user counts, sync and schedule controls. | RouterOS profile reconciliation and scheduled overrides. |
| LibreQoS | Last sync, local/remote state, rows, path and CSV preview. | Existing output generation, status, digest comparison, safe resync. |
| Load Balancer | Summary cards; Allocations, Assignments, Discovery, History tabs; allocation cards; searchable assignment table. | Percentage/count/manual policies, stable assignments, dry-run diff, manual move, audit history. |
| Upstream monitoring | Health grid and per-upstream capacity, members, RX/TX and utilization. | Checks, thresholds, failure/recovery state and traffic sampling. |
| Users and Settings | Same shell, dark cards and green/red badges. | Auth, roles, config, credentials and audit. |

The screenshot's 1,038 allocated subscribers are sample data, not a scale limit.

## Functional requirements

1. Each subscriber has one effective upstream. Keep it stable while the assignment remains valid.
2. Support percentage target, fixed subscriber count, and manual pin. Show the reason for each assignment.
3. Do not place subscribers on a known-down upstream. Move affected subscribers on failure. Re-enter a recovered upstream only after a stable period.
4. Show a precise RouterOS diff before apply. Apply idempotently, read back results, and record errors and audit events.
5. Support baseline speed and scheduled overrides with days, time window, download/upload, and priority. Show the effective value and its source.
6. Show the age of health and traffic data. `Unknown` is distinct from `Up` and `Down`.
7. Authorize every state-changing action. Do not expose secrets in API results or audit records.

## Acceptance

- Equivalent input generates the same LibreQoS output as LQoSync3 before new features are enabled.
- A dry-run enumerates every proposed subscriber and RouterOS change.
- Repeated reconciliation makes no extra writes when actual state matches desired state.
- Simulated outage/recovery produces traceable history and no assignment to a known-down upstream.
- Desktop and mobile layouts remain usable; wide tables scroll without hiding actions.

## Performance and data flow

- RouterOS collection and LibreQoS reconciliation run independently of HTTP requests. A page request reads the latest validated snapshot and does not start a router scan.
- Use one bounded collector per router, bounded retry queues, and a single writer for each externally owned target. A slow router cannot hold the UI response path or create an unbounded backlog.
- Publish snapshot versions and changed records to the UI. Batch rapid changes. Include `observed_at`, `updated_at`, and stale/error state in every operational response.
- Filter and page subscriber and assignment tables on the server. Render only visible rows in the browser. Charts request time-bucket aggregates rather than raw subscriber samples.
- Cache repeated lookup results and use incremental planning when only a subset of subscribers or upstreams changes. Do not write RouterOS or LibreQoS state when the desired and observed values match.
- Measure first-load WebAssembly transfer size, time to interactive, API p50/p95 latency, peak memory, collector cycle time, and RouterOS write count. Compare the port with LQoSync3 before setting final budgets.
- Provisional load profiles are the reference screenshot scale (about 1,000 subscribers and nine allocations) and a 10,000-subscriber stress profile. These are test workloads, not promised capacity. Final thresholds require representative hardware and network measurements.

## Later IP intelligence

Observe subscriber-to-destination-IP traffic with time and volume. Enrich destinations with ASN, organization, and likely CDN using cached, versioned data. Treat app labels as uncertain when shared CDN IPs do not identify an app. Confirm telemetry source, IPv4/IPv6 support, retention, privacy, and measured accuracy before implementation.
