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
