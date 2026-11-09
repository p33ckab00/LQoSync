# TUI visual review and transition assessment

Date: 2026-11-09. Time zone: Asia/Shanghai.
Branch: `lqosync3`. Worktree: `/home/pi/LQoSync3.0`.

## Decision and scope

All seven approved screen bodies now run in the existing Ratatui application.
The owner approved the unchanged Journal image before this implementation.
The fifteen concept PNGs retain their original hashes. No production service or configuration changed.
Each screen has a separate local checkpoint. Push and deployment are deferred by the owner.

## Comparison method and evidence

Compare panel order, proportions, labels, palette, selection, and action placement against the canonical concepts.
The [capture index](captures/README.md) links the running-terminal images and Settings fixture renders.
Captures use isolated synthetic data. They do not expose operator records or prove live router connectivity.
Terminal font metrics and cell borders differ from the generated concepts. This review does not claim pixel identity.
The final frame matrix covers seven routes, three themes, and three sizes: 80x24, 120x40, and 160x48.
Compact bodies retain panel access through scrolling. The wide concept remains the visual baseline.

## Screen comparison

| Reference | Implemented composition | Evidence and limits |
|---|---|---|
| [Dashboard](mockups/dashboard-dpdk-v2.png) | Five cards; DPDK chart/health split; planned WAN; alerts. | [Terminal](captures/dashboard-collection-v2.png). Actual counters require two producer samples. Gaps and stale data remain explicit. |
| [Subscribers](mockups/subscribers-approved-v1.png) | Cards; search/source/router/state controls; table/details; paging; status. | [Terminal](captures/subscribers-collection-v2.png). Cycle evidence now supplies session/account and lease states. Missing/stale records remain unknown; collected-only accounts remain outside shaping. |
| [Routers](mockups/routers-approved-v1.png) | Three cards; filters; table/details; source flags; navigation; status. | [Terminal](captures/routers-collection-v2.png). Reachability is the last collection result. |
| [LibreQoS](mockups/libreqos-approved-v1.png) | Actions; compatibility; sync cards; source tabs; selection; status. | [Terminal](captures/libreqos-v1.png). Native input stays distinct from compatibility output. Remote sync is not configured. |
| [Workers](mockups/workers-approved-v1.png) | Cards; filters; utilization/details; pressure; snapshot status. | [Terminal](captures/workers-v1.png). Fresh complete diagnostics are required for CLEAR. |
| [Settings router form](mockups/settings-add-router-approved-v1.png) | Categories; routers; complete form; masked secret; review/apply; result. | [Fixture render](captures/settings-add-router-v1.png). Trial-draft validation and one-step undo preserve integrity. |
| [Settings values](mockups/settings-values-approved-v1.png) | Loaded/draft/unit columns; help; cached diff; guarded actions. | [Fixture render](captures/settings-values-review-v1.png) and [read-only terminal](captures/settings-readonly-terminal-v1.png). No privileged terminal session was available. |
| [Journal](mockups/journal-proposal-v1.png) | Cards; toolbar; table/details; selection; stream status. | [Terminal](captures/journal-v1.png). Prefix severity is explicit; raw copy is preserved; expired selection cannot copy a replacement record. |

The shared shell retains sidebar groups, local theme control, and direct page keys.
Journal uses LOG AGE. Other headers use SYNC AGE; individual panels state their own source freshness.
Tab focuses controls in adapted screens. Active inputs and dialogs own their keyboard and mouse events.

## Performance and integrity review

Draw reads memory. Background loaders perform file and socket I/O.
Filters and derived views rebuild on source/input changes. Tables render visible rows.
Settings review caches field changes; drawing does not perform a quadratic text diff.
DPDK history has at most 1,801 points. Rates use producer deltas and reset after invalid intervals or counter regression.
Journal caches lowercase search text, retains the configured line limit, queues at most 2,048 messages, and drains at most 512 per loop.
Journal copies original retained records. Display sanitizes control characters. Severity requires an explicit prefix, otherwise UNKNOWN.
Quiet streams remain connected until exit or error evidence. Paused selections survive rollover while their records remain.
Settings preserves read-only checks, exact-draft preview, secret revisions, confirmation, conflicts, backups, rollback, reload, and digest verification.
Shared scrollbars route wheel, track clicks, and thumb drags to their panels. Resize clears old hit targets.
Global and Settings dialogs block background actions and use the existing confirmation paths.

## Verification

The full suite passes: 552 tests, zero failures, seven ignored manual tests.
Formatting, all-target Clippy with warnings denied, debug build, whitespace, and stored concept hashes pass.
The final frame export produces 63 frames. Isolated PTY navigation, theme, resize, quit, and terminal cleanup pass.
Focused tests cover DPDK resets/gaps, source freshness, input ownership, scrolling, Settings forms/gates, and Journal selection/stream lifecycle.
These tests and bounded work do not establish latency under every production dataset.

## Deployment limits

The new service monitor must be deployed before the live cache contains DPDK traffic history.
Sudo requires an operator password, and user namespaces are restricted. Writable Settings images are labeled fixture renders.
A privileged running-terminal save/reload check remains a deployment check. Existing isolated apply/secret/rollback tests pass.
No install, service restart, or push occurred. WAN plans and Profiles remain planned.
Subscriber state now uses the cycle collection cache. Unknown external account inventory and unavailable native row counts remain explicit.

## Terminal style and browser delivery

The terminal now supplies the approved sidebar, cards, tables, forms, detail panels, and mouse controls.
It still runs in a terminal and uses that terminal's font and clipboard support.
A separate browser product requires its own authentication, API, delivery, and deployment design.
That work is outside this implementation.
