# Topology Studio canvas cards — design

Status: owner direction received 2026-11-24. Builder: Codex (frontend lane).
Design and handoff: Claude Code. Backend gaps: Claude Code (backend lane).
Handoff: `docs/handoffs/2026-11-24-codex-studio-canvas-cards.md`.

## 1. Intent

The owner wants one workspace for the full topology workflow: the infinite canvas.
Each router has a card. The router card is the entry point for all configuration and
discovery of that router ("Router card–driven configuration workflow").

Today the workflow uses many dialogs: Router source configuration, Add router, Router
inventory, Card settings, and preview details. The layout is not good and the operator sees no
flow. This change moves the existing functions into cards and inline card settings on the
canvas. The visual reference is Stitch "Canvas Pipeline (Mockup B)".

The expected flow is:

**Add router → Test → Save → Configure sources → Discover → Pin → Configure policy →
Preview → Save → Apply (service)**

## 2. Goals

- The canvas is the only workspace for router, WAN, and subscriber-source configuration.
- An empty canvas shows one primary action: "Add router". No onboarding screen.
- Every resource has a card. Every card has the same icon actions (section 7.6).
- Settings open inline: the card expands in place on the canvas. One card is open at a time.
- Discovered resources are the inventory. The Router inventory dialog goes away.
- WAN pinning uses a pin icon, not a text button.
- Read and write configuration stay separate. Discovery never needs write access.
- Compact and short text. One fact shows one time.

## 3. Non-goals

- No change to the Sankey ribbons, live throughput, or column rules.
- No change to Topology settings, Subscriber allocation, Excluded sources, or the help
  workflow dialog in this change. They stay as dialogs (phase 3, section 11).
- No feature in the "Needs backend" column of section 8 until the backend lane delivers it.

## 4. Anti-goals

The change fails, also if it compiles, when one of these occurs:

- A card shows an invented value. Examples: RouterOS version, board, serial, uptime, RTT,
  cipher, ribbon color, CAKE queue name. The service does not send them today.
- A control offers a write scope that the service does not support.
- A secret goes into the draft, the browser storage, or the DOM after the card closes.
- A pan or zoom gesture moves the camera while the operator types in a field.
- The operator cannot complete the flow with the keyboard only.
- A save sends a partial policy set. Save sends the complete set, as today.
- An unpin removes a WAN that the topology uses, with no warning.

## 5. Constraints

- Stack: Leptos (Rust/WASM) in `crates/web-ui`. CSS in `crates/web-ui/style.css`.
- The canvas layer is HTML with `translate(x,y) scale(zoom)` from the camera
  (`topology_canvas.rs`, class `canvas-layer`). Cards in this layer move with the camera.
- Use only the current routes (section 9). Do not invent a route, request, or result.
- Keep the frontend requirements of `docs/handoffs/2026-11-23-codex-topology-frontend.md`.
  Examples: keep saved, draft, candidate, and apply evidence separate. Ignore stale
  responses. Keep secrets out of draft state. Save access profiles before source policy.
  Keep preview optional.
- The Stitch `DESIGN.md` rule "Settings open in one modal at a time" changes for the
  cards in this design: settings open inline, one card at a time.
- Accessibility: visible focus, word + icon + color for status, reduced motion, 44 px
  targets, forced-colors safe, tooltip and accessible name on every icon.
- Codex and Claude Code share the `lqosync3` worktree. Stage explicit paths only.

## 6. Approaches

### 6.1 Layout (owner picked B)

- A. Docked panel at the right edge. Rejected.
- **B. Cards on the canvas, connected as a flow. Picked.**

### 6.2 Card placement (owner picked 1)

1. **World-space cards. Picked.** Cards are children of the canvas layer. They move with pan
   and zoom and show on the minimap.
2. Screen-space overlay with the look of cards. Rejected: the infinite canvas idea is a
   fixed rule.

### 6.3 Workflow model (owner direction, 2026-11-24)

The router card drives the workflow. An earlier draft of this design used a fixed
three-step pipeline. The owner replaced it with the router-card model of section 7.

## 7. Design

### 7.1 Canvas states

| State | What the canvas shows |
| --- | --- |
| Empty: no router | A centered "Add router" card with a plus icon. Nothing else. |
| Router saved, no sources | The router card. Its source summary says "No sources". |
| Sources set, no read | The router card with a "Discover" icon in the accent color. |
| Discovered | The router card, its discovered-resource tray, and Sankey cards for pinned or grouped resources. |
| Full topology | The current Sankey (WAN → router → groups → tiers) and the trays of each router. |

### 7.2 Card types

