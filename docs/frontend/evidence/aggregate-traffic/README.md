# Aggregate traffic evidence

Date: 2026-11-20 (Asia/Shanghai). Branch: `lqosync3`.
See [the feature contract](../../AGGREGATE_TRAFFIC.md).

## Sources and pictures

The browser fixture uses synthetic traffic, accounts, ports, and worker data.
The screenshots contain no production subscriber records.

- [Dark](dark.png)
- [Light](light.png)
- [Mobile, 320px](mobile.png)

Run `sha256sum -c SHA256SUMS` in this directory to check the images.
The screenshots show one aggregate plot with Actual, Mapped, and Unmapped.
The mobile legend uses three columns without horizontal overflow.

## Checks

The native suite passed before the browser checks.
The final projection checks passed all ten overview tests and eight DPDK tests.
Seven native UI tests cover geometry, gaps, and current history/activity behavior.
The projection checks include independent availability, valid zero, null gaps, future samples, and stale cache failure.
The maximum fixture combines extended DPDK details and all three traffic histories.
It keeps the complete overview within its 60-KiB budget.
A separate check measures the full broker reply against the 64-KiB limit.

The aggregate browser check passed these cases:

- Six curves in one mirrored plot with a shared time domain.
- Separate direction capacity anchors and visible above-capacity traffic.
- One-second updates with stable source controls and pinned timestamps.
- Pointer, keyboard, and touch inspection.
- Missing samples remain distinct from zero.
- Independent unavailable and stale classification states.
- Unknown capacity and forced-color fallback.
- Light/Dark themes and 768/320px layouts without overflow.

The current sign-in/Dashboard, DPDK health, pulse, and ticker checks also passed.
The ticker test holds its fallback snapshot stable during its synthetic clock jump.
This isolates activity expiry from unrelated fixture updates.
The native web release and Rust/Wasm release builds passed.
Native and Wasm Clippy checks passed with warnings denied.
Rust 1.89 checks passed for the native web packages and the Wasm UI.
Formatting and whitespace checks passed.

The optional skill inspector scripts were unavailable in this environment.
Playwright with the installed Chromium supplied browser checks and screenshots.
The screenshots also received a visual review.
The STE checker is heuristic and does not establish certified compliance.

## Runtime

The release web binaries and assets are prepared.
Install them together with `sudo tools/frontend/install.sh` from the repository root.
This session did not replace production files or restart production services.
The core collectors, TUI, CLI accounts, and routing configuration did not change.
Longer traffic windows and subscriber queue screens remain separate work.
