# LQoSync4 feature and function map — design, not implementation

## Status and ownership

**Existing** means a function is present on the `lqosync4` branch because it is based on LQoSync3. **Proposed** means an interface or module to implement. The name of a proposed function is a working name. The Rust HTTP framework, persistent store, and RouterOS write objects remain open decisions. Leptos is the selected frontend.

The [route and component contract](ROUTES_AND_COMPONENTS.md) is the detailed map for screen placement, nested components, reads, actions, and cross-route navigation. Route and endpoint names in the table below are short summaries of that contract.

The browser reads API snapshots. The collector owns RouterOS reads. The shaping writer owns LibreQoS output. A separate, narrowly scoped router writer will own any new MikroTik changes. HTTP handlers must not call the collector or writer directly.

## Existing core functions to retain or adapt

| Existing function | Current job | LQoSync4 use |
|---|---|---|
| [`routeros::read_snapshot`](../../../src/routeros.rs) | Reads enabled PPP, DHCP, and Hotspot tables. | Feed subscriber and router snapshots. |
| [`routeros::read_router`](../../../src/routeros.rs) | Connects, retries, and reads one router. | Run in a bounded collector task. |
| [`runner::cycle_inputs`](../../../src/runner.rs) | Reads live LibreQoS files and saved state. | Preserve compatibility and shadow previews. |
| [`cycle::assemble`](../../../src/cycle.rs) | Builds circuit rows and change report from router results. | Keep one canonical shaping model. |
| [`validate::check`](../../../src/validate.rs) | Checks rows and network structure. | Reject invalid output before apply. |
| [`detect::resolve`](../../../src/detect.rs) | Selects LibreQoS output target. | Retain v2/v3 target selection. |
| [`writer::pair::render_csv`](../../../src/writer/pair.rs) | Renders ShapedDevices CSV. | Keep output parity. |
| [`writer::v3network::render_libreqos_network`](../../../src/writer/v3network.rs) | Renders LibreQoS v3 network input. | Keep output parity. |
| [`files::write_if_changed`](../../../src/files.rs) | Writes changed bytes atomically. | Avoid unnecessary shaping reloads. |

These functions do not implement WAN assignment, RouterOS writes, HTTP endpoints, or Leptos pages.

## Proposed backend and frontend map

| Feature | Rust backend responsibility and proposed function | API/read model | Leptos route and component | Main check |
|---|---|---|---|---|
| Overview | `snapshot::latest()` returns validated counts and ages; `telemetry::aggregate()` returns bucketed traffic. | `GET /api/v1/overview`, `GET /api/v1/traffic?window=` | `/` — `DashboardPage`, `SummaryCards`, `TrafficChart`, `HealthGrid`, `AllocationTable` | A slow router cannot delay a page response. |
| Routers | `routers::list()` and `routers::test_connection()` read and report core/access state. | `GET /api/v1/routers`, `POST /api/v1/routers/{id}/test` | `/routers` — `RouterTable`, `RouterDetail` | No secret in response or audit. |
| IPoE subscribers | `subscribers::discover()` identifies DHCP clients; `subscribers::authorize()` applies approved access policy. | Paged `GET /api/v1/subscribers?kind=ipoe`; explicit authorization action | `/ipoe/subscribers` — `SubscriberTable`, `AuthorizationPanel` | Unknown and unauthorized are distinct. |
| PPPoE users | `subscribers::reconcile_pppoe()` imports users and live state; `profiles::assign()` changes the desired profile. | Paged `GET /api/v1/subscribers?kind=pppoe`; profile action | `/pppoe/users` — `PppoeUserTable`, `ProfilePicker` | User identity remains stable across IP changes. |
| Profiles | `profiles::effective_at()` resolves baseline, schedule, and manual override; `profiles::preview()` shows impact. | `GET /api/v1/profiles`; preview and mutation actions | `/ipoe/profiles`, `/pppoe/profiles` — `ProfileCards`, `ScheduleEditor` | Midnight, overlap, timezone, priority. |
| LibreQoS | Existing cycle and writer plus `shaping::status()` and `shaping::preview()` expose latest result. See [v3 data flow](LIBREQOS_V3_DATA_FLOW.md). | `GET /api/v1/libreqos/status`, paged preview | `/libreqos` — `SyncStatus`, `SourcePreview` | Output parity; distinguish written from imported; no live write in shadow mode. |
| Load Balancer | `assignment::plan()` computes stable desired WAN; `assignment::diff()` compares with observed; `assignment::apply()` sends approved changes to one writer. | `GET /api/v1/allocations`, paged `GET /api/v1/assignments`, preview/apply actions | `/load-balancer` — `AllocationCards`, `AssignmentTable`, `DiscoveryTab`, `HistoryTab` | Same inputs give same plan; repeated apply makes no write. |
| Upstream health | `health::sample()` and `health::transition()` derive unknown/up/down/recovering with hold-down. | `GET /api/v1/upstreams`; event stream | `HealthGrid`, `UpstreamCard` | Stale is visible; flapping does not cause repeated moves. |
| Audit and users | `auth::authorize()` checks each action; `audit::append()` records actor, reason, before/after, result. | Auth and paged audit endpoints | `/users`, `/settings`, `HistoryTab` | Unauthorized write is rejected and logged. |
| IP intelligence — later | `telemetry::observe_destinations()` and `enrichment::lookup()` classify IP/ASN/org/CDN; `app::classify()` reports evidence and confidence. | Aggregated, retained, paged queries | Future `/traffic-intelligence` | Shared CDN IP does not imply a certain app. |

## Data contracts to define before implementation

| Contract | Required fields | Owner |
|---|---|---|
| `SubscriberIdentity` | Stable subscriber ID, source kind, router ID, circuit ID, current IPs, observed time | Collector and identity reconciliation |
| `UpstreamState` | Upstream ID, capacity, health state, sample time, last transition | Health service |
| `Assignment` | Subscriber ID, current WAN, desired WAN, policy, reason, version | Planner |
| `ApplyResult` | Plan ID, object-level changes, readback result, error, actor, time | Router writer and audit |
| `OperationalSnapshot` | Version, observed time, updated time, stale/error flags, counts | Snapshot publisher |

An action uses the snapshot version or plan ID that the operator reviewed. If the input changes, the server requires a new preview before apply. This prevents an old browser tab from applying a stale plan.
