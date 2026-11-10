# Combined mirrored DPDK chart

Date: 2026-11-10. Branch: `lqosync3`. Baseline: `1e59983`.

## Accepted behavior

Show Actual, Mapped, and Unmapped together. Use blue, amber, and violet.
Download appears above zero. Upload appears below zero with the same source colors.
Both halves share one symmetric scale. Display values retain positive upload rates.
The chart overlays traces. It does not stack or sum delivered and enqueue counters.

The default view shows all sources. Each legend row toggles its source.
The `v` key cycles source focus and returns to the prior visibility selection. The `a` key restores all sources.
Mouse controls and time windows remain available after scrolling and resizing.
Each source retains its own freshness, unknown values, age, sample count, and window peaks.
Keep missing samples and time gaps. Focus permits inspection of small unmapped traffic.

## Implementation and verification

1. Add tests for mirror signs, symmetric bounds, independent visibility, focus, cache invalidation, and time-window expiry.
2. Reuse the current history and scale helpers. Cache prepared curves by report identity and time window.
3. Rebuild at the next point expiry or future-point arrival. Recalculate freshness on each draw.
4. Keep the approved panel order and source-driven update cadence. Add no polling, dependency, animation, or interpolation.
5. Test all themes, compact layouts, mouse controls, stale states, empty selection, and maximum histories.
6. Measure repeated redraws, run the complete checks, and inspect actual terminal captures.
7. Update visual documentation and project notes. Keep the production service unchanged during this implementation.

## Result: 2026-11-11

The combined chart, source toggles, source focus, and symmetric mirror are implemented.
Uppercase A/M/U provide keyboard toggles. Each source retains independent freshness and positive numeric upload values.
Curves use one bounded cache. One canvas paints every segment without connecting gaps.
The fragmented-history stress test exposed the prior per-segment rendering cost before this correction.

616 tests passed, zero failed, seven ignored. Formatting, Clippy, build, and whitespace checks passed.
All themes, compact layouts, resizing, mouse targets, source removal, clock changes, and expiry boundaries passed.
One hundred cached debug redraws took 1123.6 ms for three complete 1,801-point histories.
One hundred cached debug redraws took 797.6 ms for 901 isolated segments per source.
These synthetic measurements are not a production frame-rate guarantee.
The capture review preserves the approved panel order and demonstrates the combined and Unmapped focus views.
No production service or configuration changed during this implementation.
