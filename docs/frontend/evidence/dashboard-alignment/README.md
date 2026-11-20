# Dashboard reference alignment evidence

Date: 2026-11-20 (Asia/Shanghai). Branch: `lqosync3`. Previous commit: `06f0536`.
The acceptance criteria are in [DASHBOARD_ALIGNMENT](../../DASHBOARD_ALIGNMENT.md).

## Result

The implementation now follows the approved picture's composition:
five icon cards, the traffic and WAN panels, the upstream table, and the shared sidebar and header.
The dark palette follows the documented tokens. The sign-in design keeps its existing palette.
The sidebar marks future screens unavailable and omits account administration.
The Dashboard card for IPoE uses bound DHCP observations instead of the total subscriber count.

Traffic has blue and green gradient areas, a rate scale, local sample times, a legend, and a text table.
The x positions use timestamps. The chart does not smooth the reported samples.
The gateway orders and bounds samples, excludes invalid timestamps, and checks the latest sample's age.
Longer windows remain disabled with a visible explanation.

The WAN panels show unavailable states because the safe published adapter does not exist yet.
They do not show invented WAN names, shares, capacity, utilization, or routing health.

Service diagnostics and the sample table keep their open state during polling.
A failed monitoring request marks retained data stale and its health unknown.
The next valid response restores the current snapshot.

## Checks

| Check | Result |
|---|---|
| Gateway tests | 11 passed, 0 failed |
| IPoE tests | Bound DHCP only; absent source and stale data stay unknown; observed zero stays zero |
| Traffic tests | 300-point ceiling, ordered times, invalid times excluded, old samples marked stale |
| Native release services | Built |
| Wasm release application | Built |
| Gateway Clippy | Passed with warnings denied |
| Wasm UI Clippy | Passed with warnings denied |
| Rust 1.89 checks | Gateway and Wasm UI passed |
| Formatting and whitespace | Passed |
| Browser reference checks | Five cards, WAN panels, disabled windows, gradient stops, scale and time labels |
| Browser refresh checks | Open diagnostics and sample table persist |
| Browser failure checks | Retained data becomes stale on HTTP 503 and recovers on a valid response |
| Browser authentication checks | Sign-in, theme modes, session persistence, and sign-out passed |
| Mobile width | 320px without page overflow; wide tables scroll in named keyboard regions |

No core service or TUI source files changed. The core test suite was not repeated for this web-only change.
Browser checks use an isolated test account and synthetic data on loopback port `19581`.
They do not create a production account or change production state.

## Pictures

| Dark | Light | Mobile | Expanded details |
|---|---|---|---|
| [Picture](dark.png) | [Picture](light.png) | [320px](mobile.png) | [Picture](expanded.png) |

The pictures show the implemented Rust application with test data.
They are not new generated mockups. Hashes are in [SHA256SUMS](SHA256SUMS).

## Skills

Vercel frontend-design informed the match to the existing composition and the restrained surrounding UI.
Google `css-layout` informed flexible grid tracks and the separation of component and viewport rules.
Its instruction is: "Do NOT use `overflow: hidden` to mask layout issues".
The Dashboard fixes widths and keeps the tables in their own scroll regions.

Google `responsive-table` informed semantic headers, captions, and named regions with `tabindex="0"`.
Its instruction is: "Use standard `<table>` elements".
The table keeps its headers on small screens and supports keyboard scrolling.
The guide's optional stacked-cell transformation is not necessary for these unavailable-state tables.
See [source records](../../SOURCES.md) for the skill repositories.

## Remaining integration

F06 remains partial until it has a safe WAN overview and any wider traffic history that the product requires.
The adapter must publish stable WAN IDs, source timestamps, capacity, membership, and attributed traffic.
It must distinguish shaping placement from packet routing.
Do not read private configuration directly or infer a successful dataplane apply from an imported file.

The web binaries and browser assets are built locally.
Run `sudo tools/frontend/install.sh` from the repository to install this update.
This session did not replace the installed services or assets.
