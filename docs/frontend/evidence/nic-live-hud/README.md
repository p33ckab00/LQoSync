# Live NIC HUD evidence

Date: 2026-11-21 (Asia/Shanghai). Branch: `lqosync3`.
See [the feature contract](../../NIC_LIVE_DIAGNOSTICS.md).

## Pictures

The pictures use synthetic fixture data, without production accounts or subscriber records.

- [Dark](dark.png)
- [Light](light.png)
- [Mobile, 320px](mobile.png)

Run `sha256sum -c SHA256SUMS` in this directory.

## Checks

The broker regression first failed because a one-second refresh kept the previous NIC timestamp.
It then passed after each refresh read the current NIC cache.
Native checks cover measured rates, unknown composition, invalid totals, duplicate samples, and chart gaps.
Core checks cover one-second intervals, the ten-second failure retry, and omission of optional module probes during fast samples.
A separate check verifies counter timestamps before optional hardware delays.

The browser check covers hover, pointer entry, keyboard, tap, live sample changes, rate conversion, raw counter persistence, and bounded trends.
It also checks reset/stale/unknown states, Light/Dark themes, forced colors, and the 320px viewport.
The session note records final build and regression results.

Vercel frontend-design and Google modern-web-guidance supplied interface guidance.
Playwright and Chromium supply the browser assertions and screenshots.
The STE checker is heuristic and does not establish certified compliance.

Production binaries and services do not change during these checks.
Install both prepared components with the [NIC update procedure](../../DEPLOYMENT.md#nic-faceplate-update).
