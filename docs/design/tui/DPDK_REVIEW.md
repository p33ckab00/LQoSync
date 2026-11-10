# DPDK telemetry review

Date: 2026-11-10. Branch: `lqosync3`. Baseline: `a51338f`.

## Scope and load

This review covers all six [DPDK checkpoints](DPDK_STATS.md), their source transport, caches, lifecycle, TUI callers, and tests.
The service owns one monitor. Each TUI reads public caches and can own one manual circuit request.
Bounds remain 4,096 workers, 512 host summaries, 1,801 points per traffic history, and sixteen tins per circuit direction.
The review assumes the current host and the existing two-second aggregate and ten-second health/host schedules.

## Findings and fixes

1. An aggregate request could finish after configuration reload and publish to its previous destination.
   A regression test reproduced this case. The monitor now rejects the superseded result before publication.
2. Fixed Mbps precision could show small positive traffic as zero. Axis labels also rounded differently from their actual bounds.
   A shared formatter now selects bps, kbps, Mbps, or Gbps. Cache values remain in Mbps.
   Dashboard, classification details, subscriber details, and copied values use explicit display units.
   The chart uses clean upper bounds with matching labels. Zero and unknown remain different values.
3. The bridge health pane could report degradation without a corresponding Operational alert.
   Fresh reported degradation now adds a warning. An unavailable or stale health source adds an unknown-state notice.
   Long operational notices wrap and retain mouse scrolling. Pressure warnings retain their separate scope.

## Chart and text behavior

The approved panel order, source selectors, time windows, mouse targets, and scroll controls remain intact.
Actual stays delivered traffic. Mapped and Unmapped stay whole-bridge enqueue classes.
The legend separates LIVE, STALE, WARMING/GAP, and UNAVAILABLE states.
It shows sample age, the visible window's directional peaks, and the count of valid rate samples.
A window peak is historical data. It does not become a current rate during a source failure.
The plot keeps explicit gaps and also breaks lines across timestamp gaps longer than six seconds.
Future points and points outside the selected window cannot affect the plot or its peaks.
Each frame prepares the selected history once. It retains spikes without averaging or interpolation.

Source counters, intervals, units, and reset rules retain their verified contracts.
Host metrics remain exact circuit/IP matches from a capped sample. They are separate from collection state.
Circuit queues remain manual point observations. RTT, queue sojourn, and raw queue memory remain distinct measurements.
Dynamic text follows source updates. The TUI does not create intermediate measurements between samples.

## Performance and verification

The maximum chart fixture has 1,801 points. One hundred debug redraws at 240x40 cells took 770.1 ms.
That is about 7.7 ms per redraw. This synthetic debug result is not a production performance guarantee.
The existing cache, host ownership, lifecycle, and maximum-size tests also form part of the complete suite.
No dependency, source poll, public-cache budget, packet-path work, or router configuration was added.
The TUI render path performs no network or file reads.

613 tests passed. Zero failed. The suite ignores seven manual tests.
Formatting, all-target Clippy with warnings denied, debug build, and whitespace checks passed.
Regression tests cover reload races, plot gaps, bounds, units, zero/unknown values, stale text, and health warnings.
The running-terminal captures use isolated synthetic input files. They are separate from live probe evidence.

- [Dashboard](captures/dashboard-telemetry-v3.png)
- [Small unmapped traffic](captures/dashboard-unmapped-v2.png)
- [Subscriber observations](captures/subscribers-telemetry-v2.png)

Publication and deployment results belong in the session handoff and the deployment manifest.
The deployment must verify the installed hash, running process, successful cycle, collection, and both telemetry caches.
A root-only circuit probe must also verify the manual queue source after restart.
