# LQoSync4 visual and responsive UI map — proposed

This document defines the LQoSync4 layout and visual hierarchy. Dimensions below are **implementation starting values**, not fixed pixel requirements. Verify them in a browser on real desktop and phone viewports. The UI uses original LQoSync4 markup, icons, and styling. [Route and component contract](ROUTES_AND_COMPONENTS.md) defines the behavior. [Reference CSS](../ui-reference.css) is a concrete styling starter for the future Leptos app; it is not loaded by the current LQoSync3 program.

## Page inventory and placement

| Page/view | Layout and text order | LQoSync4 component |
|---|---|---|
| IPoE Profiles | Sidebar at left; title and `New Profile` on one row. Each full-width profile card starts with name, green down/up speed badge, then muted subscriber/override count. Actions align right. Expanded card places help text, override table, then inline form: name, day checkboxes, start/end, download min/max, upload min/max, priority, and `Add override` at the lower right. | `IpoeProfilesPage`, `ProfileCard`, `BaselineBadge`, `ScheduleTable`, `ScheduleEditor` |
| PPPoE Users | Dense table below the shared shell. Columns are subscriber, MAC/IP, online, router, profile select, speed, override, status, last seen, and actions. Status badges use green. `Disable` and `Override` stack at the right edge. | `PppoeUsersPage`, `PppoeUserTable`, `EffectiveSpeedCell`, `RowActionMenu` |
| PPPoE Profiles | Title; router filter, `Sync All`, `New profile` at top right. Full-width compact profile cards show name, router chip and speed badge on first line; description, users and override count on second; actions right. | `PppoeProfilesPage`, `RouterFilter`, `ProfileCard` |
| LibreQoS | Title with `Sync now` right. A full-width missing-parent alert appears immediately below the header when the latest native cycle has affected circuits; its affected-circuit list opens below it. Then come the `Last sync` card, `Remote sync` card, and large scrollable input preview card. Status and timestamps are stacked labels/value lines, with status badge in the value. | `LibreQosPage`, `MissingParentAlert`, `MissingParentList`, `SyncStatus`, `RemoteSyncStatus`, `SourcePreview` |
| Load Balancer Allocations | `LB1` title, core/poll/status metadata, then five metric cards in one row. Four tabs below. `Add Allocation` right above a three-column allocation-card grid. Each card has WAN name and badges, policy and capacity line, icon actions, member/target/utilization values, RX/TX, then a traffic graph. | `LoadBalancerPage`, `BalancerSummary`, `TabBar`, `AllocationCard` |
| Load Balancer Assignments | Same title/metrics/tabs. Search bar above full-width grouped table. Columns: customer, circuit ID, MAC, IP, online, allocation, address list, synced, status, actions. WAN selection and pin/move actions stay at the right. | `AssignmentsTab`, `AssignmentTable`, `AssignmentRow` |
| Dashboard top | Five summary cards: IPoE subscribers, PPPoE online, PPPoE offline, routers, LibreQoS. Below, left traffic chart with time-window tabs; right allocation breakdown with donut and WAN legend. | `DashboardPage`, `SummaryCards`, `TrafficChart`, `AllocationBreakdown` |
| Dashboard lower | Full-width upstream health panel with small status chips in four columns. Below it, load-balancer overview table with allocation, members, capacity, RX, TX, utilization, status. | `HealthGrid`, `AllocationSummaryTable` |

The UI must use current backend data and must not include fixture customer records or literal counts. The monitor provider is configurable. Do not sum per-WAN utilization percentages for the donut center. Compute an aggregate with a documented denominator, or label the view as per-WAN distribution without a total percentage.

## Shared desktop composition

```text
┌─ sidebar 256 px ─┬─ top bar 48 px: local time · data age · theme/account ───┐
│ logo             │ page header: title left, primary action right             │
│ Dashboard        ├───────────────────────────────────────────────────────────────┤
│ Routers          │ page content, 24 px side padding, 16 px section gap        │
│ IPoE             │ metric cards / charts / health / tables or profile cards   │
│ PPPoE            │                                                              │
│ System           │                                                              │
│ server status    │                                                              │
└──────────────────┴───────────────────────────────────────────────────────────────┘
```

