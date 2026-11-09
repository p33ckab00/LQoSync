---
type: session
created: 2026-11-09
tags: [session, tui, implementation]
status: in-progress
related: ["[[2026-11-09-tui-shell-implementation]]", "[[2026-11-09-journal-design-and-tui-evolution]]", "[[HANDOFF]]"]
---

# Approved TUI screen implementation

Owner authorization: implement Dashboard through Journal, review each screen, test and capture, then commit locally.
Do not push. The owner approved the unchanged Journal image.
Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.

## Dashboard checkpoint — 2026-11-09T14:05:42+08:00

Implemented five summary cards, real DPDK actual aggregate rates/history, dataplane health, planned WAN, and alerts.
Added read-only counters to the worker snapshot/cache path. History retains at most 1,801 points over one hour.
Two producer samples establish a rate. Duplicate frames, missing counters, resets, errors, and gaps cannot generate a false spike.
Older caches remain readable. The monitor must run this code to populate the new history. No service was restarted.
The chart marks unavailable/stale data. PPPoE-offline values remain unknown without session evidence.
Click/key Refresh wakes the cache loader. It does not trigger a sync.
Compact body and alerts support wheel, track click, and thumb drag. Modal guards remain active.

Review: drawing performs no I/O. History and wire/cache sizes are bounded. Cache validation rejects invalid or unordered points.
Test evidence: 521 passed, zero failed, six manual tests ignored. All-target Clippy with warnings denied passes.
Formatting, whitespace, and debug build pass. Actual Zutty/Xvfb screenshot uses isolated synthetic fixture data.
Capture: `docs/design/tui/captures/dashboard-v1.png`. Reproduce with `scripts/dev/capture_tui.py`.
The composition follows the approved reference. Terminal font/border pixels differ; the screenshot does not claim pixel identity.
Checks/logs: `/tmp/lqosync3-dashboard-*.log`.

Remaining: Subscribers, Routers, LibreQoS, Workers, Settings, Journal. Continue after this local checkpoint.
No push, production configuration write, installation, or production service restart.

## Subscribers checkpoint

Implemented approved cards, source/router/state filters, table/details, selected ID, paging, expanded details, copy, and parent navigation.
The row index and sort cache remain authoritative. The renderer builds only visible rows; the focused test uses 1,000 records.
Missing online evidence stays unknown. Copy uses the existing OSC 52 size limit and reports an unconfirmed clipboard result.
Search owns typed input; its Clear action remains clickable. Background navigation stays blocked during active search and dialogs.
Table and details have independent wheel and interactive scrollbars. Left/Right exposes wide table columns.
Actual Zutty/Xvfb capture: `docs/design/tui/captures/subscribers-v1.png`, using synthetic data.
Checks: 523 tests pass, zero fail, six ignored; all-target Clippy, formatting, whitespace, and build pass.
Next: commit locally and continue Routers. No push or production changes.

## Routers checkpoint

Implemented configured/reachable/failed cards, filters, list/details, enabled-source flags, paging, and status.
Config changes trigger parsing; collection changes update cached results. Selection survives refresh.
Actions navigate to filtered Subscribers or the selected router Settings without modifying the draft.
Copy returns only the endpoint, including correctly bracketed IPv6 addresses. It cannot include credentials.
Table/detail scrollbars, wheel input, keyboard filters, and mouse controls share guarded paths.
Checks: 525 tests pass, zero fail, six ignored; Clippy, formatting, whitespace, build, and actual capture review pass.
Capture: `docs/design/tui/captures/routers-v1.png`. Production unchanged. Next: LibreQoS after local commit.

## LibreQoS checkpoint

Implemented action row, compatibility notice, sync cards, source files, paging/selection, and data status.
Only the service's verified pair-import result displays IMPORTED. Native file writing and watcher status remain separate.
Native source preview retains the original loaded JSON. JSON formatting runs only when the corresponding Arc changes.
Persistent parent diagnostics survive read errors and clear only after a newer valid ready result without those alerts.
Sync key/click uses the existing read-only, confirmation, and result gates. Preview performs no write or network request.
Compact layout keeps all panels on a scrollable body. Local timestamps include seconds and the UTC offset.
Checks: 527 tests pass, zero fail, six ignored; all-target Clippy, formatting, whitespace, build, and real terminal capture review pass.
Capture: `docs/design/tui/captures/libreqos-v1.png`. Removed the obsolete temporary operations view.
Next: local commit, then Workers -> Settings -> Journal. Production unchanged.

## Workers checkpoint

Implemented summary cards, Worker ID/Sample filters, utilization table, selected-worker bars/details, pressure, and snapshot status.
Filters rebuild only when telemetry/filter/freshness changes. Timer expiry updates Live/Stale filters without loader input.
Only visible worker rows render. Copy contains selected worker evidence through the existing bounded clipboard path.
Pressure requires fresh complete diagnostics for CLEAR. Missing/stale samples never indicate recovery.
Table/details/compact body have independent wheel routing and interactive scrollbars.
Checks: 529 tests pass, zero fail, six ignored; Clippy, formatting, whitespace, build, and real terminal screenshot review pass.
Capture: `docs/design/tui/captures/workers-v1.png`. Next: local commit, Settings, then Journal. Production unchanged.

## Settings checkpoint

Implemented category/group navigation, loaded/draft/unit values, cached review, field help, and review/apply controls.
Added the full router form with TLS/source/CA controls. A validated trial draft prevents partial additions and duplicate-router changes.
A router addition is one undo step. Typed secrets remain in Secret memory until existing apply writes them; cancel/undo drops them.
Mouse input covers fields, choices, lists, toggles, router selection, reset/undo, preview/apply/discard, and dialogs.
Inline field edits cannot reuse an old preview. Sidebar clicks preserve an active field. Read-only, confirmation, conflict, backup, and reload verification gates remain.
Fixed partial-viewport mouse target clipping in shared code, including row offsets in compact worker/source previews.
Review lines render only the visible range. Help and all overflowing Settings panels have independent mouse scrolling.
Checks: 534 tests pass, zero fail, seven manual tests ignored; Clippy, formatting, whitespace, build, and screenshot review pass.
Actual terminal capture is read-only. Writable Add Router and values/review images come from actual Ratatui fixture frames.
Sudo needs an operator password and user namespaces are restricted; no privileged capture or production change occurred.
Captures: `docs/design/tui/captures/settings-*.png`. Reproduce fixture frames with the ignored Settings exporter and `scripts/dev/render_terminal_frame.py`.
Next: local commit, then Journal. Do not push.
