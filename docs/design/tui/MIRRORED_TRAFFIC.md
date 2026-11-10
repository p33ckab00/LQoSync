# Combined mirrored traffic chart

Accepted extension: 2026-11-10. Implementation review: 2026-11-11.

## Composition and controls

The Dashboard retains its approved panel order. Its traffic panel overlays three sources:

| Source | Color | Meaning |
| --- | --- | --- |
| Actual | Blue | Whole-bridge delivered traffic |
| Mapped | Amber | Whole-bridge mapped enqueue traffic |
| Unmapped | Violet | Whole-bridge unmapped enqueue traffic |

Download appears above zero. Upload appears below zero with the same source colors.
Both halves use one symmetric scale. Axis labels identify D and U with positive magnitudes.
Upload values, peaks, and stored rates stay positive. Only upload plot coordinates are negative.

The chart does not stack or sum delivered and enqueue measurements.

All sources start visible. Click a legend row to toggle its trace.
Use uppercase `A`, `M`, or `U` to toggle Actual, Mapped, or Unmapped with the keyboard.
Use `v` or the Focus button to cycle Actual, Mapped, Unmapped, and the prior visibility selection.
Use `a` or All to restore all sources. An empty selection shows a notice.

A click during focus starts a visibility selection from the currently visible traces.
Focus reveals small unmapped traffic and separates overlapping traces.
Coincident traces can obscure each other in terminal cells. The legend retains each source's exact values.

Each legend row shows its own rates, freshness, age, and valid-sample count.
The summary shows peaks for the focused source, or the first visible source.
Peaks describe the selected time window. They remain historical observations during a source failure.
Missing fields stay unknown. A measured zero stays zero. Old observations show STALE.
Explicit missing samples and timestamp gaps longer than six seconds break the traces.
Future points and points outside the selected window cannot affect the curves or their peaks.

The `5m`, `15m`, and `1h` controls retain their history-availability behavior.
The minimum virtual body height increases by two rows to keep mirrored plots readable.
The existing Dashboard wheel and scrollbar retain access to all panels on compact terminals.
Controls map to the visible viewport after resizing or scrolling.
Dark and light themes use source colors. Mono retains labels, emphasis, and source focus.

## Rendering and reliability

Prepared curves are cached by telemetry report identity and time window.
A new report, changed window, source removal, clock regression, or point-boundary change rebuilds the curves.
The cache expires when a point leaves the window or a future point becomes eligible.
Freshness and text values are evaluated on every draw. Source focus does not require a source read.
The cache retains one report and at most the three bounded source histories.
The renderer paints all segments on one canvas. A zero-reference layer and at most three source layers retain their own colors.
It does not allocate one canvas or layer for each gap.
Symmetric bounds remain finite, including extreme finite cache values.

The TUI keeps its existing source-driven cadence and terminal diff rendering.
It adds no background poll, dependency, packet-path work, animation, or interpolated measurement.
The render path performs no network or file reads.

## Review evidence

The initial fragmented-history stress test exposed expensive rendering per segment.
The renderer now paints the independent segments on one shared canvas.
This preserves gaps without the repeated canvas allocation.

The complete suite passes 616 tests, with zero failed and seven manual tests ignored.
Formatting, all-target Clippy with warnings denied, build, and whitespace checks pass.
Tests cover mirror signs, preserved positive rates, shared bounds, visibility, focus, empty selection, and cache expiry.
They also cover clock changes, source removal, missing data, stale labels, all themes, resizing, and mouse targets.
The maximum fixture has three histories of 1,801 points each.
A second fixture has 901 isolated segments per source. Both fixtures verify reuse of prepared curves.
Synthetic debug redraw timings are recorded in `/home/pi/.local/state/lqosync3/mirror-performance.log`.
These timings do not guarantee a terminal frame rate on other hosts or SSH connections.

Actual terminal captures use isolated synthetic records:

- [Combined mirrored view](captures/dashboard-mirror-v1.png)
- [Focused Unmapped view](captures/dashboard-mirror-unmapped-v1.png)

The captures exercise the real TUI. They are separate from production telemetry measurements.