| Card | Where | Content (compact) | Icons |
| --- | --- | --- | --- |
| Add router | Empty canvas, or a ghost card at the end of the router column | Plus icon, "Add router" | — |
| Router | Router column of the Sankey | Name, connection state, identity (from the test), source summary chips (WAN, PPP, DHCP, Hotspot), last discovery time | settings, refresh, discover, more |
| Discovered tray | Below its router card, connected with a dashed line | One compact row for each discovered resource: name, type, short facts, pin state | pin, settings (pinned only) |
| WAN | WAN column | Name, interface, router, configured capacity down/up, live band (current) | settings, pin (unpin), more |
| Source group (PPP profile, DHCP server, Hotspot server or profile) | Group column | Name, subscriber count, rate source, parent | settings, link (parent), more |

The existing Sankey cards stay. The WAN and source-group card settings replace the Card
settings dialog for those card types.

### 7.3 Inline card settings

- The settings icon expands the card in place. The expanded card is at most 520 px wide.
  Its body scrolls at 640 px high.
- The camera moves so that the expanded card is in view at 100 % zoom. With reduced motion,
  the camera jumps.
- One card is expanded at a time. A second settings icon asks to keep or drop the open
  draft if it has changes.
- Below 60 % zoom, an expanded card shows only its title, state, and a "Zoom to edit"
  icon. Fields render only at 60 % zoom or more.
- The floating action bar (section 7.7) holds the save actions. The card holds no save
  button, except "Save connection" and "Add router" in the router connection section.

### 7.4 Workflow steps and their data

**A. Add router** (sources: `NewRouter`, `CredentialFields`, `DraftConnectionTest`)

1. The operator selects "Add router". The Add router card expands inline.
2. Fields: name, address, port, TLS (api-ssl), check certificate, CA file, username,
   password.
3. "Test" runs the draft test (`StudioJob::DraftTest`). The card shows each
   `TestCheck` stage (transport, TLS, authentication, identity, table permission) with
   word + icon + color and its safe `code`.
4. "Add router" saves the router (`POST /api/v1/routers`). The service saves a new
   router disabled.
5. The Add router card becomes a router card. Its settings open on the Connection
   section, so the operator can enable the router and run the registered test.

Router information after a test: the registered test (`/api/v1/routers/{id}/test`) gives
the RouterOS identity name and the elapsed time. The card shows only these. Version,
board, serial, uptime, and API capabilities need backend work (section 8).

**B. Router settings** (sources: `EditableSection`, Studio Access section)

Three sections in the expanded router card, as compact tabs with icons and text labels:

- **Connection:** the current connection edit, enable state, and test.
- **Access:** access profiles (max 8). Each profile has a label, username, password
  (write-only), and its write scopes. A read profile and a write profile can use different
  credentials.
- **Sources:** section C.

**C. Sources to read and to write**

- Read sources: check boxes for WAN, PPP, DHCP (IPoE), and Hotspot (`SourceKind`).
  Any combination is valid. Each source selects its read access profile.
- Tables per source come from `TableId` (for example PPP active, PPP secrets, PPP profiles).
  Show them as small chips under the source, not as separate controls.
- Write sources: only the scopes in `WriteScope`. Today this is "WAN address lists".
  Each write grant selects its access profile and has an enabled switch.
- Do not show PPP, DHCP, or queue write scopes. Section 8 lists them as backend gaps.

**D. Discover**

- The discover icon on the router card runs `StudioJob::Collect` for the selected
  sources. WAN evidence (`RouterSourceConfig.wan_candidates`) comes from the newest
  complete interface and route read. The current WAN read of `topology_inventory.rs`
  (`ReadJob::Wan`) moves to this icon.
- The tray shows the result. Each table shows its read state (complete, partial, failed,
  not requested, stale) and record count. Partial and failed never mean "zero".
- The recovery action stays for a pending read.

**E. Pin WAN interfaces**

- Each discovered interface row has a pin icon. Unpinned: outline pin, muted row.
  Pinned: filled pin and accent color.
- The pin action sends `wan.candidate_set` with `pin` through `POST /api/v1/topology/edits`.
  Ignore uses an eye-off icon (`ignore`). A second click on a filled pin sends `clear`.
- A pin only classifies the interface. It does not make a WAN node. The pinned row shows
  a draft WAN card in the WAN column with a dashed outline. Its settings ask for the
  capacity. Save of these settings sends the current `wan.create`. Then the card is solid.
- Expose pin and ignore only when the capabilities have `wan_evidence` and `policy_save`.
- Many WANs can be pinned (for example `pppoe-out1`, `pppoe-out2`).
- Unpin of a WAN that has topology children: show a confirmation in the card with the
  count of children. The service decides the result. If the service rejects it, show the
  service message. Refer to section 8 for the dependency check.

