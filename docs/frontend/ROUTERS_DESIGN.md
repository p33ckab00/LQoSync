# Routers screen design

Date: 2026-11-21 (Asia/Shanghai). Status: proposed design, not implemented. Reviewed on 2026-11-21.
Read the [review](ROUTERS_REVIEW.md) first. Its backend gaps, scope changes, and decisions take priority over this document.
Companion: [atomic implementation plan](ROUTERS_IMPLEMENTATION_PLAN.md).

## Owner decisions

- Leave the Dashboard and its current work to Claude Code.

- Use the web UI as the primary configuration and orchestration surface. Keep the TUI presentation unchanged.

- Add and edit router registrations. Disable or enable them. Do not provide router deletion.

- A disabled router must receive no LQoSync reads or writes, including tests and discovery.

- Include removal from active shaping in the disable review.

- Plan RouterOS writes for WAN pinning through address lists on the core router that owns the WAN.

- Accept subscriber IPs and application IPs from a future DPI provider as pinning inputs.

- Show the future WAN pinning UI before its backend exists. Do not represent it as operational.

- Use Vercel frontend-design, Google modern-web-guidance, and Microsoft Fluent 2 guidance. Keep Rust and Leptos.

The owner selected Fluent 2 guidance because no Microsoft frontend skill is installed locally.
This document uses its interaction guidance. It does not introduce React or Microsoft 365 dependencies.
No application code, router configuration, or production service changes belong to this planning task.
Keep the current gateway on port 9581 and the existing Traefik/nginx deployment contract.

## Research baseline

The inspected LQoSync3 HEAD was `c324203`. Dashboard changes were present in the shared worktree.
The LQoSync4 documentation reference was `origin/lqosync4` at `b5284bd`.
The reference files were read with `git show`. No LQoSync4 branch or folder was created.

