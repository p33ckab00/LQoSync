# TUI visual review and transition assessment

Date: 2026-11-09. Time zone: Asia/Shanghai.
Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.

## Decision and scope

The web-style TUI is feasible within the current Ratatui application.
This conclusion follows from the local layout, event, loader, and Settings code.
The approved PNGs remain the visual baseline. No approved image changed in this review.
The Journal PNG is a new proposal. Owner approval remains pending.

This task updates documentation and creates the Journal proposal. It does not implement the remaining screens.
The existing shared-shell work remains in the worktree. Production configuration and services remain unchanged.

## Review method

Inspect each canonical PNG directly. Compare panel order, relative widths, spacing, colors, labels, focus, and selection.
Compare the current source and first-slice terminal frames with those references.
The first-slice frames use synthetic data at 80x24, 120x40, and 160x48.
They are terminal-cell exports, not proof of full screenshot parity.

Reference images have a 2172x724 raster size. Terminal fonts and cell sizes affect final raster dimensions.
Compare actual captures at a matched viewport and font. Preserve panel geometry in terminal cells.
Check dark-theme appearance against the PNGs. Check light and monochrome themes separately for usability.

## Screen comparison

| Reference | Required composition confirmed by image review | Current implementation gap |
|---|---|---|
| [Dashboard](mockups/dashboard-dpdk-v2.png) | Overview row; five cards; DPDK chart left, health right; planned WAN table; alerts. | Existing process dashboard body remains. Add evidence-based cards, DPDK rates/history, health, and alerts. |
| [Subscribers](mockups/subscribers-approved-v1.png) | Three cards; four filters and Clear; table/details split; pagination; data status. | Existing device view remains. Add the approved body, filters, details, and online-state evidence. |
| [Routers](mockups/routers-approved-v1.png) | Configured/Reachable/Failed reads cards; search/state/Clear; table/details split; data status. | Temporary cached text view. Add table, selected details, counts, filters, and links to Subscribers/Settings. |
| [LibreQoS](mockups/libreqos-approved-v1.png) | Action row; compatibility notice; last/remote sync panels; tabbed source preview; data status. | Temporary cached status view. Add target-specific preview and evidence-based import/compatibility states. |
| [Workers](mockups/workers-approved-v1.png) | Three cards; filters; utilization table/details; pressure panel; snapshot status. | Existing worker view remains. Adapt its body and interactions without changing telemetry semantics. |
| [Settings: Add Router](mockups/settings-add-router-approved-v1.png) | Config/draft strips; group list; router list; full form; review/apply; result strip. | Existing wizard/editor remains. Build the displayed form and mouse targets around its draft and secret controls. |
| [Settings: Values](mockups/settings-values-approved-v1.png) | Group list; loaded/draft/unit table; help; diff; review/apply; result strip. | Existing editor remains. Add the approved columns, focus behavior, diff placement, and explicit gated actions. |
| [Journal proposal](mockups/journal-proposal-v1.png) | Three cards; toolbar; log table/details; selection strip; stream status; footer. | Proposal only. Reuse current workflows; add parsing, stream evidence, details, and scrollbar interaction. |

The shared shell now provides the navigation order, wide sidebar, top bar, and local theme control.

The old body style still uses different borders and gaps. Full screenshot parity does not pass yet.
The shell displays `SYNC AGE`, while approved images use screen-specific freshness labels.
Keep that interim label truthful until each source has its own freshness state.

The screenshot footer uses Tab for control focus. The current app uses Tab for route cycling.
Before form adaptation, define focus traversal and retain keys 1 through 7 for direct page navigation.
Do not pass Tab to global navigation while an editor owns input.

## Journal image review

The new image uses the Workers and Dashboard PNGs as style references.
The sidebar width, groups, active marker, top bar, footer, palette, and border style match that visual system.
The eight displayed messages contain the search word `reload`.
The selected 10:14:42 ERROR message matches its detail panel. Follow is paused in all displayed controls and status labels.
The source card and entry details both show `lqosync3.service`.
The scrollbar, copy actions, follow controls, and text-selection strip are visible without clipped panels.
The image retains a `DESIGN PROPOSAL | SAMPLE DATA` label.

