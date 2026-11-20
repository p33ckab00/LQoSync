# First slice verification

Date: 2026-11-20 (Asia/Shanghai). Source branch: `lqosync3`.
Core baseline before this task: `df5738d`.

## Executed checks

| Check | Result |
|---|---|
| Core regression suite, `cargo test -p lqosync3 --locked` | 1,571 passed, 0 failed, 12 ignored |
| Web suite, `cargo test -p web-gateway --locked` | 9 passed, 0 failed |
| `cargo clippy -p web-gateway --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all --check` | Passed |
| Rust 1.89 gateway check | Passed |
| Rust 1.89 core check | Passed |
| Rust 1.89 Wasm UI check | Passed |
| Native release binaries and Wasm release build | Passed |
| `tools/frontend/build.sh` with Trunk 0.22.2 | Passed |
| Browser checks in Chromium | Passed, including gzip delivery |
| Shell syntax, Python compilation, browser-script syntax | Passed |
| Whitespace, `git diff --check` | Passed |

The tests cover credential changes, multiple accounts, and the final-superadmin guard.
They cover session expiry, token hashes, origin checks, CSRF, logout, and access revocation.
They check that account HTTP routes do not exist.
They check the broker peer UID and rejection when the broker is unavailable.
They cover absent caches, mismatched process IDs, missing PPPoE evidence, and failed routers.

The browser checks sign-in, password visibility, sign-out, and session persistence.
They check explicit theme persistence, live System theme changes, and a 320px viewport without horizontal overflow.
The test verifies HttpOnly cookies. It checks for browser errors.
The final browser run also checks gzip delivery of generated bindings.

## Backend observation

The read-only projection ran against the current `/var/lib/lqosync3` caches.
At that check, it returned 31 subscriber records, 1 router, and 22 PPPoE sessions online.
It returned 292 traffic points and 13 health components.

The health cache reported `UNKNOWN`. The publication phase was `IMPORT_CONFIRMED`.
The projection preserved both states. It did not infer dataplane acceptance from the publication phase.
No collector, service action, configuration write, or TUI change occurred.

## Build size

The optimized Wasm file is 524,772 bytes, approximately 512 KiB.
Its deterministic gzip variant is approximately 181 KiB.
The local variable font is approximately 525 KiB, or 285 KiB with gzip.
The generated bindings are approximately 38 KiB, or 7 KiB with gzip.

These are artifact sizes. They are not production latency or throughput measurements.
The gateway serves gzip variants when the browser accepts them.
It caches static assets for one hour. It does not cache API responses or the themed index.

The generated bootstrap module uses `no-store` so that a deployment can select the current asset hashes.

## Screen evidence

These are screenshots of the implemented Rust application, not generated mockups.
The Dashboard pictures use a test-only account and synthetic data.
The fixture listens on loopback port `19581`. It is not part of the installed services.
The production application uses port `9581`.
The picture hashes are in [SHA256SUMS](screenshots/SHA256SUMS).

| Screen | Light | Dark | Mobile |
|---|---|---|---|
| Sign-in | [Picture](screenshots/lqosync-signin-light.png) | [Picture](screenshots/lqosync-signin-dark.png) | [320px](screenshots/lqosync-signin-mobile.png) |
| Dashboard | [Picture](screenshots/lqosync-dashboard-light.png) | [Picture](screenshots/lqosync-dashboard-dark.png) | [320px](screenshots/lqosync-dashboard-mobile.png) |

The Google modern-web-guidance rules informed form labels, autofill, themes, and responsive behavior.
The Vercel frontend-design skill informed typography, spacing, and the accepted visual direction.
The original design research and skill sources remain in [SOURCES](../SOURCES.md).

## Deployment limits

The production web services are not installed or started by this coding session.
Passwordless sudo is unavailable. No default account was created.

The systemd syntax check reported only the two expected missing installed executable paths.
A complete systemd runtime check requires installation.
The proxy examples are prepared. The current Traefik file was not changed.

Public DNS, cloudflared ingress, TLS, and the complete browser-to-production route remain unverified.
No production load benchmark or certified STE check is claimed.
