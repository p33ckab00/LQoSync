# DPDK review and deployment plan

Date: 2026-11-10. Branch: `lqosync3`. Baseline: `a51338f`.

1. Review all six telemetry checkpoints and their callers for correctness, bounds, polling, and rendering cost.
2. Fix concrete defects. Improve chart units, axis labels, sample age, and visible-window peak values within the approved composition.
3. Test reload races, gaps, stale values, zero values, units, chart bounds, and mouse controls.
4. Run the complete test suite, formatting, Clippy, benchmarks, and actual terminal captures.
5. Commit only this task. Preserve unrelated profile notes. Push `lqosync3` and verify CI.
6. Build a clean release artifact. Prepare checksum checks, backup, atomic install, restart, health checks, and rollback.
7. Deploy with host permissions and verify the running executable, cycle state, collection, and all telemetry caches.

The host requires a sudo password. Prepare the complete deployment before requesting the operator's terminal action if non-interactive sudo remains unavailable.
Keep existing polling intervals, source permissions, chart sources, and approved panel order.
Do not add dependencies or infer loss percentages from unrelated counters.

## Review result

The review fixed an aggregate reload race, imprecise rate display, and absent bridge health warnings.
The chart retains the approved composition and adds sample age, window peaks, and matching axis units.
613 tests passed, zero failed, seven ignored. Formatting, Clippy, build, whitespace, and capture review passed.
A 1,801-point fixture took 770.1 ms for 100 debug chart redraws.
Next: commit/push, verify CI, build the clean release, and perform the deployment checks.
