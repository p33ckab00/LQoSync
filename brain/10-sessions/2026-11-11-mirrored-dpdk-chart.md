---
type: session
created: 2026-11-11
tags: [tui, dpdk, traffic, performance]
status: deployed-verified
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

The initial implementation checkpoint was local. Production now uses the verified `3.0.169+gfd5430a` release.
The deployment restarted the service. Configuration and router policy remain unchanged.
Preserve the unrelated local-profile Knowledge link and note.
The current code and visual contract are authoritative for this change. The project graph remains an older reference.

## Release preparation

Updated: 2026-11-11T00:18:38+08:00 (Asia/Shanghai).

Code commit `fd5430a` is on GitHub `lqosync3`. CI run `42852687985` passed both jobs.
The clean release is `3.0.169+gfd5430a`. All seven TUI routes passed the release smoke check.
Artifact and deployment evidence directory: `/home/pi/.local/state/lqosync3/mirror-fd5430a/`.
Binary SHA256: `9652ccc93a9f30c0454f17c78166a4fedb2d608362e1a4e55fcf215c922062d8`.

The deployment script is ready with backup, checks, restart, and rollback.
Non-interactive sudo requires a password. The operator must run the prepared script in a terminal.
Deployment remains pending until the script and independent verification pass.

## Deployment verification

Updated: 2026-11-11T00:20:56+08:00 (Asia/Shanghai).

The operator deployed the release at 2026-11-11T00:18:53+08:00.
Independent checks confirmed the installed binary hash, version, service PID, and successful cycle.
Service PID: 544762. Automatic restarts: zero. Configuration hash: unchanged.
All six telemetry sources passed deployment checks. Fresh timestamps and bridge uptime advanced after deployment.
The bridge reports healthy with no degraded subsystems.
The circuit probe returned three matching, advancing samples.
Gap flags: [False, False, False]. Counter reset flags: [False, False, False].
All seven installed TUI routes passed the read-only smoke check. No panic occurred.

Backup: `/var/backups/lqosync3/mirror-fd5430a-5o36lykc`. No rollback was required.
Evidence: `DEPLOYED.json`, `independent-verification.json`, and `installed-tui-smoke.json` in the artifact directory above.
Reopen existing TUI sessions to load the installed chart.

## Dynamic ceiling enhancement

Updated: 2026-11-11T00:32:42+08:00 (Asia/Shanghai).

The owner requested a ceiling that follows the highest traffic while preserving the mirrored view.
The old scale rounded to 1/2/5 steps and imposed a 1 Mbps floor when Actual was visible.
The new scale adds 10% headroom, then rounds up in fine decimal steps. All sources share a 1 kbps minimum.
Both bounds follow the highest visible directional peak in the selected window.
The bounds increase for a new peak and decrease when a peak expires or its source is hidden.
An old visible peak keeps the scale stable when the latest rate falls. Upload can set both bounds.
There is no new poll, retained state, dependency, or history scan.

Regression tests cover higher peaks, expired peaks, hidden sources, upload dominance, small rates, and empty windows.
The full suite passed 617 tests, with zero failed and seven ignored. Formatting, Clippy, build, and capture review passed.
One initial full run hit a Journal channel-disconnect failure. Its isolated rerun and full-suite rerun passed.
The Journal code was not changed.
Actual synthetic terminal capture: `docs/design/tui/captures/dashboard-dynamic-ceiling-v1.png`.

The enhancement remains in the local worktree. Production remains `3.0.169+gfd5430a`.
No push, deployment, restart, or configuration change occurred for this enhancement.

## Independent directional scales

Updated: 2026-11-11T00:37:02+08:00 (Asia/Shanghai).

The owner clarified that each direction must follow its own timeline peak. This replaces the equal-bound enhancement proposal above.
Download and upload now have independent dynamic ceilings. Zero stays centered in the mirrored layout.
All visible download sources share the download ceiling. All visible upload sources share the upload ceiling.
Hiding a source or expiring its peak updates each directional ceiling separately.
The panel title states `independent D/U scales`. Read each directional axis to compare absolute rates across halves.

The renderer normalizes coordinates on one canvas without allocating transformed point arrays.
Histories, current rates, and peak values retain Mbps. Source polling and cache ownership are unchanged.
Regression tests check separate axis labels, expiry, visibility, upload dominance, low-rate traffic, and actual plotted geometry.
The full suite passed 618 tests, with zero failed and seven ignored. Formatting, Clippy, build, whitespace, and screenshot review passed.
One hundred maximum debug redraws took 1,505.9 ms for three 1,801-point histories and 818.7 ms for three 901-segment histories.
These synthetic timings are not terminal frame-rate guarantees.

The corrected screenshot is `docs/design/tui/captures/dashboard-dynamic-ceiling-v1.png`.
The enhancement remains local and uncommitted. Production remains `3.0.169+gfd5430a`.

## Directional scale release preparation

Updated: 2026-11-11T00:40:53+08:00 (Asia/Shanghai).

Owner authorized publication and deployment. Commit `b6778cb` is on GitHub `lqosync3`.
CI run `42856413482` passed both jobs. The clean release is `3.0.172+gb6778cb`.
All seven release TUI routes passed the smoke check.
Artifact: `/home/pi/.local/state/lqosync3/directional-b6778cb/`.
The deployment script includes backup, restart, cycle and telemetry verification, and rollback.
Host sudo requires a password. The operator has the prepared command to execute in a terminal.
Production activation remains pending until deployment and independent verification pass.
