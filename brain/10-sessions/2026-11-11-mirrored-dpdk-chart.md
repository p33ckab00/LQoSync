---
type: session
created: 2026-11-11
tags: [tui, dpdk, traffic, performance]
status: implemented-not-deployed
related: ["[[2026-11-10-dpdk-performance-and-deployment-review]]", "[[HANDOFF]]"]
---

# Combined mirrored DPDK chart

Updated: 2026-11-11T00:11:15+08:00 (Asia/Shanghai).

The owner approved a combined mirrored chart and requested implementation plus performance/reliability review.
All three sources start visible. Actual is blue, Mapped is amber, and Unmapped is violet.
Download appears above zero. Upload appears below zero on the same symmetric scale.
Only upload coordinates are negative. Stored rates, numeric values, and peaks stay positive.
Delivered and enqueue classes remain separate overlays. The chart does not add them.

## Controls and rendering

Legend clicks toggle sources. Uppercase A/M/U provide the same keyboard controls.
The v key cycles source focus and returns to the prior visibility selection.
The a key restores all sources. Source focus also reveals small or overlapping traffic.
Each source retains its own rates, freshness, age, and sample count.
The summary names the focused source, or the first visible source.
The Dashboard keeps its approved panel order and mouse scroll controls.

A bounded cache retains one report and the prepared source curves.
New reports, changed windows, source removal, clock regression, and time-window boundaries rebuild it.
Freshness is evaluated on every draw. The cache cannot keep expired or future points visible.
The initial fragmented-history stress test exposed expensive rendering for each segment.
The renderer now paints all segments on one canvas. Gaps remain independent.

No source polling, dependency, packet-path work, animation, or interpolated measurement was added.
The TUI retains its existing source-driven cadence and terminal diff rendering.
The render path performs no network or file reads.

## Verification

616 tests passed. Zero failed. The suite ignores seven manual tests.
Formatting, all-target Clippy with warnings denied, build, whitespace, and capture review passed.
Tests cover all themes, compact layouts, resizing, mouse targets, visibility, focus, unknown values, and stale labels.
They also cover mirror signs, symmetric bounds, cache identity, expiry, clock changes, and source removal.

Three 1,801-point histories took 1123.6 ms for 100 cached debug redraws at 240x40 cells.
Three histories with 901 isolated segments each took 797.6 ms for 100 cached debug redraws.
These are about 11.2 ms and 8.0 ms per redraw. They are synthetic results, not terminal frame-rate guarantees.
Logs: `/home/pi/.local/state/lqosync3/mirror-*.log`.
The interrupted pre-fix stress log remains as `mirror-per-segment-stress.log`.

Visual contract: `docs/design/tui/MIRRORED_TRAFFIC.md`.
Actual terminal captures use isolated synthetic records: `dashboard-mirror-v1.png` and `dashboard-mirror-unmapped-v1.png`.
The capture tool defaults to combined mode. Its explicit source options select focus.

## Handoff

The implementation is local. The production service remains on the previously verified `3.0.166+g828061e` release.
No production service, configuration, or router policy changed.
Preserve the unrelated local-profile Knowledge link and note.
The current code and visual contract are authoritative for this change. The project graph remains an older reference.
