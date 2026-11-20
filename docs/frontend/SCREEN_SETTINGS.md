# Screen-owned configuration and shared theme control

Status: owner direction. The frontend and its HTTP routes are not implemented yet.
The web UI is the primary configuration and orchestration surface. Keep the TUI as is.

## Shared theme control

Show an explicit `Light / Dark / System` selector on the sign-in screen and in the main application header.
Keep the header selector available on Dashboard and every feature screen, including narrow layouts.
Do not require navigation to global Settings to change the theme.

Use one shared preference and one theme component across routes.
Keep System as the default and follow changes in the accessing device/browser environment.
An explicit Light or Dark selection remains active until the user changes it.
Theme changes preserve current forms, filters, focus, selection, and scroll.

## Configuration ownership

Each feature owns its configuration forms, validation messages, change review, and result display.
Place a Settings tab, panel, or action inside the relevant feature screen.
Do not gather feature configuration into the global Settings screen.

A link can open the owning feature screen. It must not create a second editor for the same fields.
Use one authoritative backend field and validation rule even when several screens display its value.
Shared plans remain shared domain records. Their placement does not require duplicate per-source plans.

| Screen or feature | Configuration location | Ownership and boundary |
|---|---|---|
| Dashboard | Local view controls | Traffic window and display filters; links open the owning feature for operational changes |
| PPPoE | PPPoE view > Settings, with selected router/source | Supported PPPoE collection/source settings and policy bindings; local plan selection links to its owner |
| IPoE/DHCP | IPoE view > Settings, with selected router/source | Supported DHCP source filters, unassigned handling, and client overrides |
| Hotspot | Hotspot view > Settings, with selected router/source | Supported Hotspot source settings and policy bindings |
| Local plans/profiles | Profile or plan detail > Settings | Local rates, membership, and supported plan rules; no RouterOS profile sync |
| Routers | Router detail > Connection and Settings | Router endpoint, connection credentials, timeouts, and router-wide collection settings |
| Topology/WAN | Builder node detail or Policies | Node hierarchy, capacity, factors, fallback, shaping placement, pin/rebalance controls |
| LibreQoS | LibreQoS > Settings | Supported target, shaping paths, sync policy, and native verification configuration |
| Workers | Workers view > Settings when supported | Feature-specific telemetry/display settings; do not invent DPDK runtime tuning APIs |
| Journal | Journal view controls | Level, search, follow, and bounded view preferences; fixed service-unit permissions remain backend-owned |
| Schedules | Owning plan/node > Schedules, after D01 | Target-specific schedule rules; backend implementation remains a gate |
| Integrations | Integration detail in its owning feature | RouterOS connections under Routers; LibreQoS integration under LibreQoS |
| Global Settings | Application-wide settings only | Supported deployment identity, application defaults, and cross-application notification configuration |
| Account administration | Root CLI only | Username/password CRUD and account access state never move into web Settings |

These categories define screen ownership, not new supported backend fields.
Expose only fields that exist in the reviewed config/domain schema and have safe service operations.
Keep managed fields read-only in their owner screen, with an explanation.
If a backend function is missing, record its gap and task. Do not add a decorative Save button.

## PPPoE example

An operator opens the PPPoE subscriber view and selects a router/source.
The Settings tab shows only PPPoE configuration for that scope.
The form clearly identifies the router/source and the configuration version under review.

Router connection credentials have a link to Router detail. Local plan edits have a link to Plan detail.
The PPPoE form does not duplicate those editors or expose unrelated DHCP/LibreQoS fields.
Subscriber session accounts are distinct from web superadmin accounts.
RouterOS account authorization and profile writes remain deferred to D02.

## API and integrity

Use typed read models and scoped patches for each owning feature.
The backend resolves the scope and rejects fields outside that scope before any write.
Hiding fields in the browser does not enforce ownership.

Several scoped forms can still modify one physical configuration file.
The existing digest/revision, backup, transaction, and reload rules apply across all scopes.
Reject a stale form even if another screen made the conflicting change.
Do not create separate configuration files or divergent copies merely to match screen boundaries.

The global settings endpoint accepts application-wide fields only.
Service-owned typed settings operations remain shared implementation machinery, not a global browser editor.
Keep safe coexistence with the unchanged TUI as a release gate for configuration writes.

## Task and acceptance mapping

| Task | Required change |
|---|---|
| F05 | Shared header and sign-in theme selector with identical state |
| F07/F08 | Source-specific Settings entry points and router detail links |
| F09/F11/F12/F13 | Owner-local configuration panels for LibreQoS, plans, topology, and assignments |
| F18 | Typed scope/field allowlists, shared transaction validation, digest conflicts, and reload evidence |
| F15 | Connect scoped forms on their owner screens; leave only application-wide fields in global Settings |
| F16 | Prove all ownership, theme availability, conflict, and unchanged-TUI requirements |

Check a direct HTTP request that places a PPPoE field in global Settings. It must fail without a write.
Check a source patch that names another feature's field. It must fail without a write.
Check edits from two screens against the same old digest. Only the valid current transaction may proceed.

Check the theme selector on sign-in, Dashboard, feature screens, and phone layouts.
Check theme changes while a feature form has unsaved edits. Keep those edits and their validation state.
Do not add web account CRUD or alter existing TUI screens to satisfy this design.
