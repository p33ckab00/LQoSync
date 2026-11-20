# NIC summary placement

Date: 2026-11-21 (Asia/Shanghai). Branch: `lqosync3`.

The NIC faceplate now belongs to the service summary, beside PPPoE and LibreQoS.
It precedes Traffic (Aggregate). The DPDK card keeps worker charts, pressure, and raw counters.
Both cards share one snapshot and one server-time clock.
The cadence description remains available to screen readers. Port tooltips retain the interval and activity totals.

The layout retains the existing colors, typography, connector artwork, and independent activity LEDs.
Vercel frontend-design and Google modern-web-guidance supplied the layout guidance.
CSS Grid provides the summary layout. Flexbox keeps the physical ports adjacent.

## Pictures

These pictures use synthetic data and Reduced Motion mode.

- [Wide summary, 1920px](wide.png)
- [Dark summary](dark.png)
- [Light summary](light.png)
- [Mobile summary, 320px](mobile.png)

Run `sha256sum -c SHA256SUMS` in this directory to check the pictures.

## Checks

The NIC browser check verifies one faceplate in the summary and none in the DPDK card.
It checks placement before traffic, port adjacency, and page width at 320, 768, 1024, 1280, 1600, and 1920px.
At 1600px and above, all six cards share one row.
It also checks identity, independent sample pulses, stale/down/reset suppression, keyboard access, theme modes, and Reduced Motion.
The DPDK check verifies shared freshness with a one-hour client clock difference.
See the session note for the build and regression results.

## Install

The placement update changes only web artifacts.
After the successful build, run `sudo tools/frontend/install.sh` from the repository. Refresh the browser.
Full hardware detection still needs the [core update](../../DEPLOYMENT.md#nic-faceplate-update) if it was not installed.
Production services did not change during these checks.