LQoSync4 contributes router inventory, core/access context, detail inspection, registration, connection tests, read jobs, source links, and action results.
The older [screen contract](SCREENS.md#02-routers) and F08 describe read-only inventory first.
The owner decisions above extend that scope. They do not make the missing operations available.
The older [router image](images/02-routers.png) remains a composition reference, not an implementation contract.
Its password dots, sample records, Users navigation, and generic Request sync action are not production data or approved behavior.

| Evidence | Current result | Design consequence |
| --- | --- | --- |
| [RouterCfg](../../src/config.rs) | Connection fields and per-protocol flags exist. A router-wide administrative flag is absent. | Add a durable router control state before operational disable controls. |
| [RouterOS session](../../src/routeros.rs) | The session trait exposes reads through `print`. | A WAN address-list writer is a new backend package. |
| [Legacy cycle](../../src/runner.rs) | Subscriber reads use `reads_subscribers()`. | Source flags cannot establish a complete router I/O barrier. |
| [Coordinator](../../src/topology_manager/coordinator.rs) | Independent source sessions and read plans exist. | Disable must invalidate its sessions and queued work too. |
| [WAN discovery](../../src/topology_manager/wan.rs) | Discovery can connect to configured routers separately. | Discovery must use the same router gate. |
| [Collection](../../src/collection.rs) | Cached source states, times, counts, and subscribers exist. | Build safe router projections from service evidence. |
| [Router identity](../../src/topology_manager/policy_draft.rs) | Initial router IDs derive from the configured name. | Keep the connection name immutable until an identity migration exists. |
| [Config apply](../../src/tui/settings/apply.rs) | Digest checks and safe file/secret operations exist inside terminal code. | Extract a service boundary without changing the TUI workflow. |
| [Core dispatch](../../src/api/dispatch.rs) | No narrow router registration, disable, test, or writer operation exists. | Add typed operations, not arbitrary command forwarding. |
| [Broker](../../crates/web-gateway/src/broker.rs) | Fixed login/overview/traffic requests exist, with bounded replies. | Add narrow router DTOs and bounded job requests. |
| [HTTP routes](../../crates/web-gateway/src/http.rs) | Router endpoints are absent. | All router paths below are proposals. |
| Inspected contract and dispatcher | No typed DPI application-IP provider was found. | Application selection stays unavailable until its provider contract exists. |

## Screen hierarchy

The main screen is an inventory. A quick detail drawer supports inspection.
Use a full detail route for persistent forms and longer workflows.
All router connection settings stay under that router. Global Settings does not contain these fields.
Protocol-specific configuration remains on its feature screen, linked with the selected router scope.

| Screen | Proposed browser route | Contents and actions |
| --- | --- | --- |
| Router inventory | `/routers` | Search, filters, paged rows, Add router, Open details, Disable/Enable |
| Add router | `/routers/new` | Three-step connection/source review. Save as disabled. |
| Overview | `/routers/{id}?tab=overview` | Administrative state, observed state, roles, source ages, counts, latest job, shaping membership |
| Connection | `/routers/{id}?tab=connection` | Endpoint, API/TLS settings, username, credential status/replacement, connection test |
| Sources | `/routers/{id}?tab=sources` | WAN/PPPoE/DHCP/Hotspot participation, source states and counts, links to scoped feature settings |
| Read and write | `/routers/{id}?tab=operations` | Read capability, explicit read jobs, future WAN-write capability, job progress and results |
| WAN pinning | `/routers/{id}?tab=wan-pinning` | Core WAN target, address-list policy, subscriber/application inputs, membership preview, desired/observed state |
| Activity | `/routers/{id}?tab=activity` | Router-specific config, enable/disable, collection/test, shaping-removal, and future writer events |
| Disable review | Dialog from router detail or row action | Exact local shaping impact, I/O stop, dependent policy warnings, explicit confirmation |

The drawer links to the same stable router ID and tab. It contains summary information and Open router.
Connection forms and WAN policy editors use the full route, not nested drawers.
The Sources tab summarizes protocol participation. It does not duplicate full PPPoE or IPoE editors.
An unavailable destination feature has a clear unavailable state instead of a broken link.

## Main layout

```text
Routers                                            [Add router]
Configure endpoints, source reads, and core WAN policy.

[Search]  [Function: All]  [Admin state: All]  [Collection: All]

Router       Functions       Admin       Collection      Age   Actions
Core-01      WAN             Enabled     Not tested       —     Open / …
Access-01    PPPoE, DHCP      Disabled    Last known       8m    Open / …

25 per page                 Source snapshot / age        Previous / Next
```

Names above are illustrative. Live pages use service data only.
Keep the router name, administrative state, and row action accessible during horizontal table scroll.

Use a semantic table with header associations. Do not claim ARIA grid keyboard behavior without implementing it.
On narrow screens, hide optional columns behind a detail disclosure and keep the key state labels.
The whole page must fit at 320 CSS pixels and 200% zoom.

Core/access filters follow explicit router functions. A WAN role shows core participation. Protocol roles show access participation.
A router can have both. Do not infer its role from its address, name, port number, or traffic direction.

## Visual system and interaction

Reuse the current shell, logo with superscript v3, company branding, IBM Plex Sans, and Light/Dark/System themes.
Use existing semantic tokens, not a separate Fluent palette.
The current dark baseline is canvas `#0b0c0e`, surface `#17191b`, text `#f1f3f5`, and action `#80aaff`.
Use 14–16 px body text, 13 px metadata, a 26 px page title, and tabular figures.
Use 24 px desktop padding, 16 px phone padding, and restrained panel borders.

Use a compact table as the main visual structure. Avoid duplicate KPI cards above a small inventory.
Use a light glass effect on the detail surface only when it preserves text contrast.
Provide solid and forced-color fallbacks. Respect Reduced Motion.

A drawer provides context. A dialog confirms disable impact. A full route supports multi-section configuration.
These choices follow [Fluent drawer guidance](https://fluent2.microsoft.design/components/web/react/core/drawer/usage).
Use focus entry, keyboard containment for modal surfaces, and focus restoration from [Fluent dialog guidance](https://fluent2.microsoft.design/components/web/react/core/dialog/usage).

Disable requires review and submission. Use a labeled action, not an immediate toggle.
[Fluent switch guidance](https://fluent2.microsoft.design/components/web/react/core/switch/usage) distinguishes immediate changes from submitted changes.
Show persistent errors next to the affected form or job. A short toast can link to a durable result.

## Add and connection flow

1. Identity and connection: unique connection name, endpoint, API port, TLS and certificate options, username, new credential input.

2. Source participation: explicit WAN/PPPoE/DHCP/Hotspot roles and links to their feature settings.

3. Review: resolved defaults, safe field changes, source scope, and initial Disabled state. Select Add router.

Creation is a local configuration operation. It must not log in, test, discover, or collect from the router.
The operator then selects Enable, and can explicitly request a connection test or source read.

Disabled entries allow local connection edits and credential replacement. These actions do not contact MikroTik.

The server owns secret storage and file paths. Browser responses expose credential-configured status, not secret values or secret-file paths.
A blank replacement field preserves the current credential. A replacement requires an explicit field action.

Preserve existing API/TLS settings. Use a verified-TLS default for new entries where the endpoint supports it.
Do not silently change legacy connections during migration.

## Administrative state and no-I/O contract

Keep administrative state separate from connection health and source freshness.
Use Enabled, Disabling, Disabled, and Enable pending where an operation needs time.
A configured endpoint is not evidence of a healthy connection.

Disabled means no new or remaining LQoSync operation can use that router connection.
This includes scheduled reads, manual reads, connection tests, WAN discovery, retries, login, keepalive, writes, and readback.
Enforce the rule in the core service, before network dispatch. Every caller uses the same router ID and generation gate.
A disabled router cannot bypass the rule through the TUI, an old job, or a direct API request.
Cached inspection and local configuration remain available with Last known labels.

Disable admission must close durably before the service drains or cancels active operations.
Show Disabling until the service acknowledges quiescence. A packet already sent cannot be recalled.

An interrupted write has Unknown outcome until later reconciliation. Do not do readback while the router is disabled.
Restart recovery restores the disabled gate before it starts collectors or retries.
Enabling does not replay old write jobs.

## Disable and active shaping

The owner selected removal from active shaping as part of disable review.
Use two result tracks: router I/O state and LibreQoS publication state.
The reviewed action must not remove the router registration, credentials, history, or stable identity.

The preview contains:

- Router ID, config digest, registry revision, published generation, and impact hash.

- Exclusive router-derived circuits to remove, shared identities to keep, and their exact source memberships.

- Affected source/tree nodes, local plans, WAN dependencies, and pinned policies.

- The expected LibreQoS change and the existing safety checks that can block it.

- Confirmation that MikroTik I/O stops. No RouterOS cleanup can run after disable.

A WAN-only core can have zero directly collected subscriber circuits.
Do not remove all WAN-assigned subscribers merely because the core router is disabled.
Shared-source identities remain when another enabled source still supports them.
Dependent topology or routing changes need their own explicit preview.

Validate the candidate before mutation. On confirmation, persist the disable intent, close admission, drain sessions, and queue the reviewed local publication.
Keep each stage recoverable. Do not describe multi-file writes as one atomic transaction without a journal and recovery proof.

If publication fails, remains in shadow mode, or awaits import/apply evidence, keep the router disabled.
Show Disabled / Shaping removal pending or blocked, with the exact reason.
Do not automatically re-enable MikroTik I/O to compensate for a LibreQoS failure.
Show removal applied only after the existing publication evidence establishes it.

After enable, resume the configured reads. Fresh validated evidence can enter the normal compiler/publication flow.
Do not restore stale shaping records directly from the cached pre-disable state.

## Read and write operations

Reads use bounded core jobs. Browser navigation and filter changes only read snapshots.
Label an explicit refresh Read sources; it must not imply RouterOS mutation or LibreQoS apply.
A connection test reports TCP/TLS/login outcomes separately from full source collection.
All outcomes include observation time and a stable job ID.

The initial RouterOS write scope is WAN address-list pinning on an explicitly selected core WAN router.
PPPoE account changes, RouterOS profile writes, arbitrary scripts, and raw terminal commands are outside this router plan.
The UI displays read and writer capabilities separately from Enabled state.
An enabled router still has no write capability when its writer backend is absent.

## WAN pinning sub-screen

The sub-screen is present in the first UI implementation, even while the writer is unavailable.
Show Backend not available, a visible reason, and the planned form structure.
Do not expose a working Apply button, saved-policy claim, or fabricated live members.
Read-only or disabled fields remain labeled and discoverable.

The proposed form contains:

| Group | Fields |
| --- | --- |
| Target | Stable core router ID, explicit WAN ID, observed routing policy/table reference |
| Input | Subscriber IPs, application IPs from DPI, or both with explicit match semantics |
| Subscriber selection | Stable subscriber IDs, observed IPs, source router, source age, address family |
| Application selection | Application label, provider, observed destination IPs, observation interval, confidence, expiry |
| Address lists | LQoSync-owned list names, source/destination purpose, membership counts, ownership markers |
| Review | Adds/removals, conflicts, stale evidence, matched traffic scope, desired and observed membership |
| Result | Job/plan ID, write acknowledgement, readback, routing-policy verification, audit link |

Subscriber addresses normally supply a source-address match. Application destinations supply a destination-address match.
When both selectors are present, the proposal must state whether both must match. The initial proposed behavior is AND.
The operator must see that scope before any future apply.

Validate the address as visible at the core. A source IP changed by NAT needs an explicit mapping or an unavailable result.

Address lists provide packet-match groups. They do not alone establish a WAN route.
See [MikroTik address lists](https://manual.mikrotik.com/docs/firewall-and-quality-of-service/firewall/address-lists/) and [routing decisions](https://manual.mikrotik.com/docs/user-guides/routing-and-networking-protocols/routing-decision/).
A future writer must verify the relevant mangle/routing-policy binding and the intended WAN path.
It must not adopt, overwrite, or reorder unrelated rules without a separate approved ownership contract.
A shaping-tree WAN assignment is not proof of a MikroTik routing change.

Application addresses require an actual DPI/classification provider contract.
The inspected browser and core APIs do not supply that contract.

Do not infer an application from a port number, reverse DNS, or a shared CDN address.
A shared IP can serve several applications. Show uncertainty and bounded expiry instead of exclusive application claims.
Keep application selection unavailable until provenance, freshness, retention, and confidence fields exist.

Future membership changes must be idempotent, bounded, revision checked, and limited to owned objects.
A list readback proves membership, not the complete traffic path.
RouterOS version, IPv4/IPv6 support, existing policy bindings, NAT visibility, and pin-outage/failback behavior remain future writer gates.
No policy automatically applies itself when the writer backend later appears.

## Proposed API and DTO contract

All paths below are new. No router DELETE endpoint or delete operation is proposed.

| API | Purpose |
| --- | --- |
| `GET /api/v1/routers` | Cached paged inventory and capabilities |
| `GET /api/v1/routers/{id}` | Safe detail, source summaries, current control state |
| `POST /api/v1/routers` | Local registration, initially disabled |
| `PATCH /api/v1/routers/{id}` | Local connection/source-participation edits with expected digest |
| `POST /api/v1/routers/{id}/disable-previews` | Cached-source and published-state impact review |
| `POST /api/v1/routers/{id}/disable-jobs` | Confirm the reviewed disable and shaping-removal intent |
| `POST /api/v1/routers/{id}/enable-jobs` | Explicit enable without replay of old writes |
| `POST /api/v1/routers/{id}/test-jobs` | Bounded enabled-router connection test |
| `POST /api/v1/routers/{id}/read-jobs` | Bounded enabled-router source read |
| `GET /api/v1/router-jobs/{job_id}` | Durable stage/result projection |
| `GET /api/v1/routers/{id}/activity` | Bounded router-scoped audit events |
| `GET /api/v1/routers/{id}/wan-pinning` | Capability and future policy/evidence projection |

Proposed DTOs: RouterSummary, RouterDetail, RouterCapabilities, RouterCreate, RouterPatch, RouterDisablePreview, RouterJob, WanPinningView.
Mutations use the current session, CSRF/origin checks, expected versions, and server-attributed actor identity.
Jobs use idempotency keys. Lost responses must be recoverable through the job or intent ID.
A stale preview returns a conflict and requires a new review.
Server-side checks enforce Disabled and capability restrictions even if browser controls are changed.

## Performance, reliability, and integrity

- Use a cached immutable inventory snapshot. No page request starts RouterOS I/O.

- Default to 25 rows and cap a page at 50 rows, within the existing broker byte ceiling.

- Keep broker requests within 4096 bytes and replies within 65536 bytes. Target at most 60 KiB for a router page.

- A byte-limited page returns a cursor for the next emitted position. It must not skip records.

- Refresh list evidence at a five-second target while visible. Remote sampling remains service-owned and independently timed.

- Bound jobs per router. Serialize writes per router and coalesce duplicate reads.

- Preserve row identity, filters, scroll, tab, focus, and unsaved edits during refresh.

- Resolve stale/unknown separately from zero, healthy, unreachable, authentication failure, and TLS failure.

- Record local config commit, service reload, router disable, and shaping apply as separate evidence stages.

- Keep account CRUD in the privileged CLI. This screen does not add web user management.

## Delivery order and review gates

First deliver backend control state and cached inventory. Then add safe local registration and disable impact jobs.
Connect the UI to those capabilities. Include the visible future WAN pinning surface in that UI slice.
DPI evidence and the actual RouterOS writer are separate later packages.
Do not label this design implemented after a fixture-only UI or an unavailable-feature placeholder.

Open writer decisions include the DPI provider, supported RouterOS versions, ownership template, WAN policy binding, and outage behavior.
They do not block the inventory and router-disable design. They block production WAN pinning writes.

## References

- LQoSync4 blobs at `b5284bd`: `docs/lqosync4/docs/ROUTES_AND_COMPONENTS.md`, `PRODUCT_SPEC.md`, and `FEATURE_FUNCTION_MAP.md`.

- [Existing backend map](BACKEND_MAP.md) and [global implementation plan](IMPLEMENTATION_PLAN.md).

- [Vercel frontend-design](https://github.com/vercel-labs/open-agents/blob/main/.agents/skills/frontend-design/SKILL.md).

- [Google modern-web-guidance](https://github.com/GoogleChrome/modern-web-guidance), responsive-table and accessibility guidance retrieved on 2026-11-21.

- [Microsoft Fluent 2](https://fluent2.microsoft.design/), drawer, dialog, and switch interaction guidance.

These are planning references. The project keeps its current Rust stack and approved visual identity.