The entry-detail panel omits a separate title above its timestamp. Its visible title is `Selected entry`.
Use that actual label. Do not create extra fields from the generation prompt.
All sample counts and states require runtime evidence before display as operational values.
The full interaction and data requirements are in the [Journal section](README.md#journal-proposal).

## Existing implementation foundation

| Source | Reusable capability | Work still required |
|---|---|---|
| `src/tui/mod.rs` | Terminal event loop, mouse capture, worker channels, terminal cleanup. | Preserve cleanup and input ownership across new controls. |
| `src/tui/shell.rs`, `app.rs` | Seven routes, sidebar, theme, modal guards, resize target reset. | Match final screen freshness, focus, and status layouts. |
| `src/tui/data.rs` | Background cached-data loading. Draw reads memory. | Add missing fields and freshness outside draw; bound parsing and retained data. |
| `src/worker_util.rs`, `worker_util/ipc.rs` | Read-only analytics integration and telemetry validation. | Extend the read model for verified directional traffic counters and bounded history. |
| `src/tui/settings/` | Typed draft, validation, preview, secret controls, conflict checks, apply verification. | Reuse these actions in the approved form layout. Preserve all gates. |
| `src/tui/journal.rs`, `osc52.rs` | Bounded lines, search, minimum-level filter, follow, selection, bounded copy. | Add details, lifecycle/receive evidence, reliable field parsing, and complete mouse controls. |

The code already separates background loading from drawing. Keep that separation.
No new UI framework is required for the approved terminal composition.
These conclusions do not establish performance under every production dataset.

## Transition order and acceptance

1. Keep the shared shell. Define consistent focus and action ownership before adding forms.
2. Implement Dashboard from its approved PNG. Verify DPDK counter direction and reset/gap handling before plotting rates.
3. Adapt Subscribers, Routers, LibreQoS, and Workers. Reuse current data paths and preserve unavailable/stale states.
4. Adapt Settings around existing draft/apply behavior. Verify router creation and typed edits with isolated fixtures.
5. Adapt Journal after the owner reviews its proposal. Preserve current behavior in the meantime.
6. Complete control clicks, per-panel wheel routing, scrollbar track clicks, and thumb dragging across all screens.
7. Capture actual terminals and compare each screen against its canonical PNG. Check compact layouts separately.

Require two valid producer samples for a traffic rate. Reset after counter decreases, restarts, or invalid sample intervals.
Use chart gaps for unavailable traffic. Do not substitute process network totals or configured bandwidth for traffic.
Use real collection evidence for subscriber online state. Keep planned WAN and Profiles controls inactive.
Test empty, loading, permission denied, unavailable, stale, failed, and populated states.

For Settings, test exact-draft preview, cancellation, conflicts, masked secrets, backup, confirmation, reload, and result verification.

For Journal, test retention rollover, paused selection, search/filter changes, stream exit, permissions, long messages, and copy fallback.

For mouse input, test dialog click-through, stale resize targets, independent scrolling, and keyboard action parity.

The minimum terminal remains 80x24. Compact adaptation must preserve access to all required panels and actions.
The wide mockups do not approve an arbitrary compact redesign.
Use bounded visible-row rendering and bounded history. Measure large-data responsiveness before release.
Report screenshot parity only after both visual comparison and functional checks pass.

## Terminal style and browser delivery

The requested terminal evolution can provide sidebar navigation, cards, tables, details, forms, and mouse interaction.
The application still runs inside a terminal. Its layout uses terminal cells and the terminal's font and clipboard support.
If browser access becomes a separate requirement, it needs its own delivery plan, authentication, API, and deployment design.
That browser work is outside this terminal design task.

## Verification of this documentation task

Review all seven existing canonical views and the new Journal image directly.
Verify all fifteen PNG dimensions and stored hashes. Verify existing hashes remain unchanged.
Check local Markdown links and whitespace. Do not rerun application tests for image and prose changes alone.
The earlier 516-test result belongs to the shared-shell slice. This review does not establish new runtime acceptance.
