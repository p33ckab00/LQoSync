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