**F. WAN card settings** (sources: the Card settings `NodeForm` for a WAN node)

- Display name, source router and interface (read-only), capacity down and up
  (`Capacity::Fixed`), capacity factor, latitude and longitude (`NodeGeo`).
- Ribbon color and description are backend gaps. Do not show them.

**G. Source group settings (profile-first)**

The current source policy (`SourcePolicyDraft`) holds the rules. The group card is a view
of the policy that makes the group.

- A PPP router gets one default policy: source PPP, members active, group by
  `account.profile`. Each PPP profile then shows as one group card.
- Group card settings show:
  - **Rate order:** a numbered list of `RateSource` items. The first item that resolves
    wins. Examples: subscriber override, active comment, profile rate limit, profile map,
    fallback plan.
  - **Plan binding:** the `ProfileMap` binding of this profile to a plan.
  - **Capacity:** `CapacityRule::FromPlans` (down factor, up factor, minimum down, minimum
    up) or `Fixed`.
  - **Parent:** `ParentRule` (router or a node).
- A change that is only for one profile (a parent, a capacity) makes a new policy with the
  filter `account.profile = <name>` and a lower priority number. The editor shows this as
  "Own rules for this profile". The limit is 32 policies for each router.
- DHCP uses the same pattern with `lease.server` / `server.name` and `ServerMap`.
  Hotspot uses `hotspot.server` / `user.profile` and `ProfileComment`.

**H. Preview, save, apply**

- "Preview" runs `StudioJob::Evaluate`. The candidate cards show on the canvas as today
  (dashed, read-only overlay).
- "Save configuration" sends the complete set for the router, as today.
- The service writes `network.json` and `ShapedDevices.csv` and applies them in its own
  cycle. The bar shows Saved from the receipt and Applied from runtime evidence.

### 7.5 Router card actions

| Icon | Name (tooltip and accessible name) | Action |
| --- | --- | --- |
| gear | Router settings | Expand the router card on the last section |
| circular arrow | Refresh router | Run the registered connection test |
| radar | Discover | Run discovery for the configured read sources |
| three dots | More actions | Enable or disable the router (current `PATCH /api/v1/routers/{id}`) |

### 7.6 Icon set (all cards)

| Icon | Function |
| --- | --- |
| gear | Resource settings |
| pin (outline / filled) | Pin or unpin a discovered resource |
| circular arrow | Refresh or rediscover |
| i in a circle | Information (opens the current help workflow) |
| link | Set the topology parent |
| pencil | Edit the display name |
| three dots | More actions |
| eye-off | Ignore a discovered resource |

Each icon has a tooltip, an accessible name, and a visible pressed or active state.
Use the current `icon()` helper and the line-icon style of the canvas dock.

### 7.7 Floating action bar

- Fixed at the bottom center of the canvas viewport. It shows while a draft exists.
- Left: the draft status (`save.footer_status()`): Local draft, Saved, Pending apply,
  Applied, Failed.
- Buttons: "Preview" and "Save configuration", with their current disabled reasons.
- No "Discard" action. The current Studio has none.

### 7.8 Keyboard and focus

- Opening a card moves focus to its first field. Escape closes it and returns focus to the
  icon that opened it.
- Tab stays in the expanded card, then moves to the action bar. Collapsed cards outside the
  edit are `inert` while a card is expanded.
- Pan and zoom handlers ignore events whose target is an `input`, `select`, `textarea`,
  `button`, or `[contenteditable]` element. The wheel over a scrollable card body scrolls
  the body.
- Each card is `role="group"` with `aria-labelledby` its title. The state is in the name,
  for example "pppoe-out1, WAN, pinned".

### 7.9 Visual rules

- Use the current dark and light tokens. Do not add a palette.
- Mockup B look: number or type eyebrow in small caps, IBM Plex Mono for identifiers,
  cyan outline for the active card, green check for done, amber for partial.
- Spacing on the 4/8/16 px scale. Corners 10–14 px. No glow except the active outline.

## 8. Support matrix

