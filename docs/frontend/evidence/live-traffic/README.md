# Live traffic evidence

Date: 2026-11-20 (Asia/Shanghai). Branch: `lqosync3`. Previous commit: `61b1dc9`.
See the [design and atomic plan](../../LIVE_TRAFFIC.md).

## Result

The core samples native traffic once per second.
All three histories keep the existing one-hour time limit, with a 3601-point ceiling.
The public worker cache has a 2 MiB bound. Atomic publication and the tmpfs path remain in use.
No TUI screen or control changed.

The service publishes the true configured WAN base totals through `status.json`.
The broker projects these safe totals without access to private configuration or registry files.
Download and upload use separate sums. Shaping factors do not reduce the true capacity totals.
Catalog validation, checked addition, current status, and valid positive rates are required.

The gateway streams authenticated snapshots once per second through server-sent events.
The broker refreshes traffic each second and the full overview every five seconds.
The traffic decoder skips worker rows and the other histories without allocation of their JSON trees.
At most 16 streams can run. Slow readers have no event queue.
Session expiry, account revocation, and broker failure stop the stream.

The browser closes the connection when the page becomes hidden or the Dashboard closes.
It opens a new connection on return. A five-second overview poll remains as a fallback.
Unchanged snapshots do not cause a new chart render.

The chart uses a vertical rainbow in SVG user coordinates.
Each direction reaches red at its own configured capacity.
The upload line uses dashes. The download line is solid.
Visible text gives both capacities and utilization values.

The rate axis expands for traffic above capacity. A red boundary still marks the configured limit.

Without current capacity, colors do not claim utilization.
Unknown samples and long time gaps split both line and area geometry.

## Checks


- Core, gateway, and native chart tests: 1590 passed, 0 failed, 12 existing ignored tests.

- Capacity tests: multiple WANs, true base rates despite lower factors, absent catalog, overflow, old status, and invalid rates.

- History tests: one-second samples keep the full hour. Maximum worker rows and histories fit the bounded cache.

- Stream tests: authentication, successive events, revocation, and broker failure.

- Browser smoke: sign-in, themes, session persistence, open-panel preservation, stale failure, recovery, and sign-out.

- Browser pulse: three updates within five seconds, independent direction thresholds, traffic above capacity, and absent capacity.

- Browser lifecycle: hidden-page close, return reconnect, and sign-out cleanup.

- Browser layout: 1440px, 768px, and 320px. No page overflow. Wide tables have scroll regions.

- Browser accessibility: semantic sample table, line patterns, text values, and forced-color fallback.

- Native core, web services, and test fixture release builds: passed.

- Wasm release build, gateway Clippy, and Wasm Clippy: passed. Clippy denies warnings.

- Rust 1.89 compatibility: gateway and Wasm UI checks passed.

- Formatting, whitespace, document links, and screenshot hashes: passed.

The full-hour fixture produced a 19,511-byte browser snapshot with 300 points.
One debug projection took approximately 27 ms on this host. This is fixture evidence, not a production latency guarantee.

## Pictures

These are browser screenshots of the implemented application.
The isolated `browser_fixture` supplies synthetic accounts, two-WAN capacity, and traffic.
They contain no production subscriber or credential data.
The screenshot scale is 200 Mbps down and 80 Mbps up.
Values below these capacities correctly stop short of red.

- [Dark](dark.png)
- [Light](light.png)
- [Mobile](mobile.png)

Check file integrity with `sha256sum -c SHA256SUMS` from this directory.

## Skills and limits

Vercel frontend-design preserves the approved Dashboard composition and limits the visual change to the traffic chart.
Google modern-web-guidance `visual-effects` requires text with gradient charts.
Each theme uses explicit SVG stop colors. Forced colors remove the fills and keep solid/dashed foreground lines.
The optional design-taste inspector is unavailable. Playwright provides browser and screenshot checks.

Per-WAN traffic attribution and link health remain unavailable.
Configured capacity does not prove routing, live link availability, or applied shaping.
The prepared build is not installed by these checks.
The operator must install the updated core and web services. See [deployment](../../DEPLOYMENT.md).
