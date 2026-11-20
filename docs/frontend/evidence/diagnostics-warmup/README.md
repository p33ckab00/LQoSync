# Ticker startup regression

Date: 2026-11-20 (Asia/Shanghai). Branch: `lqosync3`. Previous commit: `b83eb63`.

## Cause and correction

The Dashboard mounts before the first monitoring response arrives.
The moving queue copied ten `Unknown` placeholders during that interval.
Later responses updated the static list but did not refresh those moving entries until their next pass.

A browser regression reproduced all ten placeholders while the static list already contained valid values.
The corrected queue waits for a timestamped snapshot.
It shows “Waiting for service status…” before that response arrives.
Both sets of moving status items then use the first source values.

## Checks

- The test delays both overview requests and the live stream.
- During the delay, the waiting message appears and the moving queue stays empty.
- After release, the moving values match all five source values in both status sets.
- Insertion continuity, repeated handoffs, viewport coverage, activity expiry, and stale status checks passed.
- Light/Dark, reduced motion, and 768/320px checks passed.
- Release Wasm, Clippy with warnings denied, and Rust 1.89 checks passed.
- Existing sign-in/Dashboard smoke, formatting, and whitespace checks passed.

Production service checks found running core, broker, and gateway services.
The status cache contained runtime mode, output target, and cycle duration.
The health cache separately reported degraded health.
This correction preserves genuine unknown or degraded source values.

The core, backend, and TUI code did not change.
The prepared web assets need installation with `sudo tools/frontend/install.sh`.
Production files and services were not changed.


## Pictures

These browser screenshots use synthetic fixture data.

- [Dark](dark.png)
- [Light](light.png)
- [Mobile with reduced motion](mobile-static.png)

Run `sha256sum -c SHA256SUMS` in this directory to check the images.
The delayed-response browser test verifies the waiting state.