| Owner workflow item | Status | Source |
| --- | --- | --- |
| Add router: name, address, port, TLS, verify, CA, username, password | Supported | `RouterEdit` |
| Test before save | Supported (stages and codes only) | `StudioJob::DraftTest` |
| Router information after test: identity name, elapsed time | Supported | `RouterTestResult` |
| Router information: version, board, serial, uptime, API capabilities | **Needs backend** | — |
| Read sources: WAN, PPP, DHCP/IPoE, Hotspot, combinations | Supported | `SourceKind`, `TableId` |
| Separate read and write credentials | Supported | access profiles, `read_access_ref`, `WriteGrant.access_ref` |
| Write: WAN address lists | Supported | `WriteScope::WanAddressLists` |
| Write: PPP profile, PPP secret, DHCP lease, queue | **Needs backend** | — |
| WAN discovery and pin / ignore / clear | Supported | `WanEvidence`, `wan.candidate_set` |
| Pinned WAN becomes a WAN node | Supported | current `wan.create` with capacity |
| Unpin dependency check | **Verify in backend** | Claude lane confirms the service rule |
| WAN capacity down/up, factor, latitude, longitude, name | Supported | `Capacity`, `NodeGeo`, node edits |
| WAN ribbon color, description | **Needs backend** | — |
| PPP profile-first groups | Supported | `GroupRule::ByField(account.profile)` |
| Rate order, plan binding, fallback, override | Supported | `RateSource`, `PlanBinding` |
| Node capacity multiplier and minimum | Supported | `CapacityRule::FromPlans` |
| Topology parent for a group | Supported | `ParentRule` |
| Subscriber data mapping (circuit ID, device name, MAC fields) | **Needs backend** | identity is `Registry` only |
| Naming templates | **Needs backend** | — |
| Per-subscriber min percent and max bandwidth | **Needs backend** | — |
| DHCP interface / VLAN association | **Needs backend** | — |
| Generate and apply `network.json`, `ShapedDevices.csv` | Supported (service) | save → service apply cycle |

A "Needs backend" item does not show in the UI. When the backend lane adds it, the
backend handoff gives the contract commit. Then the frontend adds it.

## 9. Routes in use

`/api/v1/routers`, `/api/v1/routers/{id}`, `/api/v1/routers/{id}/test`,
`/api/v1/routers/{id}/source-config`, `/api/v1/topology`, `/api/v1/topology/edits`,
`/api/v1/topology/reads`, `/api/v1/topology/source-capabilities`,
`/api/v1/topology/source-jobs`, `/api/v1/topology/source-candidates/{id}`,
`/api/v1/topology/operations/{key}`, `/api/v1/topology/throughput`.

## 10. Decisions

### 10.1 Decided by the owner (2026-11-24)

- Layout B. World-space cards.
- Router card–driven workflow (section 7).
- Minimal dialogs. Discovered resources replace the inventory screen.
- Icon-only pin. Pinned WANs become WAN cards with capacity settings.
- Codex builds it. This supersedes the earlier note "the Codex lane does not change the
  canvas" for this change only.

### 10.2 Builder calls (owner can change)

- The discovered tray is below its router card.
- One expanded card at a time. 60 % zoom limit for fields.
- Profile-only rules make a filtered policy ("Own rules for this profile").
- Topology settings, allocation, excluded sources, and help stay dialogs until phase 3.

## 11. Phases

1. **Phase 1 — frontend on the current contract.** Sections 7.1 to 7.9 with only the
   "Supported" rows of section 8. Remove the Studio dialog, the Add router dialog, and the
   Router inventory dialog. Remove Card settings for WAN and source-group cards.
2. **Phase 2 — backend gaps (Claude lane), then frontend.** Router information, write
   scopes, unpin rule, ribbon color, data mapping, naming templates, subscriber limits,
   DHCP VLAN. Each gap needs its own owner go and contract commit.
3. **Phase 3 — remaining dialogs.** Topology settings, allocation, excluded sources, and
   preview details as inline cards. A separate design.

## 12. Left to the builder

- Component split and file names.
- Tray row layout, connector curves, and camera animation time (max 250 ms).
- The compact text of each help line, if the meaning stays.
- Browser checks: update `studio-shell-check.cjs`, `studio-router-check.cjs`,
  `studio-preview-check.cjs`, and `topology-canvas-check.cjs` for the new structure.
  Keep `tools/frontend/check-all.sh` green.

## 13. References

- Stitch project "LQoSync v3 — Topology Studio": "Canvas Pipeline (Mockup B)",
  "Add Router Source Workflow", "Step 1: Setup, Credentials & Test Connection",
  "Step 3: Grouping Boundary & Auto-Balance Ingestion".
- Local copies: `docs/frontend/prototypes/topology-studio-v3/`.
- Canvas rules: `docs/superpowers/specs/2026-11-24-topology-sankey-canvas-design.md`.
- Frontend requirements: `docs/handoffs/2026-11-23-codex-topology-frontend.md`.
- API contract: `docs/frontend/TOPOLOGY_STUDIO_API.md`.
