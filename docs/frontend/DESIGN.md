# Frontend feature design

Status: implementation design baseline. All new modules and HTTP routes below are proposed.
Owner direction: Rust application logic, existing LQoSync3 backend, documented gaps, and atomic delivery.

## Product and ownership

The operator must see collection health, find subscriber problems, inspect shaping state, and edit supported topology policy.
LQoSync owns the local service plans and shaping policy.
LibreQoS enforces the shaping limits.

MikroTik retains routing, authentication, and session duties.
RouterOS profile names can remain source evidence. They must not silently replace a selected local plan.

The LQoSync4 design supplies the visual blueprint.
Current LQoSync3 contracts control operational behavior.
The newer local plan and topology workflow supersedes the pictures' RouterOS profile sync controls.

A shaping WAN parent does not prove a packet route.
Keep the TUI and all supported CLI operations.

## Rust architecture

Use Leptos CSR for the browser. Use a Rust Axum gateway for HTTP, sessions, safe DTOs, and bounded jobs.
Use a narrow local Rust broker for privileged access to the existing service and approved data files.
The HTTP gateway runs as an unprivileged service account.

The broker has no TCP listener and accepts only the gateway UID through a private Unix socket.
The existing service socket retains its current permissions and peer checks.

The broker exposes typed operations, not arbitrary socket forwarding, filesystem paths, shell commands, or process signals.
It validates each request independently and calls the current service API for supported domain mutations.
The service retains its instance lock, revisions, source reads, router sessions, compiler, and publication ownership.

The gateway must not create a second `Runner` or registry writer.

Use three workspace crates: `web-contract`, `web-gateway`, and `web-ui`.
Place the privileged broker binary and its allowlist inside `web-gateway` with separate entry points.
Keep browser DTOs independent of server configuration, secrets, and the main crate's native dependencies.

Do not compile the collector or native socket modules into Wasm.

Use SQLite on local disk for web users, sessions, audit records, and durable action records.
Do not copy topology policy into SQLite as a second authority.
Use a reviewed password hashing crate. Do not implement cryptography.

Task F00 selects compatible dependency versions and records licenses and build evidence.
Preserve the core Rust 1.89 contract. Do not raise its minimum version silently.

## Feature groups

| ID | Feature | Initial behavior | Backend dependency |
|---|---|---|---|
| UI-01 | Shell | Navigation, account, theme, service state, data age | Safe status DTO and session |
| UI-02 | Dashboard | Counts, issue links, bounded traffic series, health | Versioned snapshots and telemetry |
| UI-03 | Subscribers | PPPoE/DHCP/Hotspot filters, paging, detail, effective speed | Stable identity projection |
| UI-04 | Routers | Safe router inventory and collection errors | Redacted config and collection |
| UI-05 | Profiles | Shared local plans and membership | Existing plan operations and membership DTO |
| UI-06 | Topology | Builder, assignments, policies, publication strip | Existing topology API |
| UI-07 | LibreQoS | Candidate, written, imported, waiting, applied, issues | Receipts and apply evidence |
| UI-08 | Workers and Journal | Read-only diagnostics with bounded output | Existing telemetry and journal adapter |
| UI-09 | Users | Local accounts, roles, revocation, audit | New web identity store |
| UI-10 | Settings | Safe fields, reviewed changes, reload result | New service-owned settings boundary |
| UI-11 | Schedules | Later: local plans and node capacities | Schedule model and resolver missing |
| UI-12 | Routing controls | Later: actual upstream changes | Router writer and readback missing |

## Navigation and layout

Use Dashboard, Subscribers, Routers, Topology, LibreQoS, Workers, Settings, and Journal as the initial navigation.
Show Users only to administrators. Place local Profiles under Topology > Policies with links from Subscribers.
Use source filters for PPPoE, DHCP, and Hotspot instead of duplicate data models.

Keep deep links for the IPoE and PPPoE views shown in the pictures.
Label the initial WAN view `WAN topology`, not `Routing load balancer`.

The desktop shell uses a sidebar, slim top bar, page heading, and main content.
Use a drawer on narrow screens. Preserve visible actions and clear labels.
Use linked panels for overview, tables for comparisons, and drawers for selected-record details.

The topology view uses a tree, node details, and a keyboard-accessible parent picker.
Do not make drag gestures necessary for any operation.

## Visual system

| Token | Initial value or rule |
|---|---|
| Canvas / sidebar / surface | `#0b0c0e` / `#141517` / `#17191b` |
| Border / text / secondary text | `#2b2e32` / `#f1f3f5` / `#b2b8bf` |
| Action / healthy / warning / critical | `#80aaff` / `#2bd391` / `#ffd077` / `#ff858a` |
| Type | IBM Plex Sans for UI; IBM Plex Mono for identifiers; self-host approved font files |
| Type sizes | 14–16 px body; 13 px minimum metadata; 26 px page heading |
| Numerals | Tabular numerals for rates, counts, and timestamps |
| Space | 24 px desktop page padding; 16 px panel gaps; 16 px phone padding |
| Shape | 8–10 px control corners; 10–14 px panel corners |
| Motion | Short action feedback; respect reduced motion; preserve scroll and focus |

These values define a starting point. Validate contrast and actual browser text before release.
Status needs a word and icon as well as color.
Use neutral styling for unknown values. Never substitute zero for unavailable data.

Keep the topology hierarchy distinctive and the surrounding interface restrained.

## Interaction and state

Reads use cached evidence. A page load never starts RouterOS collection.
Explicit refresh and discovery controls create bounded jobs when source reads are necessary.
Show collection age separately from policy revision and publication state.

Preserve selection, focus, filters, scroll, and unsaved form values during background refresh.
Keep shareable filters, sort, page, selected ID, and tabs in the URL.

Use semantic HTML tables and labeled scroll regions.
Use container queries where a panel's width controls its layout.
Keep native modal semantics for blocking confirmations and safe initial focus for destructive actions.

Use menu semantics for row menus. Do not represent every overlay as a modal dialog.
Follow the Google guidance's browser fallbacks and Vercel's interaction checks.

Each operational view needs loading, empty, filtered-empty, stale, partial, unavailable, and forbidden states.
Each action needs pending, succeeded, rejected, conflict, failed, and outcome-unknown states.
Toasts supplement persistent result panels.

Never announce each traffic sample through a screen reader live region.

## Edit and publication semantics

Use `topology.edit.propose`, `topology.edit.confirm`, and `topology.edit.cancel` where the service requires proposals.
Use expected revisions for local plans and subscriber assignment.
A save result means policy saved. It does not mean LibreQoS applied it.

After activation, valid policy enters the approved automatic publication pipeline.
Do not introduce a universal second Apply gate into that pipeline.
When the manager is inactive, show `Saved; topology manager inactive` from actual status evidence.

A timeout after dispatch can mean an unknown outcome. Do not resubmit a mutation automatically.
Reconcile its durable action record and current policy before offering a new try.
Do not expose manager activation, deactivation, migration, or service restart as ordinary web edits.

Those operations keep their existing maintenance and authorization requirements.

## Delivery boundary

This change delivers documents and pictures only.
The [implementation plan](IMPLEMENTATION_PLAN.md) delivers connected vertical slices later.
Deferred domain tasks have explicit completion gates in the [backend map](BACKEND_MAP.md).