- At `>= 1200px`, sidebar width is 256 px. Main content is `minmax(0, 1fr)` and may grow across a wide monitor; cards use the available grid rather than a narrow fixed center column. Main page padding is 24 px, with 16 px between panels and 12 px between small controls.
- The top bar is 48 px high. A thin bottom border separates it from the page. The page header starts 24 px below it. Heading and primary action share one flex row; secondary metadata sits directly below the title. Chart and table panels begin after a 20 px vertical gap.
- Surface background is nearly black; sidebar and cards are slightly lighter. One-pixel borders separate cards. Corners are 14–16 px on large cards and 8–10 px on buttons/inputs. Do not rely on shadow to show a boundary.
- Use an icon with each main sidebar label. The selected item has a lighter charcoal rounded background. Section labels (`IPoE`, `PPPoE`, `System`, `Server Status`) use small uppercase muted text and spacing, not an extra card.
- Default text is at least 14 px. Compact metadata may be 13 px if contrast stays strong. Page heading is 26 px; card heading 16 px; metric number 28 px; tab/button text 14 px. Use tabular numerals for rates, counts, percentages, and timestamps.

## Text and card anatomy

| Element | Alignment and content order | Width/spacing rule |
|---|---|---|
| Metric card | Muted label top left; large number below; trend or qualifier on the same number line; timestamp or explanation last. | 18 px padding; five equal columns on wide dashboard; no truncated number. |
| Profile card collapsed | Name and router chip first; green `↓ / ↑ Mbps` badge next; actions right. Description/count row below. | 16 px padding; `minmax(0, 1fr)` text area; actions never overlap name. |
| Profile card expanded | Header stays; helper sentence; schedule table; edit form; right-aligned save action. | Grid form uses 12 px gaps; day toggles wrap; labels stay above inputs. |
| Allocation card | Name and state badges; policy/capacity sentence; icon action row; three metrics; RX/TX line; compact graph. | 18 px padding; minimum 300 px card width before wrap. |
| Status row | Name left; freshness/percentage and state right. | Text may wrap, but status word never disappears into color only. |
| Dense table row | Primary identifier first; muted secondary data underneath; status in its own cell; action cell at end. | Row height about 64–80 px; table has horizontal scroll; desktop action column sticks to right. |
| Long identifiers | Full value on focus/hover or in a detail panel; clipped display uses ellipsis. | Never ellipsize a subscriber ID in an export, audit, or copy action. |
| Missing-parent alert | Warning/critical icon and “Parent Node missing” label first; circuit and row counts next; mode/last-cycle time and exact age on the next line; `View affected circuits` link last. The list shows Circuit ID/name and rows per circuit. | Full-width panel directly under `/libreqos` header; use text and state word with color. Keep counts and link visible at 320 px and 200% zoom. Dashboard shows only the count and link in its LibreQoS metric card. |

`↓` consistently means download and `↑` means upload in speed badges. Pair rates with explicit `Mbps` in card summaries. Timestamp text includes timezone; relative age can appear beside the exact timestamp. In the profile schedule form, `min/max` labels stay visible instead of relying on placeholders. A badge always includes a word (`Online`, `Down`, `Unknown`, `Imported`) as well as color.

## Responsive layout contract

| Viewport | Shell and navigation | Content grids | Dense data and actions |
|---|---|---|---|
| `>= 1200px` desktop | Fixed 256 px sidebar; 48 px top bar. | Dashboard metrics 5 columns; charts 2; health 4; allocations 3. | Tables scroll in their panel; sticky action column; toolbars one row. |
| `768–1199px` small desktop/tablet | Sidebar 220 px when space allows; otherwise drawer. | Metrics 2–3; charts 1–2; health 2; allocations 2. | Filters wrap; tables keep horizontal scroll. |
| `< 768px` phone | Sidebar becomes overlay drawer with visible menu button; top bar shows compact time/status. | All charts, panels, profiles, and allocation cards 1 column; metric cards 2 columns where they remain legible. | Table views show key fields in stacked record cards or a scrollable table with persistent action access; no clipped action buttons. |
| `< 390px` narrow phone | Same drawer, 16 px page padding. | Metric cards 1 column if two columns would crush the number/label. | Inline profile override form becomes one column; action buttons full width. |

