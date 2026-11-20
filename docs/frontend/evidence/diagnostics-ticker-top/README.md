# Top status ticker evidence

Date: 2026-11-20 (Asia/Shanghai). Branch: `lqosync3`. Previous commit: `9c2e06b`.
See [design and backend scope](../../DIAGNOSTICS_TICKER.md).

The banner appears before the Dashboard heading.
Five permanent fields show service health, runtime mode, output target, publication phase, and last cycle.
The static details panel remains below the other panels.

Initial component states do not become activity entries.
New status/component changes appear once, with at most eight entries for 120 seconds.
A local timer removes expired entries even if source updates stop.
The permanent fields remain after all activity entries expire.
This browser observation list does not claim a full backend activity history.

## Checks

- Five Rust chart/activity tests passed. They cover baseline behavior, duplicate suppression, source changes, expiry, and the activity bound.

- The browser verified top placement and exactly five permanent fields before any activity.

- Component changes and a raw `IMPORT_CONFIRMED` update appeared. An identical snapshot added no duplicate entry.

- The browser clock test expired activity entries without another source update. All five permanent fields remained.

- Motion, stable nodes, keyboard Pause/Resume, hover pause, and reduced motion passed.

- Repeated component names, long labels, stale evidence, Light/Dark, and 768/320px layouts passed.

- Existing sign-in, theme, session, Dashboard, stale recovery, and sign-out smoke checks passed.

- Wasm release, Clippy with warnings denied, Rust 1.89 compatibility, formatting, and whitespace passed.

Core, gateway, and TUI code did not change. The core test suite was not repeated.
The prepared web assets still need installation.

## Pictures

These are browser screenshots of the implementation with synthetic fixture data.
They do not contain production subscriber or credential data.

- [Dark](dark.png)
- [Light](light.png)
- [Mobile with reduced motion](mobile-static.png)

Run `sha256sum -c SHA256SUMS` in this directory to check the image files.