At every width, page and card content use `min-width: 0` so charts and long IDs cannot force the whole app wider than the viewport. A horizontal scrollbar belongs to the table panel, not to the page. The CSS includes an optional `record-list`/`record-card` phone presentation for the same paged rows; Leptos renders only the active presentation, or marks the inactive one hidden from assistive technology. Dialogs use the viewport height and scroll their body; the title and action row remain visible. Touch controls have at least a 44 px target. The menu button, tabs, status text, and primary action remain keyboard accessible. A desktop layout that hides actions on a phone is not acceptable.

The phone drawer has a scrim, closes with Escape and after navigation, and returns focus to its menu button. When closed, its links are removed from keyboard focus. A dialog traps focus while open and returns focus to its trigger when closed. These behaviors belong to Leptos components; CSS only sets their visible layout.

## Loading, empty, and error layers

- Keep the page header and panel sizes stable while data loads. Show a short skeleton for cards or a labeled `Loading…` region; do not flash zero as an interim subscriber count.
- If a chart has no samples, keep its title and time-window control and show `No traffic samples for this period`. Do not draw a flat zero line unless the source reported zero.
- If a table has no results after filtering, show `No matching subscribers` with a clear-filter action. A genuine zero-user network uses a different empty message.
- A collector or monitor error appears inside the affected card and in its status summary. Keep the last known value with its age; mark it stale. A failed write shows the server error and audit/job ID, not a green success badge.
- A missing-parent alert stays visible on `/libreqos` until a newer clean native cycle. Show `Candidate incomplete` in shadow mode and `Apply blocked; live input unchanged` after rejected apply. A stale snapshot shows `Last known` and its age, not a green zero. The [alert contract](MISSING_PARENT_ALERT.md) defines counts, detail paging and remediation.
- Toasts announce outcomes briefly, but persistent job and import status remains on the relevant page. Screen-reader live announcements should report state changes without replaying every traffic sample.

## Visual tokens and CSS ownership

The [reference CSS](../ui-reference.css) defines the starter tokens and layout classes. `AppShell` owns grid placement. Pages own their grids. Components own internal spacing and typography. Avoid route-specific global selectors that change another page's cards. A semantic color token owns each state: healthy, warning, critical, unknown, accent. Use text and icon/state words with color. Check contrast for ordinary text and badges. Keep small metadata legible.

## Performance and visibility review

| Finding from the design | Improvement | Verification |
|---|---|---|
| Subscriber and assignment tables can show hundreds or thousands of users. | Server-filter and page by stable key; render only visible rows. Keep sort/filter/page in the URL. | 1,000 and 10,000 records: bounded DOM nodes, API p95, memory, and scroll smoothness. |
| The LibreQoS page can display a large raw CSV preview. | Show a paged or virtualized preview; allow explicit download. Do not parse/render all rows on each poll. | 1,000-row preview and large-file test without a full DOM mount. |
| Dashboard repeats traffic, health, counts, and WAN data across cards. | Read one versioned summary snapshot for counts and age; fetch bucketed chart data separately; refresh changed panels only. | A slow router does not block first paint; snapshot version is consistent across summary cards. |
| Many WAN cards contain live graphs. | Use bounded, time-bucket samples; pause graph updates offscreen; keep a text RX/TX value available. | Nine and many-WAN profile: frame time, memory, network bytes, background-tab behavior. |
| Realtime badges may imply false health when the feed is stale. | Show `observed_at`, `updated_at`, source, and stale/unknown state; do not reuse old green status as current. | Disconnect monitor/collector; UI changes to stale without changing the historical value. |
| A load-balancer donut can sum unlike percentages. | Define utilization as traffic/capacity per WAN; use capacity-weighted aggregate only with matched samples, or omit center total. | Fixture with different WAN capacities produces a mathematically valid label. |
| Dense navigation and small labels limit readability. | Use 14 px body, high-contrast muted text, visible keyboard focus, wrap/tooltip for clipped IDs, and 44 px touch controls. | Inspect 1440×900, 1024×768, 390×844, 320×640, 200% zoom, keyboard-only. |
| Hidden mobile action columns make moves and overrides risky. | Use row cards with explicit primary actions or a sticky action column inside a scroll panel. | Every write action remains visible and reachable at 320 px. |

Measure actual performance after the Leptos slice exists. Targets for transfer size, interaction time, p95 latency, and memory need a baseline on representative hardware. The layout map alone cannot prove runtime performance.
